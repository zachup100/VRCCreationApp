use crate::editors;
use crate::hub;
use crate::prefs::Prefs;
use crate::vpm::VpmContext;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HubStatus {
    found: bool,
    path: Option<String>,
    can_install: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedEditor {
    version: &'static str,
    label: &'static str,
    note: &'static str,
}

#[tauri::command]
pub fn unity_hub_status() -> HubStatus {
    let found = hub::find_hub();
    HubStatus {
        found: found.is_some(),
        path: found.map(|path| path.to_string_lossy().into_owned()),
        can_install: hub::can_install_hub(),
    }
}

#[tauri::command]
pub fn recommended_editors() -> Vec<RecommendedEditor> {
    hub::RECOMMENDED
        .iter()
        .map(|entry| RecommendedEditor {
            version: entry.version,
            label: entry.label,
            note: entry.note,
        })
        .collect()
}

#[tauri::command]
pub async fn install_unity_hub(app: AppHandle, vpm: State<'_, VpmContext>) -> Result<(), String> {
    if !hub::can_install_hub() {
        return Err("Installing Unity Hub from here is only supported on Windows. Open the download page instead.".to_string());
    }
    if hub::find_hub().is_some() {
        return Err("Unity Hub is already installed.".to_string());
    }
    let session = hub::Session::begin(&app, &vpm, "hub", "")?;
    tauri::async_runtime::spawn(hub::run_hub_install(session));
    Ok(())
}

#[tauri::command]
pub async fn install_unity_editor(
    app: AppHandle,
    version: String,
    android: bool,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let entry = hub::recommended(&version)
        .ok_or_else(|| "That version is not on the recommended list.".to_string())?;
    if hub::find_hub().is_none() {
        return Err("Unity Hub was not found. Install it first.".to_string());
    }
    let installed = editors::all(&prefs)
        .await
        .map(|found| found.iter().any(|editor| editor.version == entry.version))
        .unwrap_or(false);
    if installed {
        return Err(format!("Unity {} is already installed.", entry.version));
    }
    let session = hub::Session::begin(&app, &vpm, "editor", entry.version)?;
    tauri::async_runtime::spawn(hub::run_editor_install(session, android));
    Ok(())
}

#[tauri::command]
pub fn cancel_unity_install(vpm: State<'_, VpmContext>) -> bool {
    hub::request_cancel(&vpm)
}
