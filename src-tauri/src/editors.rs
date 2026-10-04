use crate::prefs::Prefs;
use std::path::PathBuf;
use std::sync::Mutex;
use vrc_get_vpm::unity_hub;

pub struct Editor {
    pub version: String,
    pub executable: PathBuf,
    pub architecture: Option<String>,
    pub custom: bool,
}

pub async fn all(prefs: &Mutex<Prefs>) -> Result<Vec<Editor>, String> {
    let custom = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .custom_editors();

    let mut editors: Vec<Editor> = match unity_hub::load_unity_by_loading_unity_hub_files().await {
        Ok(found) => found
            .into_iter()
            .map(|editor| Editor {
                version: editor.version.to_string(),
                executable: unity_hub::get_executable_path(&editor.path).into_owned(),
                architecture: editor
                    .architecture
                    .map(|architecture| format!("{architecture:?}").to_lowercase()),
                custom: false,
            })
            .collect(),
        Err(error) if custom.is_empty() => return Err(error.to_string()),
        Err(_) => Vec::new(),
    };

    for entry in custom {
        let executable = PathBuf::from(&entry.path);
        let known = editors
            .iter()
            .any(|editor| editor.executable == executable || editor.version == entry.version);
        if !known && executable.exists() {
            editors.push(Editor {
                version: entry.version,
                executable,
                architecture: None,
                custom: true,
            });
        }
    }

    editors.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(editors)
}

pub async fn executable_for(prefs: &Mutex<Prefs>, version: &str) -> Result<PathBuf, String> {
    all(prefs)
        .await?
        .into_iter()
        .find(|editor| editor.version == version)
        .map(|editor| editor.executable)
        .ok_or_else(|| format!("Unity {version} is not installed"))
}
