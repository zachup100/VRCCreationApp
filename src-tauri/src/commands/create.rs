use crate::commands::projects::{category_of, to_info, ProjectInfo};
use crate::applog::log;
use crate::download::ProgressHttp;
use crate::prefs::Prefs;
use crate::presets::{Preset, PresetStore};
use crate::store::{now_millis, ProjectStore, StoredProject};
use crate::vpm::VpmContext;
use include_dir::{include_dir, Dir, DirEntry};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use vrc_get_vpm::environment::PackageInstaller;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::unity_project::AddPackageOperation;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection, UnityProject, VersionSelector};
use std::str::FromStr;

static TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/project-templates");

struct Template {
    id: &'static str,
    folder: &'static str,
    title: &'static str,
    blurb: &'static str,
    kind: &'static str,
    year: &'static str,
    unity_version: &'static str,
}

const TEMPLATE_LIST: &[Template] = &[
    Template {
        id: "world-2022",
        folder: "worlds-2022.3.22f1",
        title: "Unity 2022 World Project",
        blurb: "A good starting point for a new VRChat World",
        kind: "world",
        year: "2022",
        unity_version: "2022.3.22f1",
    },
    Template {
        id: "avatar-2022",
        folder: "avatars-2022.3.22f1",
        title: "Unity 2022 Avatar Project",
        blurb: "A good starting point for a new VRChat Avatar",
        kind: "avatar",
        year: "2022",
        unity_version: "2022.3.22f1",
    },
    Template {
        id: "avatar-2019",
        folder: "avatars-2019.4.31f1",
        title: "Unity 2019 Avatar Project",
        blurb: "A good starting point for a new VRChat Avatar",
        kind: "avatar",
        year: "2019",
        unity_version: "2019.4.31f1",
    },
    Template {
        id: "world-2019",
        folder: "worlds-2019.4.31f1",
        title: "Unity 2019 World Project",
        blurb: "A good starting point for a new VRChat World",
        kind: "world",
        year: "2019",
        unity_version: "2019.4.31f1",
    },
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    id: &'static str,
    title: &'static str,
    blurb: &'static str,
    kind: &'static str,
    year: &'static str,
    unity_version: &'static str,
}

#[tauri::command]
pub fn list_templates() -> Vec<TemplateInfo> {
    TEMPLATE_LIST
        .iter()
        .map(|template| TemplateInfo {
            id: template.id,
            title: template.title,
            blurb: template.blurb,
            kind: template.kind,
            year: template.year,
            unity_version: template.unity_version,
        })
        .collect()
}

pub(crate) fn validate_name(name: &str) -> Result<(), String> {
    let invalid = |c: char| c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|');
    if name.is_empty() {
        return Err("Enter a project name".to_string());
    }
    if name.chars().any(invalid) {
        return Err("The project name contains characters that are not allowed".to_string());
    }
    if name == "." || name == ".." || name.ends_with('.') || name.ends_with(' ') {
        return Err("The project name cannot end with a dot or a space".to_string());
    }
    if name.chars().count() > 100 {
        return Err("The project name is too long".to_string());
    }
    Ok(())
}

fn write_dir(dir: &Dir<'_>, root: &Path, destination: &Path) -> std::io::Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(child) => write_dir(child, root, destination)?,
            DirEntry::File(file) => {
                let relative = file
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
                let target = destination.join(relative);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(target, file.contents())?;
            }
        }
    }
    Ok(())
}

fn replace_value(text: &mut String, key: &str, value: &str) {
    let Some(position) = text.find(key) else {
        return;
    };
    let at_line_start = text[..position]
        .chars()
        .last()
        .map(|c| c.is_ascii_whitespace())
        .unwrap_or(true);
    if !at_line_start {
        return;
    }
    if let Some(line_end) = text[position..].find('\n') {
        text.replace_range(position + key.len()..position + line_end, value);
    }
}

fn yaml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn patch_project_settings(project: &Path, name: &str) -> std::io::Result<()> {
    let file = project.join("ProjectSettings/ProjectSettings.asset");
    let Ok(mut text) = std::fs::read_to_string(&file) else {
        return Ok(());
    };
    replace_value(&mut text, "productGUID: ", &uuid::Uuid::new_v4().simple().to_string());
    replace_value(&mut text, "productName: ", &yaml_quote(name));
    std::fs::write(file, text)
}

async fn build_project(
    template: &'static Template,
    name: &str,
    target: &Path,
    vpm: &VpmContext,
    http: &ProgressHttp,
) -> Result<UnityProject, String> {
    let folder = template.folder;
    let destination = target.to_path_buf();
    let project_name = name.to_string();
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        let dir = TEMPLATES.get_dir(folder).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "template is missing")
        })?;
        write_dir(dir, dir.path(), &destination)?;
        patch_project_settings(&destination, &project_name)
    })
    .await
    .map_err(|_| "Creating the project was interrupted".to_string())?
    .map_err(|error| format!("Could not write the project files: {error}"))?;

    let mut project = UnityProject::load(DefaultProjectIo::new(target.to_path_buf().into_boxed_path()))
        .await
        .map_err(|error| format!("Could not read the new project: {error}"))?;
    let collection = vpm.collection(false).await?;
    let changes = project
        .resolve_request(&*collection)
        .await
        .map_err(|error| format!("Could not resolve the VRChat packages: {error}"))?;
    if !changes.conflicts().is_empty() {
        return Err("The VRChat packages conflict with the project template".to_string());
    }
    let installer = PackageInstaller::new(&vpm.io, Some(http));
    project
        .apply_pending_changes(&installer, changes)
        .await
        .map_err(|error| format!("Could not install the VRChat packages: {error}"))?;
    Ok(project)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedProject {
    project: ProjectInfo,
    warnings: Vec<String>,
    notes: Vec<String>,
}

#[derive(Default)]
struct PresetOutcome {
    warnings: Vec<String>,
    notes: Vec<String>,
}

fn foreign_sdk(kind: &str) -> &'static str {
    if kind == "world" {
        "com.vrchat.avatars"
    } else {
        "com.vrchat.worlds"
    }
}

async fn apply_preset(
    project: &mut UnityProject,
    preset: &Preset,
    allow_prerelease: bool,
    vpm: &VpmContext,
    http: &ProgressHttp,
) -> PresetOutcome {
    let mut outcome = PresetOutcome::default();
    let collection = match vpm.collection(false).await {
        Ok(collection) => collection,
        Err(message) => {
            outcome
                .warnings
                .push(format!("Preset packages were not installed: {message}"));
            return outcome;
        }
    };
    let latest = VersionSelector::latest_for(Some(project.unity_version()), allow_prerelease);

    let mut candidates = Vec::new();
    for package in &preset.packages {
        let found = match &package.version {
            Some(text) => match Version::from_str(text) {
                Ok(version) => collection
                    .find_package_by_name(&package.name, VersionSelector::specific_version(&version)),
                Err(_) => None,
            },
            None => collection.find_package_by_name(&package.name, latest),
        };
        match found {
            Some(info) => candidates.push(info),
            None => outcome.warnings.push(match &package.version {
                Some(version) => format!("{} {} was not found", package.name, version),
                None => format!("{} has no version for this project", package.name),
            }),
        }
    }

    let mut provided: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for manifest in project.all_installed_packages() {
        let provider = manifest.display_name().unwrap_or(manifest.name()).to_string();
        for legacy in manifest.legacy_packages() {
            provided.insert(legacy.to_string(), provider.clone());
        }
    }
    for info in &candidates {
        let provider = info.display_name().unwrap_or(info.name()).to_string();
        for legacy in info.legacy_packages() {
            provided.entry(legacy.to_string()).or_insert_with(|| provider.clone());
        }
    }

    let other_sdk = foreign_sdk(&preset.kind);
    let mut wanted = Vec::new();
    for info in candidates {
        let label = info.display_name().unwrap_or(info.name()).to_string();
        if let Some(provider) = provided.get(info.name()) {
            outcome
                .notes
                .push(format!("{label} is already included in {provider}"));
            continue;
        }
        if info.name() == other_sdk || info.vpm_dependencies().contains_key(other_sdk) {
            let sdk = if preset.kind == "world" { "Avatars" } else { "Worlds" };
            outcome.warnings.push(format!(
                "{label} needs the VRChat {sdk} SDK, so it was not installed in this {} project",
                preset.kind
            ));
            continue;
        }
        let already = project
            .get_locked(info.name())
            .map(|locked| locked.version() == info.version())
            .unwrap_or(false);
        if !already {
            wanted.push(info);
        }
    }
    if wanted.is_empty() {
        return outcome;
    }

    let changes = match project
        .add_package_request(&*collection, &wanted, AddPackageOperation::AutoDetected, allow_prerelease)
        .await
    {
        Ok(changes) => changes,
        Err(error) => {
            outcome
                .warnings
                .push(format!("Preset packages were not installed: {error}"));
            return outcome;
        }
    };
    if !changes.conflicts().is_empty() {
        outcome
            .warnings
            .push("Preset packages were not installed because they conflict with the project".to_string());
        return outcome;
    }
    let pulls_other_sdk = changes.package_changes().iter().any(|(name, change)| {
        name.as_ref() == other_sdk
            && change
                .as_install()
                .map(|install| install.install_package().is_some())
                .unwrap_or(false)
    });
    if pulls_other_sdk {
        outcome.warnings.push(format!(
            "Preset packages were not installed because they would add the other VRChat SDK to this {} project",
            preset.kind
        ));
        return outcome;
    }
    let installer = PackageInstaller::new(&vpm.io, Some(http));
    if let Err(error) = project.apply_pending_changes(&installer, changes).await {
        outcome
            .warnings
            .push(format!("Preset packages could not be installed: {error}"));
    }
    outcome
}

#[tauri::command]
pub async fn create_project(
    app: AppHandle,
    template: String,
    name: String,
    location: String,
    preset_id: Option<String>,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
    presets: State<'_, Mutex<PresetStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<CreatedProject, String> {
    let _busy = vpm.try_busy()?;
    let template = TEMPLATE_LIST
        .iter()
        .find(|item| item.id == template)
        .ok_or_else(|| "Unknown project template".to_string())?;
    let name = name.trim().to_string();
    validate_name(&name)?;
    let preset = match preset_id {
        Some(id) => {
            let found = presets
                .lock()
                .map_err(|_| "Presets are unavailable".to_string())?
                .find(&id)
                .cloned()
                .ok_or_else(|| "This preset no longer exists".to_string())?;
            if found.kind != template.kind {
                return Err("This preset is for a different template type".to_string());
            }
            Some(found)
        }
        None => None,
    };
    let allow_prerelease = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .show_prerelease;

    let parent = PathBuf::from(location.trim());
    if !parent.is_absolute() || !parent.is_dir() {
        return Err("The project location is not an existing folder".to_string());
    }
    let target = parent.join(&name);
    match tokio::fs::create_dir(&target).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(format!("A folder named \"{name}\" already exists in that location"));
        }
        Err(error) => return Err(format!("Could not create the project folder: {error}")),
    }

    let http = ProgressHttp::new(vpm.http.clone(), app.clone());
    let mut project = match build_project(template, &name, &target, &vpm, &http).await {
        Ok(project) => project,
        Err(message) => {
            let _ = tokio::fs::remove_dir_all(&target).await;
            return Err(message);
        }
    };

    let outcome = match &preset {
        Some(preset) => apply_preset(&mut project, preset, allow_prerelease, &vpm, &http).await,
        None => PresetOutcome::default(),
    };

    let stored = StoredProject {
        path: target.to_string_lossy().into_owned(),
        unity_version: Some(project.unity_version().to_string()),
        category: category_of(project.detect_project_type().await).to_string(),
        favorite: false,
        added_at: now_millis(),
        last_modified: now_millis(),
        launch_args: None,
    };
    log(&app, "success", format!("Created project {name} from {}", template.title));
    for note in &outcome.notes {
        log(&app, "info", note.clone());
    }
    for warning in &outcome.warnings {
        log(&app, "warning", warning.clone());
    }
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.insert(stored.clone());
    store.save().map_err(|error| error.to_string())?;
    Ok(CreatedProject {
        project: to_info(&stored),
        warnings: outcome.warnings,
        notes: outcome.notes,
    })
}
