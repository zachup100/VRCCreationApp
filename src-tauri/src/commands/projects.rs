use crate::applog::{log, project_label};
use crate::args::split_args;
use crate::prefs::Prefs;
use crate::store::{now_millis, path_key, ProjectStore, StoredProject};
use crate::vpm::VpmContext;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};
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
    launch_args: Option<String>,
    exists: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    found: usize,
    imported: usize,
    skipped: usize,
}

pub(crate) fn vpm_category(has_avatars: bool, has_worlds: bool, direct_avatars: bool, direct_worlds: bool) -> Option<&'static str> {
    match (has_avatars, has_worlds) {
        (true, false) => Some("avatar"),
        (false, true) => Some("world"),
        (true, true) if direct_worlds && !direct_avatars => Some("world"),
        (true, true) => Some("avatar"),
        (false, false) => None,
    }
}

pub(crate) async fn detect_type(project: &UnityProject) -> ProjectType {
    let direct = |name: &str| project.dependencies().any(|dependency| dependency == name);
    match vpm_category(
        project.is_locked("com.vrchat.avatars"),
        project.is_locked("com.vrchat.worlds"),
        direct("com.vrchat.avatars"),
        direct("com.vrchat.worlds"),
    ) {
        Some("world") => ProjectType::Worlds,
        Some(_) => ProjectType::Avatars,
        None => project.detect_project_type().await,
    }
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
        launch_args: project.launch_args.clone(),
    }
}

fn lock<'a>(
    state: &'a State<'_, Mutex<ProjectStore>>,
) -> Result<std::sync::MutexGuard<'a, ProjectStore>, String> {
    state.lock().map_err(|_| "Project store is unavailable".to_string())
}

#[tauri::command]
pub async fn list_projects(state: State<'_, Mutex<ProjectStore>>) -> Result<Vec<ProjectInfo>, String> {
    let stored: Vec<StoredProject> = lock(&state)?.projects().to_vec();
    let mut projects: Vec<ProjectInfo> = tokio::task::spawn_blocking(move || stored.iter().map(to_info).collect())
        .await
        .map_err(|_| "Could not read the project list".to_string())?;
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
        category: category_of(detect_type(&project).await).to_string(),
        favorite: false,
        added_at: now_millis(),
        last_modified: now_millis(),
        launch_args: None,
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
    app: AppHandle,
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
                    launch_args: None,
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
    log(&app, "success", format!("Imported projects from VCC: {imported} added, {} already in the list", found - imported));
    Ok(ImportResult {
        found,
        imported,
        skipped: found - imported,
    })
}

#[tauri::command]
pub async fn open_project(
    app: AppHandle,
    path: String,
    editor_version: Option<String>,
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    let (project_version, project_args) = {
        let store = lock(&state)?;
        let project = store
            .find(&path)
            .ok_or_else(|| "This project is not in the list".to_string())?;
        (project.unity_version.clone(), project.launch_args.clone())
    };
    let version = editor_version
        .or(project_version)
        .ok_or_else(|| "The Unity version of this project is unknown".to_string())?;
    let default_args = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .default_unity_args
        .clone();
    let args = split_args(
        project_args
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(&default_args),
    )?;
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
        .args(args)
        .spawn()
        .map_err(|error| format!("Could not start Unity: {error}"))?;
    log(&app, "info", format!("Opened {} with Unity {version}", project_label(&path)));
    let mut store = lock(&state)?;
    store.touch(&path);
    store.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_project_launch_args(
    path: String,
    args: String,
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    split_args(&args)?;
    let trimmed = args.trim();
    let value = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    let mut store = lock(&state)?;
    if !store.set_launch_args(&path, value) {
        return Err("This project is not in the list".to_string());
    }
    store.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn open_project_folder(
    path: String,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    {
        let store = lock(&state)?;
        if store.find(&path).is_none() {
            return Err("This project is not in the list".to_string());
        }
    }
    tokio::task::spawn_blocking(move || open::that_detached(path))
        .await
        .map_err(|_| "Could not open the folder".to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn refresh_project_info(
    vpm: State<'_, VpmContext>,
    state: State<'_, Mutex<ProjectStore>>,
) -> Result<usize, String> {
    let Ok(_busy) = vpm.try_busy() else {
        return Ok(0);
    };
    let paths: Vec<String> = lock(&state)?
        .projects()
        .iter()
        .map(|project| project.path.clone())
        .filter(|path| Path::new(path).is_dir())
        .collect();
    let mut updates = Vec::new();
    for path in paths {
        let Ok(loaded) = UnityProject::load(DefaultProjectIo::new(PathBuf::from(&path).into_boxed_path())).await else {
            continue;
        };
        let category = category_of(detect_type(&loaded).await).to_string();
        updates.push((path, loaded.unity_version().to_string(), category));
    }
    let mut store = lock(&state)?;
    let mut changed = 0;
    for (path, version, category) in updates {
        if let Some(current) = store.find(&path) {
            if current.unity_version.as_deref() != Some(version.as_str()) || current.category != category {
                store.update_details(&path, Some(version), category);
                changed += 1;
            }
        }
    }
    if changed > 0 {
        store.save().map_err(|error| error.to_string())?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_types_map_to_the_categories_the_interface_uses() {
        assert_eq!(category_of(ProjectType::Avatars), "avatar");
        assert_eq!(category_of(ProjectType::UpmAvatars), "avatar");
        assert_eq!(category_of(ProjectType::Worlds), "world");
        assert_eq!(category_of(ProjectType::UpmWorlds), "world");
        assert_eq!(category_of(ProjectType::LegacyAvatars), "legacy-avatar");
        assert_eq!(category_of(ProjectType::LegacyWorlds), "legacy-world");
        assert_eq!(category_of(ProjectType::LegacySdk2), "legacy-world");
        assert_eq!(category_of(ProjectType::Unknown), "other");
        assert_eq!(category_of(ProjectType::VpmStarter), "other");
    }

    #[test]
    fn a_single_sdk_decides_the_category() {
        assert_eq!(vpm_category(true, false, false, false), Some("avatar"));
        assert_eq!(vpm_category(false, true, false, false), Some("world"));
        assert_eq!(vpm_category(false, false, true, true), None);
    }

    #[test]
    fn with_both_sdks_the_directly_requested_one_wins() {
        assert_eq!(vpm_category(true, true, false, true), Some("world"));
        assert_eq!(vpm_category(true, true, true, false), Some("avatar"));
        assert_eq!(vpm_category(true, true, true, true), Some("avatar"));
        assert_eq!(vpm_category(true, true, false, false), Some("avatar"));
    }
}
