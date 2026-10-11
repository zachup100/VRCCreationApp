use crate::applog::{log, project_label};
use crate::commands::projects::{category_of, detect_type};
use crate::prefs::Prefs;
use crate::store::ProjectStore;
use crate::vpm::VpmContext;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use vrc_get_vpm::environment::PackageInstaller;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::{ProjectType, UnityProject, VRCHAT_RECOMMENDED_2022_UNITY};

const EVENT: &str = "project-task";
pub(crate) const BACKUP_MARKER: &str = "VRCCreationApp";
pub(crate) const LEGACY_BACKUP_MARKER: &str = "VRCreationApp";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupMeta {
    pub app: String,
    pub version: u32,
    pub project_name: String,
    pub category: String,
    pub unity_version: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Clone)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum TaskProgress {
    Phase { name: &'static str },
    Backup { done: usize, total: usize },
    Restore { done: usize, total: usize },
    Duplicate { done: usize, total: usize },
    UnityLog { line: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    path: String,
    files: usize,
    bytes: u64,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MigrationKind {
    Vpm,
    Unity2022,
}

pub(crate) fn emit(app: &AppHandle, progress: TaskProgress) {
    let _ = app.emit(EVENT, progress);
}

pub(crate) fn backup_directory(app: &AppHandle, prefs: &State<'_, Mutex<Prefs>>) -> Result<PathBuf, String> {
    let configured = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .backup_location
        .clone();
    match configured {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?
            .join("Backups")),
    }
}

pub(crate) fn safe_file_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect()
}

pub(crate) fn collect_files(
    root: &Path,
    relative: &Path,
    files: &mut Vec<(PathBuf, String)>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        let child = relative.join(&name);
        if file_type.is_dir() {
            if lower == ".git" {
                continue;
            }
            if relative.as_os_str().is_empty() && (matches!(lower.as_str(), "logs" | "obj" | "temp") || lower.starts_with("library")) {
                continue;
            }
            collect_files(root, &child, files)?;
        } else if file_type.is_file() {
            let archive_name = child
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((entry.path(), archive_name));
        }
    }
    Ok(())
}

pub(crate) fn write_backup(
    progress: &mut dyn FnMut(usize, usize),
    project: &Path,
    target: &Path,
    unity_version: Option<String>,
    category: String,
) -> Result<BackupResult, String> {
    let mut files = Vec::new();
    collect_files(project, Path::new(""), &mut files)
        .map_err(|error| format!("Could not read the project files: {error}"))?;
    let total = files.len();
    let root_name = project
        .file_name()
        .map(|name| safe_file_name(&name.to_string_lossy()))
        .unwrap_or_else(|| "project".to_string());

    let outcome = (|| -> Result<u64, String> {
        let file = File::create(target).map_err(|error| format!("Could not create the backup file: {error}"))?;
        let mut writer = zip::ZipWriter::new(std::io::BufWriter::new(file));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        let mut bytes = 0u64;
        for (index, (path, name)) in files.iter().enumerate() {
            let mut source = File::open(path).map_err(|error| format!("Could not read {name}: {error}"))?;
            writer
                .start_file(format!("{root_name}/{name}"), options)
                .map_err(|error| format!("Could not write the backup: {error}"))?;
            bytes += std::io::copy(&mut source, &mut writer).map_err(|error| format!("Could not write the backup: {error}"))?;
            if index % 25 == 0 || index + 1 == total {
                progress(index + 1, total);
            }
        }
        let meta = BackupMeta {
            app: BACKUP_MARKER.to_string(),
            version: 1,
            project_name: root_name.clone(),
            category,
            unity_version,
            created_at: crate::store::now_millis(),
        };
        writer.set_comment(serde_json::to_string(&meta).map_err(|error| error.to_string())?);
        writer
            .finish()
            .map_err(|error| format!("Could not finish the backup: {error}"))?;
        Ok(bytes)
    })();

    match outcome {
        Ok(bytes) => Ok(BackupResult {
            path: target.to_string_lossy().into_owned(),
            files: total,
            bytes,
        }),
        Err(message) => {
            let _ = std::fs::remove_file(target);
            Err(message)
        }
    }
}

async fn create_backup(
    app: &AppHandle,
    project: &Path,
    prefs: &State<'_, Mutex<Prefs>>,
) -> Result<BackupResult, String> {
    let directory = backup_directory(app, prefs)?;
    std::fs::create_dir_all(&directory).map_err(|error| format!("Could not create the backup folder: {error}"))?;
    let canonical_project = project
        .canonicalize()
        .map_err(|error| format!("Could not read the project folder: {error}"))?;
    let canonical_directory = directory
        .canonicalize()
        .map_err(|error| format!("Could not read the backup folder: {error}"))?;
    if canonical_directory.starts_with(&canonical_project) {
        return Err("The backup folder is inside the project. Choose another folder in Settings.".to_string());
    }
    let name = project
        .file_name()
        .map(|name| safe_file_name(&name.to_string_lossy()))
        .unwrap_or_else(|| "project".to_string());
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let target = directory.join(format!("{name}-{stamp}.zip"));

    let (unity_version, category) = match load_project(&project.to_string_lossy()).await {
        Ok(loaded) => (
            Some(loaded.unity_version().to_string()),
            category_of(detect_type(&loaded).await).to_string(),
        ),
        Err(_) => (None, "other".to_string()),
    };

    emit(app, TaskProgress::Phase { name: "backup" });
    let handle = app.clone();
    let source = project.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut report = |done: usize, total: usize| emit(&handle, TaskProgress::Backup { done, total });
        write_backup(&mut report, &source, &target, unity_version, category)
    })
        .await
        .map_err(|_| "The backup was interrupted".to_string())?
}

fn check_listed(path: &str, store: &State<'_, Mutex<ProjectStore>>) -> Result<(), String> {
    let store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    if store.find(path).is_none() {
        return Err("This project is not in the list".to_string());
    }
    if !Path::new(path).is_dir() {
        return Err("The project folder no longer exists".to_string());
    }
    Ok(())
}

async fn load_project(path: &str) -> Result<UnityProject, String> {
    UnityProject::load(DefaultProjectIo::new(PathBuf::from(path).into_boxed_path()))
        .await
        .map_err(|error| format!("Could not read the project: {error}"))
}

#[tauri::command]
pub async fn backup_project(
    app: AppHandle,
    path: String,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<BackupResult, String> {
    let _busy = vpm.try_busy()?;
    check_listed(&path, &store)?;
    let result = create_backup(&app, Path::new(&path), &prefs).await?;
    log(&app, "success", format!("Backed up {} to {}", project_label(&path), result.path));
    Ok(result)
}

#[tauri::command]
pub async fn open_backup_folder(app: AppHandle, prefs: State<'_, Mutex<Prefs>>) -> Result<(), String> {
    let directory = backup_directory(&app, &prefs)?;
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    tokio::task::spawn_blocking(move || open::that_detached(directory))
        .await
        .map_err(|_| "Could not open the folder".to_string())?
        .map_err(|error| error.to_string())
}

fn run_unity_upgrade(app: &AppHandle, executable: PathBuf, project: &Path) -> Result<(), String> {
    let mut child = Command::new(executable)
        .args(["-quit", "-batchmode", "-ignorecompilererrors", "-logFile", "-", "-projectPath"])
        .arg(project)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start Unity: {error}"))?;
    let mut tail: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            if !line.is_empty() {
                if tail.len() >= 25 {
                    tail.pop_front();
                }
                tail.push_back(line.clone());
                emit(app, TaskProgress::UnityLog { line });
            }
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("Could not wait for Unity: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        let lines: Vec<String> = tail.into_iter().collect();
        log(app, "error", format!("Unity exited with {status}. Last output:\n{}", lines.join("\n")));
        Err(format!("Unity exited with {status}"))
    }
}

#[tauri::command]
pub async fn migrate_project(
    app: AppHandle,
    path: String,
    kind: MigrationKind,
    backup: bool,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<Option<BackupResult>, String> {
    let _busy = vpm.try_busy()?;
    check_listed(&path, &store)?;
    let allow_prerelease = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .show_prerelease;

    let mut project = load_project(&path).await?;
    let unity_executable = match kind {
        MigrationKind::Vpm => {
            match project.detect_project_type().await {
                ProjectType::LegacyWorlds | ProjectType::LegacyAvatars => {}
                _ => return Err("Only legacy VRChat SDK3 projects can be migrated automatically. SDK2 projects have to be upgraded by hand.".to_string()),
            }
            None
        }
        MigrationKind::Unity2022 => {
            if project.unity_version().major() != 2019 {
                return Err("Only Unity 2019 projects can be migrated to Unity 2022".to_string());
            }
            let wanted = VRCHAT_RECOMMENDED_2022_UNITY.to_string();
            let executable = crate::editors::executable_for(&prefs, &wanted)
                .await
                .map_err(|_| format!("Unity {wanted} is not installed. Install it first, then try again."))?;
            Some(executable)
        }
    };

    let backup_result = if backup {
        Some(create_backup(&app, Path::new(&path), &prefs).await?)
    } else {
        None
    };

    emit(&app, TaskProgress::Phase { name: "packages" });
    let collection = vpm.collection(false).await?;
    let http = vpm.progress_http(app.clone());
    let installer = PackageInstaller::new(&vpm.io, Some(&http));
    match kind {
        MigrationKind::Vpm => project
            .migrate_vpm(&*collection, &installer, allow_prerelease)
            .await
            .map_err(|error| format!("Could not migrate the project: {error}"))?,
        MigrationKind::Unity2022 => project
            .migrate_unity_2022(&*collection, &installer)
            .await
            .map_err(|error| format!("Could not migrate the packages: {error}"))?,
    }

    if let Some(executable) = unity_executable {
        emit(&app, TaskProgress::Phase { name: "unity" });
        let handle = app.clone();
        let project_path = PathBuf::from(&path);
        tokio::task::spawn_blocking(move || run_unity_upgrade(&handle, executable, &project_path))
            .await
            .map_err(|_| "The Unity upgrade was interrupted".to_string())??;
    }

    log(
        &app,
        "success",
        format!(
            "Migrated {} ({})",
            project_label(&path),
            if kind == MigrationKind::Vpm { "VPM package system" } else { "Unity 2022" }
        ),
    );
    let refreshed = load_project(&path).await?;
    let category = category_of(detect_type(&refreshed).await).to_string();
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.update_info(&path, Some(refreshed.unity_version().to_string()), category);
    store.save().map_err(|error| error.to_string())?;
    Ok(backup_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn sample_project(root: &Path) {
        write(root, "Assets/a.txt", "asset");
        write(root, "Assets/Library/keep.txt", "nested library folders are kept");
        write(root, "Packages/vpm-manifest.json", "{}");
        write(root, "ProjectSettings/ProjectVersion.txt", "m_EditorVersion: 2022.3.22f1\n");
        write(root, "Project.sln", "solution");
        write(root, "Library/cache.bin", "excluded");
        write(root, "LibraryBackup/cache.bin", "excluded");
        write(root, "Temp/t.txt", "excluded");
        write(root, "Logs/l.txt", "excluded");
        write(root, "obj/o.txt", "excluded");
        write(root, ".git/config", "excluded");
        write(root, "Assets/.git/config", "excluded");
    }

    fn collected(root: &Path) -> Vec<String> {
        let mut files = Vec::new();
        collect_files(root, Path::new(""), &mut files).unwrap();
        let mut names: Vec<String> = files.into_iter().map(|(_, name)| name).collect();
        names.sort();
        names
    }

    #[test]
    fn safe_file_name_replaces_reserved_characters() {
        assert_eq!(safe_file_name("a:b/c*d"), "a_b_c_d");
        assert_eq!(safe_file_name("Plain Name"), "Plain Name");
    }

    #[test]
    fn caches_and_version_control_are_left_out_of_backups() {
        let dir = tempfile::tempdir().unwrap();
        sample_project(dir.path());
        assert_eq!(
            collected(dir.path()),
            vec![
                "Assets/Library/keep.txt",
                "Assets/a.txt",
                "Packages/vpm-manifest.json",
                "Project.sln",
                "ProjectSettings/ProjectVersion.txt",
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Assets/a.txt", "asset");
        std::os::unix::fs::symlink(dir.path().join("Assets/a.txt"), dir.path().join("Assets/link.txt")).unwrap();
        assert_eq!(collected(dir.path()), vec!["Assets/a.txt"]);
    }

    #[test]
    fn a_backup_contains_the_project_under_its_folder_name_with_a_marker() {
        let parent = tempfile::tempdir().unwrap();
        let project = parent.path().join("My Project");
        sample_project(&project);
        let target = parent.path().join("backup.zip");
        let mut reports = Vec::new();
        let mut report = |done: usize, total: usize| reports.push((done, total));
        let result = write_backup(&mut report, &project, &target, Some("2022.3.22f1".to_string()), "world".to_string()).unwrap();
        assert_eq!(result.files, 5);
        assert!(result.bytes > 0);
        assert_eq!(reports.last().copied(), Some((5, 5)));

        let mut archive = zip::ZipArchive::new(std::io::BufReader::new(File::open(&target).unwrap())).unwrap();
        let meta: BackupMeta = serde_json::from_slice(archive.comment()).unwrap();
        assert_eq!(meta.app, BACKUP_MARKER);
        assert_eq!(meta.project_name, "My Project");
        assert_eq!(meta.category, "world");
        assert_eq!(meta.unity_version.as_deref(), Some("2022.3.22f1"));
        assert!(archive.by_name("My Project/Assets/a.txt").is_ok());
        assert!(archive.by_name("My Project/Library/cache.bin").is_err());
    }

    #[test]
    fn a_failed_backup_leaves_no_partial_file() {
        let parent = tempfile::tempdir().unwrap();
        let missing = parent.path().join("does-not-exist");
        let target = parent.path().join("backup.zip");
        let mut report = |_: usize, _: usize| {};
        assert!(write_backup(&mut report, &missing, &target, None, "other".to_string()).is_err());
        assert!(!target.exists());
    }
}