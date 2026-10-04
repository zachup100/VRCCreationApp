use crate::store::{now_millis, path_key, ProjectStore, StoredProject};
use crate::vpm::VpmContext;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;
use vrc_get_vpm::environment::VccDatabaseConnection;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo};
use vrc_get_vpm::unity_hub;
use vrc_get_vpm::{ProjectType, UnityProject};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    path: String,
    name: String,
    unity_version: Option<String>,
    category: String,
    favorite: bool,
    last_modified: i64,
    exists: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    found: usize,
    imported: usize,
    skipped: usize,
}

pub(crate) fn category_of(project_type: ProjectType) -> &'static str {
    match project_type {
        ProjectType::LegacyAvatars => "legacy-avatar",
        ProjectType::LegacyWorlds | ProjectType::LegacySdk2 => "legacy-world",
        ProjectType::UpmAvatars | ProjectType::Avatars => "avatar",
        ProjectType::UpmWorlds | ProjectType::Worlds => "world",
        _ => "other",
    }
}

pub(crate) fn to_info(project: &StoredProject) -> ProjectInfo {
    let name = Path::new(&project.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&project.path)
        .to_string();
    ProjectInfo {
        exists: Path::new(&project.path).is_dir(),
        name,
        path: project.path.clone(),
        unity_version: project.unity_version.clone(),
        category: project.category.clone(),
        favorite: project.favorite,
        last_modified: project.last_modified,
    }
}

fn lock<'a>(
    state: &'a State<'_, Mutex<ProjectStore>>,
) -> Result<std::sync::MutexGuard<'a, ProjectStore>, String> {
    state.lock().map_err(|_| "Project store is unavailable".to_string())
}

#[tauri::command]
pub fn list_projects(state: State<'_, Mutex<ProjectStore>>) -> Result<Vec<ProjectInfo>, String> {
    let store = lock(&state)?;
    let mut projects: Vec<ProjectInfo> = store.projects().iter().map(to_info).collect();
    projects.sort_by(|a, b| {
        b.favorite
            .cmp(&a.favorite)
            .then(b.last_modified.cmp(&a.last_modified))
    });
    Ok(projects)
}

#[tauri::command]
pub async fn add_project(
    path: String,
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<ProjectInfo, String> {
    let _busy = vpm.try_busy()?;
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("The selected path is not a folder".to_string());
    }
    let project = UnityProject::load(DefaultProjectIo::new(root.into_boxed_path()))
        .await
        .map_err(|_| "The selected folder is not a Unity project".to_string())?;
    let stored = StoredProject {
        path: path.trim_end_matches(['/', '\\']).to_string(),
        unity_version: Some(project.unity_version().to_string()),
        category: category_of(project.detect_project_type().await).to_string(),
        favorite: false,
        added_at: now_millis(),
        last_modified: now_millis(),
    };
    let mut store = lock(&state)?;
    if !store.insert(stored.clone()) {
        return Err("This project is already in the list".to_string());
    }
    store.save().map_err(|error| error.to_string())?;
    Ok(to_info(&stored))
}

#[tauri::command]
pub fn remove_project(
    path: String,
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    let mut store = lock(&state)?;
    if store.remove(&path) {
        store.save().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_project_favorite(
    path: String,
    favorite: bool,
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    let mut store = lock(&state)?;
    if store.set_favorite(&path, favorite) {
        store.save().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn import_from_vcc(
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<ImportResult, String> {
    let _busy = vpm.try_busy()?;
    let io = DefaultEnvironmentIo::new_default();
    let database_file = io.resolve(Path::new("vcc.liteDb"));
    if !database_file.is_file() {
        return Err(format!(
            "No VCC project database found at {}",
            database_file.display()
        ));
    }

    let candidates: Vec<StoredProject> = {
        let connection = VccDatabaseConnection::connect(&io)
            .await
            .map_err(|error| format!("Could not read the VCC database: {error}"))?;
        connection
            .get_projects()
            .iter()
            .filter_map(|project| {
                let path = project.path()?.trim_end_matches(['/', '\\']).to_string();
                let added = project
                    .crated_at()
                    .map(|date| date.as_unix_milliseconds())
                    .unwrap_or_else(now_millis);
                let modified = project
                    .last_modified()
                    .map(|date| date.as_unix_milliseconds())
                    .unwrap_or(added);
                Some(StoredProject {
                    path,
                    unity_version: project.unity_version().map(|version| version.to_string()),
                    category: category_of(project.project_type()).to_string(),
                    favorite: project.favorite(),
                    added_at: added,
                    last_modified: modified,
                })
            })
            .collect()
    };

    let found = candidates.len();
    let mut store = lock(&state)?;
    let mut imported = 0;
    let mut seen = std::collections::HashSet::new();
    for candidate in candidates {
        if seen.insert(path_key(&candidate.path)) && store.insert(candidate) {
            imported += 1;
        }
    }
    if imported > 0 {
        store.save().map_err(|error| error.to_string())?;
    }
    Ok(ImportResult {
        found,
        imported,
        skipped: found - imported,
    })
}

#[tauri::command]
pub async fn open_project(
    path: String,
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    let version = {
        let store = lock(&state)?;
        let project = store
            .find(&path)
            .ok_or_else(|| "This project is not in the list".to_string())?;
        project
            .unity_version
            .clone()
            .ok_or_else(|| "The Unity version of this project is unknown".to_string())?
    };
    if !Path::new(&path).is_dir() {
        return Err("The project folder no longer exists".to_string());
    }
    let editors = unity_hub::load_unity_by_loading_unity_hub_files()
        .await
        .map_err(|error| error.to_string())?;
    let editor = editors
        .iter()
        .find(|editor| editor.version.to_string() == version)
        .ok_or_else(|| format!("Unity {version} is not installed"))?;
    let executable = unity_hub::get_executable_path(&editor.path).into_owned();
    std::process::Command::new(executable)
        .arg("-projectPath")
        .arg(&path)
        .spawn()
        .map_err(|error| format!("Could not start Unity: {error}"))?;
    let mut store = lock(&state)?;
    store.touch(&path);
    store.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_project_folder(
    path: String,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    {
        let store = lock(&state)?;
        if store.find(&path).is_none() {
            return Err("This project is not in the list".to_string());
        }
    }
    open::that(&path).map_err(|error| error.to_string())
}
