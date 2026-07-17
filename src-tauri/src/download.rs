use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

use crate::engine;
use crate::info::extract_error;
use crate::persist;

pub const EVT_PROGRESS: &str = "download://progress";
pub const EVT_STATUS: &str = "download://status";
pub const EVT_COMPLETE: &str = "download://complete";
pub const EVT_ERROR: &str = "download://error";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Downloading,
    Processing,
    Paused,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub dest_dir: String,
    #[serde(default)]
    pub format_selector: Option<String>,
    #[serde(default)]
    pub audio_only: bool,
    #[serde(default)]
    pub audio_format: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
}

struct Job {
    request: DownloadRequest,
    status: JobStatus,
    child: Option<Arc<Mutex<Child>>>,
    /// last tmpfilename/filename seen in progress lines — used for cancel cleanup
    tmp_files: Vec<String>,
    filepath: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub dest_dir: String,
    pub audio_only: bool,
    pub status: JobStatus,
    pub filepath: Option<String>,
    pub error: Option<String>,
}

pub struct DownloadManager {
    jobs: Mutex<HashMap<String, Job>>,
    semaphore: std::sync::Mutex<Arc<Semaphore>>,
}

impl DownloadManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            semaphore: std::sync::Mutex::new(Arc::new(Semaphore::new(max_concurrent.max(1)))),
        }
    }

    fn current_semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.lock().unwrap().clone()
    }

    fn set_max_concurrent(&self, n: usize) {
        *self.semaphore.lock().unwrap() = Arc::new(Semaphore::new(n.max(1)));
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusPayload {
    id: String,
    status: JobStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    id: String,
    stage: String, // "downloading" | "processing"
    downloaded: Option<u64>,
    total: Option<u64>,
    percent: Option<f64>,
    speed: Option<f64>,
    eta: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompletePayload {
    id: String,
    filepath: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    id: String,
    message: String,
}

fn emit_status(app: &AppHandle, id: &str, status: JobStatus) {
    let _ = app.emit(EVT_STATUS, StatusPayload { id: id.into(), status });
}

/// yt-dlp accepts arbitrary URLs; we only ever hand it http(s) links. Reject
/// anything else so a caller can't coax it into `file:`, `ytdl:` or other
/// extractor schemes that read the local machine.
fn validate_url(url: &str) -> Result<(), String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err("URL must start with http:// or https://".into())
    }
}

/// The format selector is passed verbatim to yt-dlp's `-f`. Our UI only ever
/// produces selectors built from format ids and preset expressions, so we
/// constrain the input to that grammar as defense-in-depth against an
/// untrusted caller injecting a hostile selector expression.
fn validate_format_selector(sel: &str) -> Result<(), String> {
    const MAX_LEN: usize = 200;
    if sel.is_empty() || sel.len() > MAX_LEN {
        return Err("invalid format selector".into());
    }
    // allow: format ids, +, /, comparison filters like [height<=1080]
    let ok = sel
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '[' | ']' | '<' | '>' | '=' | '.' | '_' | '-' | ':' | ' '));
    if ok {
        Ok(())
    } else {
        Err("format selector contains invalid characters".into())
    }
}

fn build_args(req: &DownloadRequest, ffmpeg_dir: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    if req.audio_only {
        let fmt = req.audio_format.as_deref().unwrap_or("mp3");
        if fmt == "m4a" {
            args.extend(["-f".into(), "bestaudio[ext=m4a]/bestaudio/best".into()]);
        } else {
            args.extend(["-f".into(), "bestaudio/best".into()]);
        }
        args.extend([
            "-x".into(),
            "--audio-format".into(),
            fmt.into(),
            "--audio-quality".into(),
            "0".into(),
        ]);
    } else {
        let selector = req
            .format_selector
            .clone()
            .unwrap_or_else(|| "bestvideo+bestaudio/best".into());
        args.extend(["-f".into(), selector]);
    }

    args.extend([
        "--ffmpeg-location".into(),
        ffmpeg_dir.into(),
        "-P".into(),
        req.dest_dir.clone(),
        "-o".into(),
        "%(title)s [%(id)s].%(ext)s".into(),
        "--no-playlist".into(),
        "--newline".into(),
        "--no-warnings".into(),
        "--continue".into(),
        "--windows-filenames".into(),
        "--trim-filenames".into(),
        "200".into(),
        "--progress-template".into(),
        "download:PROG::%(progress)j".into(),
        "--progress-template".into(),
        "postprocess:PP::%(progress)j".into(),
        "--print".into(),
        "after_move:FILE::%(filepath)s".into(),
        "--no-simulate".into(),
        "--".into(),
        req.url.clone(),
    ]);
    args
}

async fn run_job(app: AppHandle, manager: Arc<DownloadManager>, id: String) {
    let permit = match manager.current_semaphore().acquire_owned().await {
        Ok(p) => p,
        Err(_) => return,
    };

    // The job may have been cancelled while queued, or this permit may be from
    // a swapped-out semaphore — re-check state before spawning.
    let request = {
        let jobs = manager.jobs.lock().await;
        match jobs.get(&id) {
            Some(j) if j.status == JobStatus::Queued => j.request.clone(),
            _ => return,
        }
    };

    let spawn_result: Result<Child, String> = (|| {
        let ffmpeg_dir = engine::ffmpeg_location()?.to_string_lossy().into_owned();
        let mut cmd = engine::ytdlp_command(&app)?;
        cmd.args(build_args(&request, &ffmpeg_dir))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.spawn().map_err(|e| format!("failed to start yt-dlp: {e}"))
    })();

    let mut child = match spawn_result {
        Ok(c) => c,
        Err(e) => {
            fail_job(&app, &manager, &id, e).await;
            drop(permit);
            return;
        }
    };

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let child = Arc::new(Mutex::new(child));

    {
        let mut jobs = manager.jobs.lock().await;
        if let Some(job) = jobs.get_mut(&id) {
            job.child = Some(child.clone());
            job.status = JobStatus::Downloading;
        }
    }
    emit_status(&app, &id, JobStatus::Downloading);

    // stderr collector (bounded)
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        let mut buf: Vec<String> = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            if buf.len() >= 50 {
                buf.remove(0);
            }
            buf.push(line);
        }
        buf.join("\n")
    });

    // stdout: progress + final file path
    let mut lines = BufReader::new(stdout).lines();
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    let mut final_path: Option<String> = None;

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(json) = line.strip_prefix("PROG::") {
            if let Ok(p) = serde_json::from_str::<Value>(json) {
                track_tmp_files(&manager, &id, &p).await;
                if last_emit.elapsed() >= Duration::from_millis(250) {
                    last_emit = Instant::now();
                    emit_progress(&app, &id, &p, "downloading");
                }
            }
        } else if let Some(json) = line.strip_prefix("PP::") {
            let mark = {
                let mut jobs = manager.jobs.lock().await;
                jobs.get_mut(&id)
                    .filter(|j| j.status == JobStatus::Downloading)
                    .map(|j| {
                        j.status = JobStatus::Processing;
                    })
                    .is_some()
            };
            if mark {
                emit_status(&app, &id, JobStatus::Processing);
            }
            if let Ok(p) = serde_json::from_str::<Value>(json) {
                emit_progress(&app, &id, &p, "processing");
            }
        } else if let Some(path) = line.strip_prefix("FILE::") {
            final_path = Some(path.to_string());
        }
    }

    let exit = child.lock().await.wait().await;
    let stderr_text = stderr_task.await.unwrap_or_default();

    let mut jobs = manager.jobs.lock().await;
    let Some(job) = jobs.get_mut(&id) else { return };
    job.child = None;

    match job.status {
        // pause_download / cancel_download killed the process and already set
        // the terminal state — leave it alone.
        JobStatus::Paused | JobStatus::Cancelled => {}
        _ => match exit {
            Ok(status) if status.success() => {
                job.status = JobStatus::Completed;
                job.filepath = final_path.clone();
                let entry = persist::HistoryEntry {
                    id: id.clone(),
                    url: job.request.url.clone(),
                    title: job.request.title.clone().unwrap_or_else(|| job.request.url.clone()),
                    filepath: final_path.clone(),
                    audio_only: job.request.audio_only,
                    completed_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                };
                drop(jobs);
                persist::append_history(&app, entry);
                emit_status(&app, &id, JobStatus::Completed);
                let _ = app.emit(EVT_COMPLETE, CompletePayload { id: id.clone(), filepath: final_path });
            }
            _ => {
                let msg = extract_error(&stderr_text);
                job.status = JobStatus::Error;
                job.error = Some(msg.clone());
                drop(jobs);
                emit_status(&app, &id, JobStatus::Error);
                let _ = app.emit(EVT_ERROR, ErrorPayload { id: id.clone(), message: msg });
            }
        },
    }

    drop(permit);
}

async fn track_tmp_files(manager: &DownloadManager, id: &str, progress: &Value) {
    let mut names: Vec<String> = Vec::new();
    for key in ["tmpfilename", "filename"] {
        if let Some(name) = progress.get(key).and_then(Value::as_str) {
            names.push(name.to_string());
        }
    }
    if names.is_empty() {
        return;
    }
    let mut jobs = manager.jobs.lock().await;
    if let Some(job) = jobs.get_mut(id) {
        for n in names {
            if !job.tmp_files.contains(&n) {
                job.tmp_files.push(n);
            }
        }
    }
}

fn emit_progress(app: &AppHandle, id: &str, p: &Value, stage: &str) {
    let downloaded = p.get("downloaded_bytes").and_then(Value::as_u64);
    let total = p
        .get("total_bytes")
        .and_then(Value::as_u64)
        .or_else(|| p.get("total_bytes_estimate").and_then(Value::as_f64).map(|f| f as u64));
    let percent = match (downloaded, total) {
        (Some(d), Some(t)) if t > 0 => Some((d as f64 / t as f64) * 100.0),
        _ => None,
    };
    let _ = app.emit(
        EVT_PROGRESS,
        ProgressPayload {
            id: id.into(),
            stage: stage.into(),
            downloaded,
            total,
            percent,
            speed: p.get("speed").and_then(Value::as_f64),
            eta: p.get("eta").and_then(Value::as_f64),
        },
    );
}

async fn fail_job(app: &AppHandle, manager: &DownloadManager, id: &str, msg: String) {
    {
        let mut jobs = manager.jobs.lock().await;
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Error;
            job.error = Some(msg.clone());
        }
    }
    emit_status(app, id, JobStatus::Error);
    let _ = app.emit(EVT_ERROR, ErrorPayload { id: id.into(), message: msg });
}

struct KilledJob {
    tmp_files: Vec<String>,
    dest_dir: String,
}

async fn kill_job_child(manager: &DownloadManager, id: &str, new_status: JobStatus) -> Result<KilledJob, String> {
    let (child, tmp_files, dest_dir) = {
        let mut jobs = manager.jobs.lock().await;
        let job = jobs.get_mut(id).ok_or_else(|| "unknown download id".to_string())?;
        match job.status {
            JobStatus::Queued | JobStatus::Downloading | JobStatus::Processing => {}
            _ => return Err("download is not active".into()),
        }
        job.status = new_status;
        (job.child.take(), job.tmp_files.clone(), job.request.dest_dir.clone())
    };
    if let Some(child) = child {
        let mut child = child.lock().await;
        let _ = child.kill().await;
    }
    Ok(KilledJob { tmp_files, dest_dir })
}

/// True when `candidate` sits directly inside `base`. The candidate file may
/// already be gone, so we resolve its parent directory (which still exists)
/// and confirm that resolves under the canonicalized base. Guards against a
/// yt-dlp-reported path escaping the destination via `..` or symlinks.
fn is_within(base: Option<&std::path::Path>, candidate: &str) -> bool {
    let Some(base) = base else { return false };
    let path = std::path::Path::new(candidate);
    let Some(parent) = path.parent() else { return false };
    match std::fs::canonicalize(parent) {
        Ok(real_parent) => real_parent == base,
        Err(_) => false,
    }
}

fn snapshot(id: &str, job: &Job) -> JobSnapshot {
    JobSnapshot {
        id: id.to_string(),
        url: job.request.url.clone(),
        title: job.request.title.clone(),
        thumbnail: job.request.thumbnail.clone(),
        dest_dir: job.request.dest_dir.clone(),
        audio_only: job.request.audio_only,
        status: job.status,
        filepath: job.filepath.clone(),
        error: job.error.clone(),
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    manager: State<'_, Arc<DownloadManager>>,
    request: DownloadRequest,
) -> Result<String, String> {
    validate_url(&request.url)?;
    if let Some(sel) = request.format_selector.as_deref() {
        validate_format_selector(sel)?;
    }
    if !std::path::Path::new(&request.dest_dir).is_dir() {
        return Err(format!("destination folder does not exist: {}", request.dest_dir));
    }
    let id = Uuid::new_v4().to_string();
    {
        let mut jobs = manager.jobs.lock().await;
        jobs.insert(
            id.clone(),
            Job {
                request,
                status: JobStatus::Queued,
                child: None,
                tmp_files: Vec::new(),
                filepath: None,
                error: None,
            },
        );
    }
    emit_status(&app, &id, JobStatus::Queued);
    let manager = manager.inner().clone();
    let task_id = id.clone();
    tauri::async_runtime::spawn(run_job(app, manager, task_id));
    Ok(id)
}

#[tauri::command]
pub async fn pause_download(manager: State<'_, Arc<DownloadManager>>, app: AppHandle, id: String) -> Result<(), String> {
    // yt-dlp keeps .part files; resume re-runs with --continue
    kill_job_child(&manager, &id, JobStatus::Paused).await?;
    emit_status(&app, &id, JobStatus::Paused);
    Ok(())
}

#[tauri::command]
pub async fn resume_download(manager: State<'_, Arc<DownloadManager>>, app: AppHandle, id: String) -> Result<(), String> {
    {
        let mut jobs = manager.jobs.lock().await;
        let job = jobs.get_mut(&id).ok_or_else(|| "unknown download id".to_string())?;
        if job.status != JobStatus::Paused && job.status != JobStatus::Error {
            return Err("download is not paused".into());
        }
        job.status = JobStatus::Queued;
        job.error = None;
    }
    emit_status(&app, &id, JobStatus::Queued);
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn(run_job(app, manager, id));
    Ok(())
}

#[tauri::command]
pub async fn cancel_download(manager: State<'_, Arc<DownloadManager>>, app: AppHandle, id: String) -> Result<(), String> {
    let KilledJob { tmp_files, dest_dir } = kill_job_child(&manager, &id, JobStatus::Cancelled).await?;
    emit_status(&app, &id, JobStatus::Cancelled);
    // best-effort cleanup of partial files after the process has died.
    // tmp/filenames originate from yt-dlp output, so constrain deletion to the
    // job's own destination directory — never follow a path outside it.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let base = tokio::fs::canonicalize(&dest_dir).await.ok();
        for f in tmp_files {
            for candidate in [f.clone(), format!("{f}.part"), format!("{f}.ytdl")] {
                if !is_within(base.as_deref(), &candidate) {
                    continue;
                }
                let _ = tokio::fs::remove_file(&candidate).await;
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn remove_job(manager: State<'_, Arc<DownloadManager>>, id: String) -> Result<(), String> {
    let mut jobs = manager.jobs.lock().await;
    match jobs.get(&id).map(|j| j.status) {
        Some(JobStatus::Completed | JobStatus::Cancelled | JobStatus::Error | JobStatus::Paused) => {
            jobs.remove(&id);
            Ok(())
        }
        Some(_) => Err("cancel the download before removing it".into()),
        None => Ok(()),
    }
}

#[tauri::command]
pub async fn get_queue(manager: State<'_, Arc<DownloadManager>>) -> Result<Vec<JobSnapshot>, String> {
    let jobs = manager.jobs.lock().await;
    Ok(jobs.iter().map(|(id, job)| snapshot(id, job)).collect())
}

#[tauri::command]
pub async fn set_max_concurrent(manager: State<'_, Arc<DownloadManager>>, max: usize) -> Result<(), String> {
    manager.set_max_concurrent(max);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> DownloadRequest {
        DownloadRequest {
            url: "https://example.com/watch?v=abc".into(),
            dest_dir: "/tmp/out".into(),
            format_selector: None,
            audio_only: false,
            audio_format: None,
            title: None,
            thumbnail: None,
        }
    }

    /// Return the value following the first occurrence of `flag`.
    fn arg_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter().position(|a| a == flag).map(|i| args[i + 1].as_str())
    }

    #[test]
    fn build_args_default_video_selector() {
        let args = build_args(&req(), "/ff");
        assert_eq!(arg_after(&args, "-f"), Some("bestvideo+bestaudio/best"));
        // url is passed after the `--` terminator, never as a flag
        assert_eq!(args.last().unwrap(), "https://example.com/watch?v=abc");
        let dd = args.iter().position(|a| a == "--").unwrap();
        assert!(args[..dd].iter().all(|a| a != "https://example.com/watch?v=abc"));
    }

    #[test]
    fn build_args_uses_explicit_selector() {
        let mut r = req();
        r.format_selector = Some("137+140".into());
        let args = build_args(&r, "/ff");
        assert_eq!(arg_after(&args, "-f"), Some("137+140"));
    }

    #[test]
    fn build_args_audio_mp3_extracts() {
        let mut r = req();
        r.audio_only = true;
        r.audio_format = Some("mp3".into());
        let args = build_args(&r, "/ff");
        assert_eq!(arg_after(&args, "-f"), Some("bestaudio/best"));
        assert!(args.iter().any(|a| a == "-x"));
        assert_eq!(arg_after(&args, "--audio-format"), Some("mp3"));
    }

    #[test]
    fn build_args_audio_m4a_prefers_m4a_stream() {
        let mut r = req();
        r.audio_only = true;
        r.audio_format = Some("m4a".into());
        let args = build_args(&r, "/ff");
        assert_eq!(arg_after(&args, "-f"), Some("bestaudio[ext=m4a]/bestaudio/best"));
        assert_eq!(arg_after(&args, "--audio-format"), Some("m4a"));
    }

    #[test]
    fn build_args_passes_ffmpeg_and_dest() {
        let args = build_args(&req(), "/opt/ff");
        assert_eq!(arg_after(&args, "--ffmpeg-location"), Some("/opt/ff"));
        assert_eq!(arg_after(&args, "-P"), Some("/tmp/out"));
        assert!(args.iter().any(|a| a == "--no-playlist"));
    }

    #[test]
    fn validate_url_accepts_http_and_https() {
        assert!(validate_url("http://x").is_ok());
        assert!(validate_url("https://x").is_ok());
    }

    #[test]
    fn validate_url_rejects_other_schemes() {
        for u in ["file:///etc/passwd", "ftp://x", "javascript:alert(1)", "  https://x", ""] {
            assert!(validate_url(u).is_err(), "should reject {u:?}");
        }
    }

    #[test]
    fn validate_format_selector_allows_ui_grammar() {
        for s in [
            "137+140",
            "bestvideo+bestaudio/best",
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
            "bestaudio[ext=m4a]/bestaudio/best",
        ] {
            assert!(validate_format_selector(s).is_ok(), "should allow {s:?}");
        }
    }

    #[test]
    fn validate_format_selector_rejects_injection_and_bounds() {
        assert!(validate_format_selector("").is_err());
        assert!(validate_format_selector(&"1".repeat(201)).is_err());
        for s in ["137;rm -rf /", "$(whoami)", "best`id`", "a|b", "a\nb"] {
            assert!(validate_format_selector(s).is_err(), "should reject {s:?}");
        }
    }

    #[test]
    fn is_within_confines_to_base() {
        let base_raw = std::env::temp_dir().join(format!("ythx-{}", std::process::id()));
        std::fs::create_dir_all(&base_raw).unwrap();
        let base = std::fs::canonicalize(&base_raw).unwrap();

        let inside = base.join("clip.mp4.part");
        std::fs::write(&inside, b"x").unwrap();
        assert!(is_within(Some(&base), inside.to_str().unwrap()));

        // a sibling directory's file must not be considered inside
        let outside = std::env::temp_dir().join("etc-shadow-decoy");
        assert!(!is_within(Some(&base), outside.to_str().unwrap()));

        // parent escape via ..
        let escape = base.join("../elsewhere/f");
        assert!(!is_within(Some(&base), escape.to_str().unwrap()));

        // no base => never within
        assert!(!is_within(None, inside.to_str().unwrap()));

        std::fs::remove_dir_all(&base_raw).ok();
    }
}
