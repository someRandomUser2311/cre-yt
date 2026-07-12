mod download;
mod engine;
mod info;
mod persist;
mod updater;

use std::sync::Arc;
use tauri::Manager;

const DEFAULT_MAX_CONCURRENT: usize = 2;

/// Sensible default save location shown before the user picks one.
#[tauri::command]
fn suggest_download_dir(app: tauri::AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .download_dir()
        .or_else(|_| app.path().video_dir())
        .or_else(|_| app.path().home_dir())
        .map_err(|e| format!("no usable download directory: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(Arc::new(download::DownloadManager::new(DEFAULT_MAX_CONCURRENT)))
        .invoke_handler(tauri::generate_handler![
            suggest_download_dir,
            info::fetch_video_info,
            download::start_download,
            download::pause_download,
            download::resume_download,
            download::cancel_download,
            download::remove_job,
            download::get_queue,
            download::set_max_concurrent,
            updater::get_ytdlp_version,
            updater::update_ytdlp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
