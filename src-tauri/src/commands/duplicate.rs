use crate::applog::{log, project_label};
use crate::commands::create::validate_name;
use crate::commands::migrate::{emit, TaskProgress};
use crate::commands::projects::{to_info, ProjectInfo};
use crate::store::{now_millis, ProjectStore, StoredProject};
use crate::vpm::VpmContext;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};

pub(crate) struct CopyEntry {
    relative: PathBuf,
    is_dir: bool,
}

pub(crate) fn collect_entries(
    root: &Path,
    relative: &Path,
    include_library: bool,
    entries: &mut Vec<CopyEntry>,
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
            if relative.as_os_str().is_empty()
                && (lower == "temp" || (!include_library && lower.starts_with("library")))
            {
                continue;
            }
            entries.push(CopyEntry {
                relative: child.clone(),
                is_dir: true,
            });
            collect_entries(root, &child, include_library, entries)?;
        } else if file_type.is_file() {
            entries.push(CopyEntry {
                relative: child,
                is_dir: false,
            });
        }
    }
    Ok(())
}

pub(crate) fn copy_project(
    source: &Path,
    target: &Path,
    include_library: bool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<usize, String> {
    let mut entries = Vec::new();
    collect_entries(source, Path::new(""), include_library, &mut entries)
        .map_err(|error| format!("Could not read the project files: {error}"))?;
    let total = entries.len();
    for (index, entry) in entries.iter().enumerate() {
        let from = source.join(&entry.relative);
        let to = target.join(&entry.relative);
        if entry.is_dir {
            std::fs::create_dir_all(&to).map_err(|error| format!("Could not create a folder in the copy: {error}"))?;
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|error| format!("Could not create a folder in the copy: {error}"))?;
            }
            std::fs::copy(&from, &to).map_err(|error| format!("Could not copy {}: {error}", entry.relative.display()))?;
            if let Ok(modified) = std::fs::metadata(&from).and_then(|meta| meta.modified()) {
                if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&to) {
                    let _ = file.set_modified(modified);
                }
            }
        }
        if index % 50 == 0 || index + 1 == total {
            progress(index + 1, total);
        }
    }
    Ok(total)
}

#[tauri::command]
pub async fn duplicate_project(
    app: AppHandle,
    path: String,
    name: String,
    include_library: bool,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
) -> Result<ProjectInfo, String> {
    let _busy = vpm.try_busy()?;
    let original = {
        let store = store
            .lock()
            .map_err(|_| "Project store is unavailable".to_string())?;
        store
            .find(&path)
            .cloned()
            .ok_or_else(|| "This project is not in the list".to_string())?
    };
    let source = PathBuf::from(&original.path);
    if !source.is_dir() {
        return Err("The project folder no longer exists".to_string());
    }
    let name = name.trim().to_string();
    validate_name(&name)?;
    let parent = source
        .parent()
        .ok_or_else(|| "This project has no parent folder to copy into".to_string())?
        .to_path_buf();
    let target = parent.join(&name);
    match tokio::fs::create_dir(&target).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(format!("A folder named \"{name}\" already exists next to the original"));
        }
        Err(error) => return Err(format!("Could not create the project folder: {error}")),
    }

    let copied = {
        let handle = app.clone();
        let from = source.clone();
        let to = target.clone();
        tokio::task::spawn_blocking(move || {
            let mut report = |done: usize, total: usize| emit(&handle, TaskProgress::Duplicate { done, total });
            copy_project(&from, &to, include_library, &mut report)
        })
        .await
        .map_err(|_| "The copy was interrupted".to_string())
        .and_then(|result| result)
    };
    if let Err(message) = copied {
        let _ = tokio::fs::remove_dir_all(&target).await;
        return Err(message);
    }

    let stored = StoredProject {
        path: target.to_string_lossy().into_owned(),
        unity_version: original.unity_version.clone(),
        category: original.category.clone(),
        favorite: false,
        added_at: now_millis(),
        last_modified: now_millis(),
        launch_args: original.launch_args.clone(),
    };
    log(&app, "success", format!("Duplicated {} as {}", project_label(&original.path), name));
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.insert(stored.clone());
    store.save().map_err(|error| error.to_string())?;
    Ok(to_info(&stored))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn sample(root: &Path) {
        write(root, "Assets/a.txt", "asset");
        write(root, "Assets/deep/er/b.txt", "deep");
        write(root, "Packages/vpm-manifest.json", "{}");
        write(root, "ProjectSettings/ProjectVersion.txt", "m_EditorVersion: 2022.3.22f1\n");
        write(root, "Library/cache.bin", "cache");
        write(root, "Library/sub/more.bin", "more");
        write(root, "Temp/UnityLockFile", "lock");
        write(root, "Logs/l.txt", "log");
        write(root, ".git/config", "git");
        std::fs::create_dir_all(root.join("Assets/EmptyFolder")).unwrap();
    }

    #[test]
    fn a_full_copy_keeps_everything_except_temp() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Original");
        let target = dir.path().join("Copy");
        sample(&source);
        std::fs::create_dir(&target).unwrap();
        let mut calls = Vec::new();
        let mut report = |done: usize, total: usize| calls.push((done, total));
        let total = copy_project(&source, &target, true, &mut report).unwrap();
        assert!(total > 0);
        assert_eq!(calls.last().map(|call| call.0), Some(total));
        assert_eq!(std::fs::read_to_string(target.join("Assets/deep/er/b.txt")).unwrap(), "deep");
        assert_eq!(std::fs::read_to_string(target.join("Library/sub/more.bin")).unwrap(), "more");
        assert!(target.join("Assets/EmptyFolder").is_dir());
        assert!(target.join(".git/config").is_file());
        assert!(target.join("Logs/l.txt").is_file());
        assert!(!target.join("Temp").exists());
    }

    #[test]
    fn the_library_folder_can_be_left_out() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Original");
        let target = dir.path().join("Copy");
        sample(&source);
        write(&source, "LibraryBackup/x.bin", "x");
        std::fs::create_dir(&target).unwrap();
        let mut report = |_: usize, _: usize| {};
        copy_project(&source, &target, false, &mut report).unwrap();
        assert!(!target.join("Library").exists());
        assert!(!target.join("LibraryBackup").exists());
        assert!(target.join("Assets/a.txt").is_file());
        assert!(!target.join("Temp").exists());
    }

    #[test]
    fn the_original_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Original");
        let target = dir.path().join("Copy");
        sample(&source);
        std::fs::create_dir(&target).unwrap();
        let mut report = |_: usize, _: usize| {};
        copy_project(&source, &target, true, &mut report).unwrap();
        assert!(source.join("Temp/UnityLockFile").is_file());
        assert!(source.join("Library/cache.bin").is_file());
        assert_eq!(std::fs::read_to_string(source.join("Assets/a.txt")).unwrap(), "asset");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed_or_copied() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Original");
        let target = dir.path().join("Copy");
        write(&source, "Assets/a.txt", "asset");
        std::os::unix::fs::symlink(source.join("Assets/a.txt"), source.join("Assets/link.txt")).unwrap();
        std::fs::create_dir(&target).unwrap();
        let mut report = |_: usize, _: usize| {};
        copy_project(&source, &target, true, &mut report).unwrap();
        assert!(target.join("Assets/a.txt").is_file());
        assert!(!target.join("Assets/link.txt").exists());
    }

    #[test]
    fn a_missing_source_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut report = |_: usize, _: usize| {};
        assert!(copy_project(&dir.path().join("nope"), &dir.path().join("out"), true, &mut report).is_err());
    }
}
