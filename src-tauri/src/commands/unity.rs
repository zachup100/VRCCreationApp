use serde::Serialize;
use vrc_get_vpm::unity_hub;

const UNITY_DOWNLOAD_URL: &str = "https://unity.com/download";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityEditorInfo {
    version: String,
    path: String,
    architecture: Option<String>,
}

#[tauri::command]
pub async fn list_unity_editors() -> Result<Vec<UnityEditorInfo>, String> {
    let editors = unity_hub::load_unity_by_loading_unity_hub_files()
        .await
        .map_err(|error| error.to_string())?;

    let mut result: Vec<UnityEditorInfo> = editors
        .into_iter()
        .map(|editor| UnityEditorInfo {
            version: editor.version.to_string(),
            path: unity_hub::get_executable_path(&editor.path)
                .to_string_lossy()
                .into_owned(),
            architecture: editor
                .architecture
                .map(|architecture| format!("{architecture:?}").to_lowercase()),
        })
        .collect();

    result.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(result)
}

#[tauri::command]
pub fn open_unity_download_page() -> Result<(), String> {
    open::that(UNITY_DOWNLOAD_URL).map_err(|error| error.to_string())
}
