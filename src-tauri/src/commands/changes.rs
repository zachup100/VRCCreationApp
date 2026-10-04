use crate::store::ProjectStore;
use crate::vpm::VpmContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::State;
use vrc_get_vpm::environment::PackageInstaller;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::unity_project::pending_project_changes::{
    PackageChange, PendingProjectChanges, RemoveReason,
};
use vrc_get_vpm::unity_project::AddPackageOperation;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection, UnityProject, VersionSelector};

#[derive(Deserialize)]
pub struct InstallRequest {
    name: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRequest {
    #[serde(default)]
    installs: Vec<InstallRequest>,
    #[serde(default)]
    removes: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallChange {
    name: String,
    display_name: String,
    version: String,
    previous_version: Option<String>,
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveChange {
    name: String,
    reason: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictChange {
    package: String,
    conflicting_packages: Vec<String>,
    conflicts_with_unity: bool,
    unlocked_names: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSummary {
    installs: Vec<InstallChange>,
    removes: Vec<RemoveChange>,
    conflicts: Vec<ConflictChange>,
    legacy_files: Vec<String>,
    legacy_folders: Vec<String>,
}

fn summarize(project: &UnityProject, changes: &PendingProjectChanges<'_>) -> ChangeSummary {
    let mut installs = Vec::new();
    let mut removes = Vec::new();
    for (name, change) in changes.package_changes() {
        match change {
            PackageChange::Install(install) => {
                let Some(package) = install.install_package() else {
                    continue;
                };
                let previous = project
                    .get_locked(name)
                    .map(|locked| locked.version().clone());
                let kind = match &previous {
                    None => "new",
                    Some(old) if old < package.version() => "upgrade",
                    Some(old) if old > package.version() => "downgrade",
                    Some(_) => "reinstall",
                };
                installs.push(InstallChange {
                    name: name.to_string(),
                    display_name: package.display_name().unwrap_or(name).to_string(),
                    version: package.version().to_string(),
                    previous_version: previous.map(|version| version.to_string()),
                    kind,
                });
            }
            PackageChange::Remove(remove) => removes.push(RemoveChange {
                name: name.to_string(),
                reason: match remove.reason() {
                    RemoveReason::Requested => "requested",
                    RemoveReason::Legacy => "legacy",
                    RemoveReason::Unused => "unused",
                },
            }),
        }
    }
    installs.sort_by(|a, b| a.name.cmp(&b.name));
    removes.sort_by(|a, b| a.name.cmp(&b.name));

    let mut conflicts: Vec<ConflictChange> = changes
        .conflicts()
        .iter()
        .map(|(package, info)| ConflictChange {
            package: package.to_string(),
            conflicting_packages: info
                .conflicting_packages()
                .iter()
                .map(|name| name.to_string())
                .collect(),
            conflicts_with_unity: info.conflicts_with_unity(),
            unlocked_names: info
                .unlocked_names()
                .iter()
                .map(|name| name.to_string())
                .collect(),
        })
        .collect();
    conflicts.sort_by(|a, b| a.package.cmp(&b.package));

    let relative = |path: &std::path::Path| path.to_string_lossy().into_owned();
    ChangeSummary {
        installs,
        removes,
        conflicts,
        legacy_files: changes
            .remove_legacy_files()
            .iter()
            .map(|(path, _)| relative(path))
            .collect(),
        legacy_folders: changes
            .remove_legacy_folders()
            .iter()
            .map(|(path, _)| relative(path))
            .collect(),
    }
}

async fn load_project(
    path: &str,
    store: &State<'_, Mutex<ProjectStore>>,
) -> Result<UnityProject, String> {
    {
        let store = store
            .lock()
            .map_err(|_| "Project store is unavailable".to_string())?;
        if store.find(path).is_none() {
            return Err("This project is not in the list".to_string());
        }
    }
    UnityProject::load(DefaultProjectIo::new(PathBuf::from(path).into_boxed_path()))
        .await
        .map_err(|error| format!("Could not read the project: {error}"))
}

async fn plan_installs<'env>(
    project: &UnityProject,
    collection: &'env vrc_get_vpm::environment::PackageCollection,
    installs: &[InstallRequest],
) -> Result<PendingProjectChanges<'env>, String> {
    let mut packages = Vec::with_capacity(installs.len());
    for install in installs {
        let version = Version::from_str(&install.version)
            .map_err(|_| format!("Invalid version {}", install.version))?;
        let package = collection
            .find_package_by_name(&install.name, VersionSelector::specific_version(&version))
            .ok_or_else(|| format!("{} {} was not found", install.name, install.version))?;
        packages.push(package);
    }
    project
        .add_package_request(collection, &packages, AddPackageOperation::AutoDetected, false)
        .await
        .map_err(|error| error.to_string())
}

async fn plan_removes(
    project: &UnityProject,
    removes: &[String],
) -> Result<PendingProjectChanges<'static>, String> {
    let names: Vec<&str> = removes.iter().map(String::as_str).collect();
    project
        .remove_request(&names)
        .await
        .map_err(|error| error.to_string())
}

fn merge(mut first: ChangeSummary, second: ChangeSummary) -> ChangeSummary {
    first.installs.extend(second.installs);
    first.removes.extend(second.removes);
    first.conflicts.extend(second.conflicts);
    first.legacy_files.extend(second.legacy_files);
    first.legacy_folders.extend(second.legacy_folders);
    first.installs.sort_by(|a, b| a.name.cmp(&b.name));
    first.removes.sort_by(|a, b| a.name.cmp(&b.name));
    first
}

#[tauri::command]
pub async fn project_plan_changes(
    path: String,
    request: ChangeRequest,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
) -> Result<ChangeSummary, String> {
    if request.installs.is_empty() && request.removes.is_empty() {
        return Err("No package changes were selected".to_string());
    }
    let _ops = vpm.try_busy()?;
    let project = load_project(&path, &store).await?;
    let collection = vpm.collection(false).await?;

    let remove_summary = if request.removes.is_empty() {
        None
    } else {
        let changes = plan_removes(&project, &request.removes).await?;
        Some(summarize(&project, &changes))
    };
    let install_summary = if request.installs.is_empty() {
        None
    } else {
        let changes = plan_installs(&project, &collection, &request.installs).await?;
        Some(summarize(&project, &changes))
    };

    match (remove_summary, install_summary) {
        (Some(removes), Some(installs)) => Ok(merge(removes, installs)),
        (Some(summary), None) | (None, Some(summary)) => Ok(summary),
        (None, None) => Err("No package changes were selected".to_string()),
    }
}

#[tauri::command]
pub async fn project_apply_changes(
    path: String,
    request: ChangeRequest,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
) -> Result<(), String> {
    if request.installs.is_empty() && request.removes.is_empty() {
        return Err("No package changes were selected".to_string());
    }
    let _ops = vpm.try_busy()?;
    let collection = vpm.collection(false).await?;
    let installer = PackageInstaller::new(&vpm.io, Some(&vpm.http));

    if !request.removes.is_empty() {
        let mut project = load_project(&path, &store).await?;
        let changes = plan_removes(&project, &request.removes).await?;
        if !changes.conflicts().is_empty() {
            return Err("These changes have conflicts and cannot be applied".to_string());
        }
        project
            .apply_pending_changes(&installer, changes)
            .await
            .map_err(|error| format!("Could not apply the changes: {error}"))?;
    }

    if !request.installs.is_empty() {
        let mut project = load_project(&path, &store).await?;
        let changes = plan_installs(&project, &collection, &request.installs).await?;
        if !changes.conflicts().is_empty() {
            return Err("These changes have conflicts and cannot be applied".to_string());
        }
        project
            .apply_pending_changes(&installer, changes)
            .await
            .map_err(|error| format!("Could not apply the changes: {error}"))?;
    }

    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.touch(&path);
    store.save().map_err(|error| error.to_string())
}
