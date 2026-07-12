use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use tauri::AppHandle;
use tokio::time::{timeout, Duration};

use crate::engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub tbr: Option<f64>,
    pub format_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub is_playlist: bool,
    pub title: String,
    pub webpage_url: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub formats: Vec<Format>,
    pub entries: Vec<PlaylistEntry>,
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    match v.get(key) {
        Some(Value::String(s)) if !s.is_empty() && s != "none" => Some(s.clone()),
        _ => None,
    }
}

fn opt_f64(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

fn opt_u64(v: &Value, key: &str) -> Option<u64> {
    v.get(key).and_then(Value::as_u64)
}

fn parse_format(f: &Value) -> Option<Format> {
    let format_id = opt_str(f, "format_id")?;
    let ext = opt_str(f, "ext").unwrap_or_default();
    // storyboards and other non-media pseudo-formats
    if ext == "mhtml" {
        return None;
    }
    let vcodec = opt_str(f, "vcodec");
    let acodec = opt_str(f, "acodec");
    if vcodec.is_none() && acodec.is_none() {
        return None;
    }
    Some(Format {
        format_id,
        ext,
        resolution: opt_str(f, "resolution"),
        height: opt_u64(f, "height").map(|h| h as u32),
        fps: opt_f64(f, "fps"),
        vcodec,
        acodec,
        filesize: opt_u64(f, "filesize"),
        filesize_approx: opt_u64(f, "filesize_approx"),
        tbr: opt_f64(f, "tbr"),
        format_note: opt_str(f, "format_note"),
    })
}

pub fn parse_media_info(root: &Value) -> MediaInfo {
    let is_playlist = root.get("_type").and_then(Value::as_str) == Some("playlist");
    let title = opt_str(root, "title").unwrap_or_else(|| "Untitled".into());
    let webpage_url = opt_str(root, "webpage_url")
        .or_else(|| opt_str(root, "original_url"))
        .unwrap_or_default();
    let thumbnail = opt_str(root, "thumbnail").or_else(|| {
        root.get("thumbnails")
            .and_then(Value::as_array)
            .and_then(|t| t.last())
            .and_then(|t| opt_str(t, "url"))
    });

    let formats = root
        .get("formats")
        .and_then(Value::as_array)
        .map(|fs| fs.iter().filter_map(parse_format).collect())
        .unwrap_or_default();

    let entries = root
        .get("entries")
        .and_then(Value::as_array)
        .map(|es| {
            es.iter()
                .map(|e| PlaylistEntry {
                    id: opt_str(e, "id"),
                    title: opt_str(e, "title"),
                    url: opt_str(e, "url").or_else(|| {
                        // some extractors omit url in flat entries; YouTube ids are reconstructable
                        opt_str(e, "id").map(|id| format!("https://www.youtube.com/watch?v={id}"))
                    }),
                    duration: opt_f64(e, "duration"),
                })
                .collect()
        })
        .unwrap_or_default();

    MediaInfo {
        is_playlist,
        title,
        webpage_url,
        thumbnail,
        duration: opt_f64(root, "duration"),
        uploader: opt_str(root, "uploader").or_else(|| opt_str(root, "channel")),
        formats,
        entries,
    }
}

/// Extract the most useful error line from yt-dlp stderr.
pub fn extract_error(stderr: &str) -> String {
    stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("ERROR:"))
        .map(|l| l.trim_start_matches("ERROR:").trim().to_string())
        .unwrap_or_else(|| {
            let tail: Vec<&str> = stderr.lines().rev().take(3).collect();
            tail.into_iter().rev().collect::<Vec<_>>().join(" ")
        })
}

#[tauri::command]
pub async fn fetch_video_info(app: AppHandle, url: String) -> Result<MediaInfo, String> {
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Please enter a valid http(s) URL".into());
    }

    let mut cmd = engine::ytdlp_command(&app)?;
    cmd.args(["-J", "--flat-playlist", "--no-warnings", "--", &url])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = timeout(Duration::from_secs(60), cmd.output())
        .await
        .map_err(|_| "Timed out fetching video info (60s)".to_string())?
        .map_err(|e| format!("failed to run yt-dlp: {e}"))?;

    if !output.status.success() {
        return Err(extract_error(&String::from_utf8_lossy(&output.stderr)));
    }

    let root: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("could not parse yt-dlp output: {e}"))?;
    Ok(parse_media_info(&root))
}
