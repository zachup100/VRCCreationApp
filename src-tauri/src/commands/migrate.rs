use crate::commands::projects::category_of;
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
use vrc_get_vpm::unity_hub;
use vrc_get_vpm::{ProjectType, UnityProject, VRCHAT_RECOMMENDED_2022_UNITY};

const EVENT: &str = "project-task";

#[derive(Serialize, Clone)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum TaskProgress {
    Phase { name: &'static str },
    Backup { done: usize, total: usize },
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

fn emit(app: &AppHandle, progress: TaskProgress) {
    let _ = app.emit(EVENT, progress);
}

fn backup_directory(app: &AppHandle, prefs: &State<'_, Mutex<Prefs>>) -> Result<PathBuf, String> {
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

fn safe_file_name(name: &str) -> String {
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

fn collect_files(
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

fn write_backup(app: &AppHandle, project: &Path, target: &Path) -> Result<BackupResult, String> {
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
                emit(app, TaskProgress::Backup { done: index + 1, total });
            }
        }
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

    emit(app, TaskProgress::Phase { name: "backup" });
    let handle = app.clone();
    let source = project.to_path_buf();
    tokio::task::spawn_blocking(move || write_backup(&handle, &source, &target))
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
    create_backup(&app, Path::new(&path), &prefs).await
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
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            if !line.is_empty() {
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
            let editors = unity_hub::load_unity_by_loading_unity_hub_files()
                .await
                .map_err(|error| error.to_string())?;
            let editor = editors
                .iter()
                .find(|editor| editor.version.to_string() == wanted)
                .ok_or_else(|| format!("Unity {wanted} is not installed. Install it first, then try again."))?;
            Some(unity_hub::get_executable_path(&editor.path).into_owned())
        }
    };

    let backup_result = if backup {
        Some(create_backup(&app, Path::new(&path), &prefs).await?)
    } else {
        None
    };

    emit(&app, TaskProgress::Phase { name: "packages" });
    let collection = vpm.collection(false).await?;
    let installer = PackageInstaller::new(&vpm.io, Some(&vpm.http));
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

    let refreshed = load_project(&path).await?;
    let category = category_of(refreshed.detect_project_type().await).to_string();
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.update_info(&path, Some(refreshed.unity_version().to_string()), category);
    store.save().map_err(|error| error.to_string())?;
    Ok(backup_result)
}
