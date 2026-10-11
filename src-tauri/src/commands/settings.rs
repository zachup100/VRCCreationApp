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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_value_clears_the_setting() {
        assert_eq!(folder_setting("", "folder").unwrap(), None);
        assert_eq!(folder_setting("   ", "folder").unwrap(), None);
    }

    #[test]
    fn relative_and_missing_folders_are_rejected() {
        assert!(folder_setting("relative/path", "folder").is_err());
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        assert!(folder_setting(&missing.to_string_lossy(), "folder").is_err());
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(folder_setting(&file.to_string_lossy(), "folder").is_err());
    }

    #[test]
    fn an_existing_folder_is_stored_without_a_trailing_separator() {
        let dir = tempfile::tempdir().unwrap();
        let with_slash = format!("{}/", dir.path().to_string_lossy());
        let stored = folder_setting(&with_slash, "folder").unwrap().unwrap();
        assert_eq!(stored, dir.path().to_string_lossy());
    }
}
#[tauri::command]
pub fn get_private_host_blocking(vpm: State<'_, VpmContext>) -> bool {
    vpm.private_hosts_blocked()
}

#[tauri::command]
pub fn set_private_host_blocking(enabled: bool, vpm: State<'_, VpmContext>) -> bool {
    vpm.block_private_hosts
        .store(enabled, std::sync::atomic::Ordering::Relaxed);
    enabled
}
