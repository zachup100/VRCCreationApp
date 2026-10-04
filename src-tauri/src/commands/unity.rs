use crate::editors;
use crate::prefs::{CustomEditor, Prefs};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;
use vrc_get_vpm::unity_hub;

const ALLOWED_HOSTS: &[&str] = &[
    "unity.com",
    "vcc.docs.vrchat.com",
    "creators.vrchat.com",
    "docs.vrchat.com",
    "github.com",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityEditorInfo {
    version: String,
    path: String,
    architecture: Option<String>,
    custom: bool,
}

impl From<editors::Editor> for UnityEditorInfo {
    fn from(editor: editors::Editor) -> Self {
        Self {
            version: editor.version,
            path: editor.executable.to_string_lossy().into_owned(),
            architecture: editor.architecture,
            custom: editor.custom,
        }
    }
}

#[tauri::command]
pub async fn list_unity_editors(
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<Vec<UnityEditorInfo>, String> {
    Ok(editors::all(&prefs)
        .await?
        .into_iter()
        .map(UnityEditorInfo::from)
        .collect())
}

fn looks_like_unity(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name == "unity" || name == "unity.exe" || name.ends_with(".app")
}

#[tauri::command]
pub async fn add_custom_editor(
    path: String,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<UnityEditorInfo, String> {
    let chosen = PathBuf::from(path.trim());
    if !looks_like_unity(&chosen) {
        return Err("Choose the Unity program itself (Unity.exe, Unity, or Unity.app).".to_string());
    }
    let executable = unity_hub::get_executable_path(&chosen).into_owned();
    if !executable.is_file() {
        return Err("That file does not exist.".to_string());
    }

    let version = match unity_hub::load_unity_version(&chosen).await {
        Ok(version) => version,
        Err(_) => tokio::time::timeout(
            Duration::from_secs(90),
            vrc_get_vpm::unity::call_unity_for_version(&executable),
        )
        .await
        .map_err(|_| "Unity took too long to report its version.".to_string())?
        .map_err(|_| "Could not read the version of this Unity editor.".to_string())?,
    }
    .to_string();

    let existing = editors::all(&prefs).await.unwrap_or_default();
    if let Some(other) = existing
        .iter()
        .find(|editor| editor.executable == executable || editor.version == version)
    {
        return Err(format!(
            "Unity {} is already listed at {}.",
            other.version,
            other.executable.display()
        ));
    }

    let mut guard = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?;
    guard.add_custom_editor(CustomEditor {
        path: executable.to_string_lossy().into_owned(),
        version: version.clone(),
    });
    guard.save().map_err(|error| error.to_string())?;
    Ok(UnityEditorInfo {
        version,
        path: executable.to_string_lossy().into_owned(),
        architecture: None,
        custom: true,
    })
}

#[tauri::command]
pub fn remove_custom_editor(
    path: String,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let mut guard = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?;
    if !guard.remove_custom_editor(&path) {
        return Err("That editor is not in the custom list.".to_string());
    }
    guard.save().map_err(|error| error.to_string())
}

pub(crate) fn check_allowed(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| "Only https links can be opened".to_string())?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if !ALLOWED_HOSTS.contains(&host) {
        return Err("This link is not on the allowed list".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn open_external(url: String) -> Result<(), String> {
    check_allowed(&url)?;
    tokio::task::spawn_blocking(move || open::that_detached(url))
        .await
        .map_err(|_| "Could not open the link".to_string())?
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_https_links_pass() {
        assert!(check_allowed("https://unity.com/download").is_ok());
        assert!(check_allowed("https://creators.vrchat.com").is_ok());
        assert!(check_allowed("https://github.com?tab=readme").is_ok());
        assert!(check_allowed("https://vcc.docs.vrchat.com/vpm#section").is_ok());
    }

    #[test]
    fn plain_http_and_other_schemes_are_rejected() {
        assert!(check_allowed("http://unity.com/download").is_err());
        assert!(check_allowed("file:///etc/passwd").is_err());
        assert!(check_allowed("javascript:alert(1)").is_err());
        assert!(check_allowed("unity.com").is_err());
    }

    #[test]
    fn look_alike_hosts_are_rejected() {
        assert!(check_allowed("https://evil.com/").is_err());
        assert!(check_allowed("https://unity.com.evil.com/").is_err());
        assert!(check_allowed("https://unity.com@evil.com/").is_err());
        assert!(check_allowed("https://evil.com/unity.com").is_err());
    }
}