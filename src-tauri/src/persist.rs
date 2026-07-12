use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const HISTORY_STORE: &str = "history.json";
const HISTORY_CAP: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub filepath: Option<String>,
    pub audio_only: bool,
    pub completed_at: u64,
}

/// Written from Rust on completion so history is correct even when the
/// frontend is on another tab (or mid-reload in dev).
pub fn append_history(app: &AppHandle, entry: HistoryEntry) {
    let Ok(store) = app.store(HISTORY_STORE) else {
        return;
    };
    let mut entries = store
        .get("entries")
        .and_then(|v| serde_json::from_value::<Vec<HistoryEntry>>(v).ok())
        .unwrap_or_default();
    entries.insert(0, entry);
    entries.truncate(HISTORY_CAP);
    store.set("entries", json!(entries));
    let _ = store.save();
}
