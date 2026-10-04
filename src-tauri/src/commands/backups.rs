use crate::applog::log;
use crate::commands::create::validate_name;
use crate::commands::migrate::{backup_directory, emit, BackupMeta, TaskProgress, BACKUP_MARKER, LEGACY_BACKUP_MARKER};
use crate::commands::projects::{category_of, detect_type, to_info, vpm_category, ProjectInfo};
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

fn category_from_manifest(text: &str) -> Option<&'static str> {
    let document: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let keys = |section: &str| -> Vec<String> {
        document
            .get(section)
            .and_then(serde_json::Value::as_object)
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    };
    let locked = keys("locked");
    let direct = keys("dependencies");
    let has = |list: &[String], name: &str| list.iter().any(|item| item == name);
    vpm_category(
        has(&locked, "com.vrchat.avatars") || has(&direct, "com.vrchat.avatars"),
        has(&locked, "com.vrchat.worlds") || has(&direct, "com.vrchat.worlds"),
        has(&direct, "com.vrchat.avatars"),
        has(&direct, "com.vrchat.worlds"),
    )
}

fn read_backup(path: &Path) -> Option<BackupInfo> {
    let size_bytes = std::fs::metadata(path).ok()?.len();
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let mut archive = ZipArchive::new(BufReader::new(File::open(path).ok()?)).ok()?;

    if let Ok(meta) = serde_json::from_slice::<BackupMeta>(archive.comment()) {
        if meta.app == BACKUP_MARKER || meta.app == LEGACY_BACKUP_MARKER {
            let derived = read_text(&mut archive, &format!("{}/Packages/vpm-manifest.json", meta.project_name))
                .and_then(|text| category_from_manifest(&text));
            return Some(BackupInfo {
                file_name,
                project_name: meta.project_name,
                category: derived.map(str::to_string).unwrap_or(meta.category),
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

    let category = read_text(&mut archive, &format!("{name}/Packages/vpm-manifest.json"))
        .and_then(|text| category_from_manifest(&text))
        .unwrap_or("other");

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

fn extract_backup(
    zip_path: &Path,
    target: &Path,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<(), String> {
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
            progress(index + 1, total);
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
        tokio::task::spawn_blocking(move || {
            let mut report = |done: usize, total: usize| emit(&handle, TaskProgress::Restore { done, total });
            extract_backup(&source, &destination, &mut report)
        })
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
                category_of(detect_type(&loaded).await).to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::migrate::write_backup;
    use std::io::Write;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn sample_project(root: &Path) {
        write(root, "Assets/a.txt", "asset contents");
        write(root, "Packages/vpm-manifest.json", r#"{"dependencies":{"com.vrchat.worlds":{"version":"3.x"}}}"#);
        write(root, "ProjectSettings/ProjectVersion.txt", "m_EditorVersion: 2022.3.22f1\nm_EditorVersionWithRevision: 2022.3.22f1 (abc)\n");
        write(root, "Library/cache.bin", "excluded");
    }

    fn make_backup(parent: &Path, name: &str, file_name: &str) -> PathBuf {
        let project = parent.join(name);
        sample_project(&project);
        let target = parent.join(file_name);
        let mut report = |_: usize, _: usize| {};
        write_backup(&mut report, &project, &target, Some("2022.3.22f1".to_string()), "world".to_string()).unwrap();
        target
    }

    fn plain_zip(path: &Path, entries: &[(&str, &str)], comment: Option<&str>) {
        let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents.as_bytes()).unwrap();
        }
        if let Some(comment) = comment {
            writer.set_comment(comment);
        }
        writer.finish().unwrap();
    }

    #[test]
    fn a_marked_backup_is_recognised_with_its_stored_details() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_backup(dir.path(), "My Project", "My Project-2026-10-03_21-30-00.zip");
        let info = read_backup(&path).unwrap();
        assert_eq!(info.project_name, "My Project");
        assert_eq!(info.category, "world");
        assert_eq!(info.unity_version.as_deref(), Some("2022.3.22f1"));
        assert_eq!(info.file_name, "My Project-2026-10-03_21-30-00.zip");
        assert!(info.created_at > 0);
        assert!(info.size_bytes > 0);
    }

    #[test]
    fn an_older_backup_without_a_marker_is_recognised_by_name_and_layout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Old World-2026-01-02_03-04-05.zip");
        plain_zip(
            &path,
            &[
                ("Old World/ProjectSettings/ProjectVersion.txt", "m_EditorVersion: 2019.4.31f1\nm_EditorVersionWithRevision: 2019.4.31f1 (x)\n"),
                ("Old World/Packages/vpm-manifest.json", r#"{"dependencies":{"com.vrchat.avatars":{"version":"3.x"}}}"#),
            ],
            None,
        );
        let info = read_backup(&path).unwrap();
        assert_eq!(info.project_name, "Old World");
        assert_eq!(info.unity_version.as_deref(), Some("2019.4.31f1"));
        assert_eq!(info.category, "avatar");
        assert!(info.created_at > 0);
    }

    #[test]
    fn zips_that_the_app_did_not_make_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let random = dir.path().join("holiday-photos.zip");
        plain_zip(&random, &[("a.txt", "x")], None);
        assert!(read_backup(&random).is_none());

        let wrong_marker = dir.path().join("x.zip");
        plain_zip(&wrong_marker, &[("a.txt", "x")], Some(r#"{"app":"SomethingElse","version":1,"projectName":"x","category":"other","unityVersion":null,"createdAt":1}"#));
        assert!(read_backup(&wrong_marker).is_none());

        let lookalike = dir.path().join("Name-2026-01-02_03-04-05.zip");
        plain_zip(&lookalike, &[("Name/readme.txt", "no project version here")], None);
        assert!(read_backup(&lookalike).is_none());

        let not_a_zip = dir.path().join("broken.zip");
        std::fs::write(&not_a_zip, "not a zip file").unwrap();
        assert!(read_backup(&not_a_zip).is_none());
    }

    #[test]
    fn only_plain_zip_file_names_inside_the_folder_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_backup(dir.path(), "P", "P-2026-10-03_21-30-00.zip");
        assert!(resolve_backup(dir.path(), "P-2026-10-03_21-30-00.zip").is_ok());
        assert!(resolve_backup(dir.path(), "../P-2026-10-03_21-30-00.zip").is_err());
        assert!(resolve_backup(dir.path(), "sub/P.zip").is_err());
        assert!(resolve_backup(dir.path(), "notes.txt").is_err());
        assert!(resolve_backup(dir.path(), "missing.zip").is_err());
        assert!(resolve_backup(dir.path(), &path.to_string_lossy()).is_err());

        let foreign = dir.path().join("other.zip");
        plain_zip(&foreign, &[("a.txt", "x")], None);
        assert!(resolve_backup(dir.path(), "other.zip").is_err());
    }

    #[test]
    fn a_backup_restores_to_the_same_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_backup(dir.path(), "My Project", "My Project-2026-10-03_21-30-00.zip");
        let target = dir.path().join("restored");
        std::fs::create_dir(&target).unwrap();
        let mut calls = 0;
        let mut report = |_: usize, _: usize| calls += 1;
        extract_backup(&path, &target, &mut report).unwrap();
        assert!(calls > 0);
        assert_eq!(std::fs::read_to_string(target.join("Assets/a.txt")).unwrap(), "asset contents");
        assert!(target.join("Packages/vpm-manifest.json").is_file());
        assert!(target.join("ProjectSettings/ProjectVersion.txt").is_file());
        assert!(!target.join("Library").exists());
        assert!(!target.join("My Project").exists());
    }

    #[test]
    fn entries_that_escape_the_folder_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let evil = dir.path().join("evil.zip");
        plain_zip(&evil, &[("Name/ok.txt", "fine"), ("Name/../../escaped.txt", "bad")], None);
        let target = dir.path().join("out");
        std::fs::create_dir(&target).unwrap();
        let mut report = |_: usize, _: usize| {};
        assert!(extract_backup(&evil, &target, &mut report).is_err());
        assert!(!dir.path().join("escaped.txt").exists());
        assert!(!target.join("escaped.txt").exists());
    }

    #[test]
    fn a_backup_with_more_than_one_top_level_folder_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mixed = dir.path().join("mixed.zip");
        plain_zip(&mixed, &[("One/a.txt", "a"), ("Two/b.txt", "b")], None);
        let target = dir.path().join("out");
        std::fs::create_dir(&target).unwrap();
        let mut report = |_: usize, _: usize| {};
        assert!(extract_backup(&mixed, &target, &mut report).is_err());
    }

    #[test]
    fn the_project_type_comes_from_the_direct_sdk_when_both_are_installed() {
        let both = r#"{"dependencies":{"com.vrchat.worlds":{"version":"3.x"}},"locked":{"com.vrchat.worlds":{"version":"3.10.5"},"com.vrchat.avatars":{"version":"3.10.5"}}}"#;
        assert_eq!(category_from_manifest(both), Some("world"));
        let avatar_direct = r#"{"dependencies":{"com.vrchat.avatars":{"version":"3.x"}},"locked":{"com.vrchat.worlds":{},"com.vrchat.avatars":{}}}"#;
        assert_eq!(category_from_manifest(avatar_direct), Some("avatar"));
        let single = r#"{"dependencies":{},"locked":{"com.vrchat.worlds":{}}}"#;
        assert_eq!(category_from_manifest(single), Some("world"));
        assert_eq!(category_from_manifest(r#"{"dependencies":{},"locked":{}}"#), None);
        assert_eq!(category_from_manifest("not json"), None);
    }

    #[test]
    fn a_stored_type_is_corrected_from_the_manifest_in_the_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Mixed-2026-10-03_21-30-00.zip");
        let manifest = r#"{"dependencies":{"com.vrchat.worlds":{"version":"3.x"}},"locked":{"com.vrchat.worlds":{},"com.vrchat.avatars":{}}}"#;
        let meta = r#"{"app":"VRCreationApp","version":1,"projectName":"Mixed","category":"avatar","unityVersion":"2022.3.22f1","createdAt":1}"#;
        plain_zip(&path, &[("Mixed/Packages/vpm-manifest.json", manifest)], Some(meta));
        assert_eq!(read_backup(&path).unwrap().category, "world");
    }

    #[test]
    fn a_stored_type_is_kept_when_the_backup_has_no_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Legacy-2026-10-03_21-30-00.zip");
        let meta = r#"{"app":"VRCreationApp","version":1,"projectName":"Legacy","category":"legacy-world","unityVersion":"2019.4.31f1","createdAt":1}"#;
        plain_zip(&path, &[("Legacy/Assets/a.txt", "x")], Some(meta));
        assert_eq!(read_backup(&path).unwrap().category, "legacy-world");
    }

    #[test]
    fn backups_made_under_the_new_and_the_old_app_name_are_both_recognised() {
        let dir = tempfile::tempdir().unwrap();
        for (index, app) in ["VRCCreationApp", "VRCreationApp"].iter().enumerate() {
            let path = dir.path().join(format!("P{index}-2026-10-03_21-30-0{index}.zip"));
            let meta = format!(r#"{{"app":"{app}","version":1,"projectName":"P{index}","category":"world","unityVersion":"2022.3.22f1","createdAt":1}}"#);
            plain_zip(&path, &[(&format!("P{index}/Assets/a.txt"), "x")], Some(&meta));
            assert!(read_backup(&path).is_some(), "{app} should be recognised");
        }
    }

    #[test]
    fn an_sdk_listed_only_as_a_dependency_still_decides_the_type() {
        let avatar = r#"{"dependencies":{"com.vrchat.avatars":{"version":"3.x"}}}"#;
        assert_eq!(category_from_manifest(avatar), Some("avatar"));
        let world = r#"{"dependencies":{"com.vrchat.worlds":{"version":"3.x"}},"locked":{}}"#;
        assert_eq!(category_from_manifest(world), Some("world"));
    }
}
