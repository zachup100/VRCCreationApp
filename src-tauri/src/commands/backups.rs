use crate::applog::log;
use crate::commands::create::validate_name;
use crate::commands::migrate::{backup_directory, emit, BackupMeta, TaskProgress, BACKUP_MARKER};
use crate::commands::projects::{category_of, to_info, ProjectInfo};
use crate::prefs::Prefs;
use crate::store::{now_millis, ProjectStore, StoredProject};
use crate::vpm::VpmContext;
use chrono::{Local, NaiveDateTime, TimeZone};
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::UnityProject;
use zip::ZipArchive;

const STAMP_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";
const STAMP_LENGTH: usize = 19;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    file_name: String,
    project_name: String,
    category: String,
    unity_version: Option<String>,
    created_at: i64,
    size_bytes: u64,
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RestoreOutcome {
    Restored { project: ProjectInfo },
    Conflict { name: String },
}

fn read_text(archive: &mut ZipArchive<BufReader<File>>, name: &str) -> Option<String> {
    let entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.take(1024 * 1024).read_to_string(&mut text).ok()?;
    Some(text)
}

fn read_backup(path: &Path) -> Option<BackupInfo> {
    let size_bytes = std::fs::metadata(path).ok()?.len();
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let mut archive = ZipArchive::new(BufReader::new(File::open(path).ok()?)).ok()?;

    if let Ok(meta) = serde_json::from_slice::<BackupMeta>(archive.comment()) {
        if meta.app == BACKUP_MARKER {
            return Some(BackupInfo {
                file_name,
                project_name: meta.project_name,
                category: meta.category,
                unity_version: meta.unity_version,
                created_at: meta.created_at,
                size_bytes,
            });
        }
    }

    let stem = path.file_stem()?.to_string_lossy().into_owned();
    if stem.len() <= STAMP_LENGTH + 1 || !stem.is_char_boundary(stem.len() - STAMP_LENGTH - 1) {
        return None;
    }
    let (name_part, stamp) = stem.split_at(stem.len() - STAMP_LENGTH);
    let name = name_part.strip_suffix('-')?;
    let created = NaiveDateTime::parse_from_str(stamp, STAMP_FORMAT).ok()?;
    let created_at = Local.from_local_datetime(&created).earliest()?.timestamp_millis();

    let version_text = read_text(&mut archive, &format!("{name}/ProjectSettings/ProjectVersion.txt"))?;
    let unity_version = version_text
        .lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion:"))
        .map(|value| value.trim().to_string());

    let category = match read_text(&mut archive, &format!("{name}/Packages/vpm-manifest.json")) {
        Some(manifest) if manifest.contains("com.vrchat.avatars") => "avatar",
        Some(manifest) if manifest.contains("com.vrchat.worlds") => "world",
        _ => "other",
    };

    Some(BackupInfo {
        file_name,
        project_name: name.to_string(),
        category: category.to_string(),
        unity_version,
        created_at,
        size_bytes,
    })
}

fn resolve_backup(directory: &Path, file_name: &str) -> Result<(PathBuf, BackupInfo), String> {
    let plain = Path::new(file_name)
        .file_name()
        .map(|name| name.to_string_lossy() == file_name)
        .unwrap_or(false);
    if !plain || !file_name.to_ascii_lowercase().ends_with(".zip") {
        return Err("This is not a backup file".to_string());
    }
    let path = directory.join(file_name);
    if !path.is_file() {
        return Err("The backup file no longer exists".to_string());
    }
    let info = read_backup(&path).ok_or_else(|| "This is not a backup made by this app".to_string())?;
    Ok((path, info))
}

#[tauri::command]
pub async fn list_backups(
    app: AppHandle,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<Vec<BackupInfo>, String> {
    let directory = backup_directory(&app, &prefs)?;
    tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            return Vec::new();
        };
        let mut backups: Vec<BackupInfo> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".zip")
            })
            .filter_map(|entry| read_backup(&entry.path()))
            .collect();
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        backups
    })
    .await
    .map_err(|_| "Could not read the backups".to_string())
}

#[tauri::command]
pub async fn delete_backup(
    app: AppHandle,
    file_name: String,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    let directory = backup_directory(&app, &prefs)?;
    let (path, info) = tokio::task::spawn_blocking(move || resolve_backup(&directory, &file_name))
        .await
        .map_err(|_| "Could not read the backup".to_string())??;
    tokio::fs::remove_file(path)
        .await
        .map_err(|error| format!("Could not delete the backup: {error}"))?;
    log(&app, "info", format!("Deleted the backup {}", info.file_name));
    Ok(())
}

fn backup_root(archive: &mut ZipArchive<BufReader<File>>) -> Result<String, String> {
    let mut root: Option<String> = None;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("The backup is damaged: {error}"))?;
        let Some(path) = entry.enclosed_name() else {
            return Err("The backup contains an unsafe path".to_string());
        };
        let Some(first) = path.components().next() else {
            continue;
        };
        let first = first.as_os_str().to_string_lossy().into_owned();
        match &root {
            None => root = Some(first),
            Some(existing) if *existing == first => {}
            Some(_) => return Err("The backup has an unexpected layout".to_string()),
        }
    }
    root.ok_or_else(|| "The backup is empty".to_string())
}

fn extract_backup(app: &AppHandle, zip_path: &Path, target: &Path) -> Result<(), String> {
    let mut archive = ZipArchive::new(BufReader::new(
        File::open(zip_path).map_err(|error| format!("Could not open the backup: {error}"))?,
    ))
    .map_err(|error| format!("The backup is damaged: {error}"))?;
    let root = backup_root(&mut archive)?;
    let total = archive.len();

    for index in 0..total {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("The backup is damaged: {error}"))?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        let Ok(relative) = path.strip_prefix(&root) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        let destination = target.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination)
                .map_err(|error| format!("Could not write the project files: {error}"))?;
        } else {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("Could not write the project files: {error}"))?;
            }
            let mut output = File::create(&destination)
                .map_err(|error| format!("Could not write the project files: {error}"))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|error| format!("Could not write the project files: {error}"))?;
        }
        if index % 25 == 0 || index + 1 == total {
            emit(app, TaskProgress::Restore { done: index + 1, total });
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    file_name: String,
    location: String,
    name: Option<String>,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<RestoreOutcome, String> {
    let _busy = vpm.try_busy()?;
    let directory = backup_directory(&app, &prefs)?;
    let (zip_path, info) = {
        let directory = directory.clone();
        tokio::task::spawn_blocking(move || resolve_backup(&directory, &file_name))
            .await
            .map_err(|_| "Could not read the backup".to_string())??
    };

    let parent = PathBuf::from(location.trim());
    if !parent.is_absolute() || !parent.is_dir() {
        return Err("The project location is not an existing folder".to_string());
    }

    let project_name = match name {
        Some(chosen) => chosen.trim().to_string(),
        None => info.project_name.clone(),
    };
    validate_name(&project_name)?;
    let target = parent.join(&project_name);
    match tokio::fs::create_dir(&target).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Ok(RestoreOutcome::Conflict { name: project_name });
        }
        Err(error) => return Err(format!("Could not create the project folder: {error}")),
    }

    let extraction = {
        let handle = app.clone();
        let source = zip_path.clone();
        let destination = target.clone();
        tokio::task::spawn_blocking(move || extract_backup(&handle, &source, &destination))
            .await
            .map_err(|_| "The restore was interrupted".to_string())
            .and_then(|result| result)
    };
    if let Err(message) = extraction {
        let _ = tokio::fs::remove_dir_all(&target).await;
        return Err(message);
    }

    let (unity_version, category) =
        match UnityProject::load(DefaultProjectIo::new(target.clone().into_boxed_path())).await {
            Ok(loaded) => (
                Some(loaded.unity_version().to_string()),
                category_of(loaded.detect_project_type().await).to_string(),
            ),
            Err(_) => (info.unity_version.clone(), info.category.clone()),
        };

    let stored = StoredProject {
        path: target.to_string_lossy().into_owned(),
        unity_version: unity_version.clone(),
        category: category.clone(),
        favorite: false,
        added_at: now_millis(),
        last_modified: now_millis(),
        launch_args: None,
    };
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    if !store.insert(stored.clone()) {
        store.update_info(&stored.path, unity_version, category);
    }
    store.save().map_err(|error| error.to_string())?;
    log(&app, "success", format!("Restored {} to {}", info.project_name, stored.path));
    Ok(RestoreOutcome::Restored {
        project: to_info(&stored),
    })
}
