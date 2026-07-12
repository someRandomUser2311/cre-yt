use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tokio::process::Command;

pub const YTDLP: &str = "yt-dlp";

/// Directory containing the app executable — Tauri installs sidecars here,
/// and `tauri dev` copies them into target/debug.
pub fn exe_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| format!("cannot locate app executable: {e}"))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "app executable has no parent directory".into())
}

fn with_exe_suffix(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// Path where the self-updater installs newer yt-dlp builds.
pub fn updated_ytdlp_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?;
    Ok(dir.join("bin").join(with_exe_suffix(YTDLP)))
}

/// The updated copy (if present) shadows the bundled sidecar.
pub fn ytdlp_path(app: &AppHandle) -> Result<PathBuf, String> {
    let updated = updated_ytdlp_path(app)?;
    if updated.is_file() {
        return Ok(updated);
    }
    Ok(exe_dir()?.join(with_exe_suffix(YTDLP)))
}

/// Directory passed to yt-dlp as --ffmpeg-location (the sidecar ffmpeg lives
/// next to the app executable).
pub fn ffmpeg_location() -> Result<PathBuf, String> {
    exe_dir()
}

/// Base yt-dlp command with platform quirks applied.
pub fn ytdlp_command(app: &AppHandle) -> Result<Command, String> {
    let mut cmd = Command::new(ytdlp_path(app)?);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd.kill_on_drop(true);
    Ok(cmd)
}
