use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const CAPACITY: usize = 500;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    id: u64,
    time: i64,
    level: &'static str,
    message: String,
}

#[derive(Default)]
pub struct AppLog {
    entries: Mutex<VecDeque<LogEntry>>,
    next: AtomicU64,
}

pub fn log(app: &AppHandle, level: &'static str, message: impl Into<String>) {
    let Some(state) = app.try_state::<AppLog>() else {
        return;
    };
    let entry = LogEntry {
        id: state.next.fetch_add(1, Ordering::Relaxed),
        time: crate::store::now_millis(),
        level,
        message: message.into(),
    };
    if let Ok(mut entries) = state.entries.lock() {
        if entries.len() >= CAPACITY {
            entries.pop_front();
        }
        entries.push_back(entry.clone());
    }
    let _ = app.emit("app-log", entry);
}

pub fn project_label(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

#[tauri::command]
pub fn get_logs(state: State<'_, AppLog>) -> Vec<LogEntry> {
    state
        .entries
        .lock()
        .map(|entries| entries.iter().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn clear_logs(state: State<'_, AppLog>) {
    if let Ok(mut entries) = state.entries.lock() {
        entries.clear();
    }
}
