use crate::commands::projects::{category_of, to_info, ProjectInfo};
use crate::store::{now_millis, ProjectStore, StoredProject};
use crate::vpm::VpmContext;
use include_dir::{include_dir, Dir, DirEntry};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;
use vrc_get_vpm::environment::PackageInstaller;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::UnityProject;

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
    let installer = PackageInstaller::new(&vpm.io, Some(&vpm.http));
    project
        .apply_pending_changes(&installer, changes)
        .await
        .map_err(|error| format!("Could not install the VRChat packages: {error}"))?;
    Ok(project)
}

#[tauri::command]
pub async fn create_project(
    template: String,
    name: String,
    location: String,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
) -> Result<ProjectInfo, String> {
    let _busy = vpm.try_busy()?;
    let template = TEMPLATE_LIST
        .iter()
        .find(|item| item.id == template)
        .ok_or_else(|| "Unknown project template".to_string())?;
    let name = name.trim().to_string();
    validate_name(&name)?;

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

    let project = match build_project(template, &name, &target, &vpm).await {
        Ok(project) => project,
        Err(message) => {
            let _ = tokio::fs::remove_dir_all(&target).await;
            return Err(message);
        }
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
    let mut store = store
        .lock()
        .map_err(|_| "Project store is unavailable".to_string())?;
    store.insert(stored.clone());
    store.save().map_err(|error| error.to_string())?;
    Ok(to_info(&stored))
}
