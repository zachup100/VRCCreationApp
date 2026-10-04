use crate::args::split_args;
use crate::prefs::{AppSettings, Prefs};
use crate::vpm::VpmContext;
use serde::Deserialize;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    default_project_location: Option<String>,
    backup_location: Option<String>,
    show_prerelease: Option<bool>,
    default_unity_args: Option<String>,
}

fn folder_setting(value: &str, label: &str) -> Result<Option<String>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path = Path::new(trimmed);
    if !path.is_absolute() || !path.is_dir() {
        return Err(format!("The {label} must be an existing folder"));
    }
    Ok(Some(trimmed.trim_end_matches(['/', '\\']).to_string()))
}

#[tauri::command]
pub fn get_settings(prefs: State<'_, Mutex<Prefs>>) -> Result<AppSettings, String> {
    let prefs = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?;
    Ok(prefs.settings().clone())
}

#[tauri::command]
pub fn update_settings(
    patch: SettingsPatch,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<AppSettings, String> {
    let _busy = vpm.try_busy()?;
    let location = patch
        .default_project_location
        .as_deref()
        .map(|value| folder_setting(value, "default project location"))
        .transpose()?;
    let backup = patch
        .backup_location
        .as_deref()
        .map(|value| folder_setting(value, "backup folder"))
        .transpose()?;
    if let Some(args) = patch.default_unity_args.as_deref() {
        split_args(args)?;
    }

    let mut prefs = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?;
    let settings = prefs.settings_mut();
    if let Some(value) = location {
        settings.default_project_location = value;
    }
    if let Some(value) = backup {
        settings.backup_location = value;
    }
    if let Some(value) = patch.show_prerelease {
        settings.show_prerelease = value;
    }
    if let Some(value) = patch.default_unity_args {
        settings.default_unity_args = value.trim().to_string();
    }
    let result = prefs.settings().clone();
    prefs.save().map_err(|error| error.to_string())?;
    Ok(result)
}
