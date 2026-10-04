use crate::applog::{log, project_label};
use crate::commands::packages::redundant_dependencies;
use crate::commands::projects::{category_of, detect_type};
use crate::download::ProgressHttp;
use crate::prefs::Prefs;
use crate::store::ProjectStore;
use crate::vpm::VpmContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::{AppHandle, State};
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
    #[serde(default)]
    resolve: bool,
    #[serde(default)]
    reinstall: Vec<String>,
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
    allow_prerelease: bool,
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
    let mut provided: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for manifest in project.all_installed_packages() {
        let provider = manifest.display_name().unwrap_or(manifest.name()).to_string();
        for legacy in manifest.legacy_packages() {
            provided.insert(legacy.to_string(), provider.clone());
        }
    }
    for package in &packages {
        let provider = package.display_name().unwrap_or(package.name()).to_string();
        for legacy in package.legacy_packages() {
            provided.entry(legacy.to_string()).or_insert_with(|| provider.clone());
        }
    }
    for package in &packages {
        if let Some(provider) = provided.get(package.name()) {
            let label = package.display_name().unwrap_or(package.name());
            return Err(format!(
                "{label} is already included in {provider} and does not need to be installed"
            ));
        }
    }
    project
        .add_package_request(collection, &packages, AddPackageOperation::AutoDetected, allow_prerelease)
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

impl ChangeRequest {
    fn is_empty(&self) -> bool {
        self.installs.is_empty() && self.removes.is_empty() && !self.resolve && self.reinstall.is_empty()
    }

    fn is_repair(&self) -> bool {
        self.resolve || !self.reinstall.is_empty()
    }

    fn validate(&self) -> Result<(), String> {
        if self.is_empty() {
            return Err("No package changes were selected".to_string());
        }
        if self.is_repair() && (!self.installs.is_empty() || !self.removes.is_empty() || (self.resolve && !self.reinstall.is_empty())) {
            return Err("Repairing packages cannot be combined with other changes".to_string());
        }
        Ok(())
    }
}

async fn plan_repair<'env>(
    project: &UnityProject,
    collection: &'env vrc_get_vpm::environment::PackageCollection,
    request: &ChangeRequest,
) -> Result<PendingProjectChanges<'env>, String> {
    if request.resolve {
        return project
            .resolve_request(collection)
            .await
            .map_err(|error| error.to_string());
    }
    let names: Vec<&str> = request.reinstall.iter().map(String::as_str).collect();
    project
        .reinstall_request(collection, &names)
        .await
        .map_err(|error| error.to_string())
}

async fn refresh_details(path: &str, store: &State<'_, Mutex<ProjectStore>>) {
    let Ok(project) = UnityProject::load(DefaultProjectIo::new(PathBuf::from(path).into_boxed_path())).await else {
        return;
    };
    let category = category_of(detect_type(&project).await).to_string();
    let version = project.unity_version().to_string();
    if let Ok(mut store) = store.lock() {
        store.update_details(path, Some(version), category);
        let _ = store.save();
    }
}

fn describe(request: &ChangeRequest) -> String {
    if request.resolve {
        return "resolved the project's packages".to_string();
    }
    if !request.reinstall.is_empty() {
        return format!("reinstalled {}", request.reinstall.join(", "));
    }
    let mut parts = Vec::new();
    if !request.removes.is_empty() {
        parts.push(format!("removed {}", request.removes.join(", ")));
    }
    if !request.installs.is_empty() {
        let list: Vec<String> = request
            .installs
            .iter()
            .map(|item| format!("{} {}", item.name, item.version))
            .collect();
        parts.push(format!("installed {}", list.join(", ")));
    }
    parts.join("; ")
}

fn prerelease_enabled(prefs: &State<'_, Mutex<Prefs>>) -> Result<bool, String> {
    Ok(prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .show_prerelease)
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
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<ChangeSummary, String> {
    let allow_prerelease = prerelease_enabled(&prefs)?;
    request.validate()?;
    let _ops = vpm.try_busy()?;
    let project = load_project(&path, &store).await?;
    let collection = vpm.collection(false).await?;

    if request.is_repair() {
        let changes = plan_repair(&project, &collection, &request).await?;
        return Ok(summarize(&project, &changes));
    }

    let remove_summary = if request.removes.is_empty() {
        None
    } else {
        let changes = plan_removes(&project, &request.removes).await?;
        Some(summarize(&project, &changes))
    };
    let install_summary = if request.installs.is_empty() {
        None
    } else {
        let changes = plan_installs(&project, &collection, &request.installs, allow_prerelease).await?;
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
    app: AppHandle,
    path: String,
    request: ChangeRequest,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let allow_prerelease = prerelease_enabled(&prefs)?;
    request.validate()?;
    let _ops = vpm.try_busy()?;
    let collection = vpm.collection(false).await?;
    let http = ProgressHttp::new(vpm.http.clone(), app.clone());
    let installer = PackageInstaller::new(&vpm.io, Some(&http));

    if request.is_repair() {
        let mut project = load_project(&path, &store).await?;
        let changes = plan_repair(&project, &collection, &request).await?;
        if !changes.conflicts().is_empty() {
            return Err("These changes have conflicts and cannot be applied".to_string());
        }
        project
            .apply_pending_changes(&installer, changes)
            .await
            .map_err(|error| format!("Could not apply the changes: {error}"))?;
        log(&app, "success", format!("{}: {}", project_label(&path), describe(&request)));
        refresh_details(&path, &store).await;
        let mut store = store
            .lock()
            .map_err(|_| "Project store is unavailable".to_string())?;
        store.touch(&path);
        return store.save().map_err(|error| error.to_string());
    }

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
        let changes = plan_installs(&project, &collection, &request.installs, allow_prerelease).await?;
        if !changes.conflicts().is_empty() {
            return Err("These changes have conflicts and cannot be applied".to_string());
        }
        project
            .apply_pending_changes(&installer, changes)
            .await
            .map_err(|error| format!("Could not apply the changes: {error}"))?;
    }

    log(&app, "success", format!("{}: {}", project_label(&path), describe(&request)));
    refresh_details(&path, &store).await;
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.touch(&path);
    store.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn project_cleanup_manifest(
    app: AppHandle,
    path: String,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
) -> Result<Vec<String>, String> {
    let _ops = vpm.try_busy()?;
    let project = load_project(&path, &store).await?;
    let names = redundant_dependencies(&project);
    if names.is_empty() {
        return Ok(names);
    }
    let manifest_path = std::path::Path::new(&path).join("Packages").join("vpm-manifest.json");
    let text = tokio::fs::read_to_string(&manifest_path)
        .await
        .map_err(|error| format!("Could not read vpm-manifest.json: {error}"))?;
    let updated = crate::manifest::remove_dependency_entries(&text, &names)?;
    let temp = manifest_path.with_extension("json.tmp");
    tokio::fs::write(&temp, updated)
        .await
        .map_err(|error| format!("Could not write vpm-manifest.json: {error}"))?;
    tokio::fs::rename(&temp, &manifest_path)
        .await
        .map_err(|error| format!("Could not update vpm-manifest.json: {error}"))?;
    log(
        &app,
        "success",
        format!("{}: removed redundant entries {}", project_label(&path), names.join(", ")),
    );
    refresh_details(&path, &store).await;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(json: &str) -> ChangeRequest {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn an_empty_request_is_rejected() {
        assert!(request(r#"{"installs":[],"removes":[]}"#).validate().is_err());
        assert!(request("{}").validate().is_err());
    }

    #[test]
    fn installs_and_removes_may_be_combined() {
        let mixed = request(r#"{"installs":[{"name":"a","version":"1.0.0"}],"removes":["b"]}"#);
        assert!(mixed.validate().is_ok());
        assert!(!mixed.is_repair());
    }

    #[test]
    fn repairs_cannot_be_combined_with_other_changes() {
        assert!(request(r#"{"resolve":true}"#).validate().is_ok());
        assert!(request(r#"{"reinstall":["a"]}"#).validate().is_ok());
        assert!(request(r#"{"resolve":true}"#).is_repair());
        assert!(request(r#"{"resolve":true,"installs":[{"name":"a","version":"1.0.0"}]}"#).validate().is_err());
        assert!(request(r#"{"reinstall":["a"],"removes":["b"]}"#).validate().is_err());
        assert!(request(r#"{"resolve":true,"reinstall":["a"]}"#).validate().is_err());
    }

    #[test]
    fn descriptions_name_what_changed() {
        assert_eq!(describe(&request(r#"{"resolve":true}"#)), "resolved the project's packages");
        assert_eq!(describe(&request(r#"{"reinstall":["a","b"]}"#)), "reinstalled a, b");
        assert_eq!(
            describe(&request(r#"{"installs":[{"name":"a","version":"1.0.0"}],"removes":["b","c"]}"#)),
            "removed b, c; installed a 1.0.0"
        );
    }
}