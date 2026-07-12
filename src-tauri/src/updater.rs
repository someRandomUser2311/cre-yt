use std::process::Stdio;
use tauri::AppHandle;
use tokio::time::{timeout, Duration};

use crate::engine;

#[cfg(target_os = "linux")]
const ASSET_NAME: &str = "yt-dlp_linux";
#[cfg(target_os = "windows")]
const ASSET_NAME: &str = "yt-dlp.exe";
#[cfg(target_os = "macos")]
const ASSET_NAME: &str = "yt-dlp_macos";

async fn run_version(path: &std::path::Path) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new(path);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let out = timeout(Duration::from_secs(15), cmd.arg("--version").stdin(Stdio::null()).output())
        .await
        .map_err(|_| "yt-dlp --version timed out".to_string())?
        .map_err(|e| format!("failed to run yt-dlp: {e}"))?;
    if !out.status.success() {
        return Err("yt-dlp --version failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[tauri::command]
pub async fn get_ytdlp_version(app: AppHandle) -> Result<String, String> {
    run_version(&engine::ytdlp_path(&app)?).await
}

/// Downloads the latest yt-dlp release into the app data dir, where it
/// shadows the read-only bundled sidecar (see engine::ytdlp_path).
#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<String, String> {
    let url = format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{ASSET_NAME}");
    let client = reqwest::Client::builder()
        .user_agent("youtube-downloader-app")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download failed: {e}"))?;
    let bytes = resp.bytes().await.map_err(|e| format!("download failed: {e}"))?;

    let dest = engine::updated_ytdlp_path(&app)?;
    let dir = dest.parent().ok_or("bad destination path")?;
    tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;

    let staging = dest.with_extension("new");
    tokio::fs::write(&staging, &bytes).await.map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))
            .await
            .map_err(|e| e.to_string())?;
    }

    // verify the new binary actually runs before swapping it in
    let version = run_version(&staging).await.map_err(|e| {
        format!("downloaded binary failed verification: {e}")
    })?;
    tokio::fs::rename(&staging, &dest).await.map_err(|e| e.to_string())?;
    Ok(version)
}
