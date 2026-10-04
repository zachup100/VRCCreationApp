use serde::Serialize;
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
pub fn open_external(url: String) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| "Only https links can be opened".to_string())?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if !ALLOWED_HOSTS.contains(&host) {
        return Err("This link is not on the allowed list".to_string());
    }
    open::that(&url).map_err(|error| error.to_string())
}
