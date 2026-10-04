use crate::prefs::Prefs;
use crate::store::ProjectStore;
use crate::vpm::{VpmContext, CURATED_ID, LOCAL_USER_ID, OFFICIAL_ID};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection, UnityProject, VersionSelector};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRow {
    name: String,
    display_name: String,
    installed_version: Option<String>,
    latest_version: Option<String>,
    versions: Vec<String>,
    source: &'static str,
    repository: Option<String>,
    has_update: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPackages {
    unity_version: String,
    rows: Vec<PackageRow>,
}

#[derive(Default)]
pub(crate) struct Accumulator {
    pub display_name: Option<String>,
    pub versions: Vec<Version>,
    official: bool,
    curated: bool,
    local: bool,
    community: Vec<String>,
    pub visible: bool,
}

impl Accumulator {
    pub fn classify(&self) -> (&'static str, Option<String>) {
        if self.official {
            ("official", None)
        } else if self.curated {
            ("curated", None)
        } else if !self.community.is_empty() {
            ("community", self.community.first().cloned())
        } else {
            ("local", None)
        }
    }
}

pub(crate) fn hidden_ids(prefs: &Mutex<Prefs>) -> Result<HashSet<String>, String> {
    let prefs = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?;
    Ok([OFFICIAL_ID, CURATED_ID, LOCAL_USER_ID]
        .into_iter()
        .chain(prefs.hidden().iter().map(String::as_str))
        .filter(|id| prefs.is_hidden(id))
        .map(str::to_string)
        .collect())
}

pub(crate) fn gather(
    collection: &vrc_get_vpm::environment::PackageCollection,
    hidden: &HashSet<String>,
    allow_prerelease: bool,
) -> HashMap<String, Accumulator> {
    let mut names: HashMap<String, Accumulator> = HashMap::new();
    for info in collection.get_all_packages() {
        if info.is_yanked() {
            continue;
        }
        let entry = names.entry(info.name().to_string()).or_default();
        if entry.display_name.is_none() {
            entry.display_name = info.display_name().map(str::to_string);
        }
        if allow_prerelease || !info.version().is_pre() {
            entry.versions.push(info.version().clone());
        }
        match info.repo() {
            Some(repository) => {
                let id = repository
                    .id()
                    .or(repository.url().map(|url| url.as_str()))
                    .unwrap_or_default();
                entry.visible |= !hidden.contains(id);
                match id {
                    OFFICIAL_ID => entry.official = true,
                    CURATED_ID => entry.curated = true,
                    _ => {
                        let label = repository.name().unwrap_or(id).to_string();
                        if !entry.community.contains(&label) {
                            entry.community.push(label);
                        }
                    }
                }
            }
            None => {
                entry.local = true;
                entry.visible |= !hidden.contains(LOCAL_USER_ID);
            }
        }
    }
    names
}

#[tauri::command]
pub async fn project_packages(
    path: String,
    refresh: bool,
    vpm: State<'_, VpmContext>,
    store: State<'_, Mutex<ProjectStore>>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<ProjectPackages, String> {
    let _busy = vpm.try_busy()?;
    {
        let store = store
            .lock()
            .map_err(|_| "Project store is unavailable".to_string())?;
        if store.find(&path).is_none() {
            return Err("This project is not in the list".to_string());
        }
    }

    let allow_prerelease = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .show_prerelease;
    let hidden = hidden_ids(&prefs)?;

    let project = UnityProject::load(DefaultProjectIo::new(PathBuf::from(&path).into_boxed_path()))
        .await
        .map_err(|error| format!("Could not read the project: {error}"))?;
    let collection = vpm.collection(refresh).await?;

    let unity = project.unity_version();
    let selector = VersionSelector::latest_for(Some(unity), allow_prerelease);

    let mut names = gather(&collection, &hidden, allow_prerelease);

    let mut installed: HashMap<String, (Version, Option<String>)> = HashMap::new();
    for manifest in project.all_installed_packages() {
        installed.insert(
            manifest.name().to_string(),
            (
                manifest.version().clone(),
                manifest.display_name().map(str::to_string),
            ),
        );
    }

    let mut all_names: HashSet<String> = names.keys().cloned().collect();
    all_names.extend(installed.keys().cloned());

    let mut rows: Vec<PackageRow> = all_names
        .into_iter()
        .filter_map(|name| {
            let accumulator = names.remove(&name).unwrap_or_default();
            let installed_entry = installed.get(&name);
            if installed_entry.is_none() && !accumulator.visible {
                return None;
            }

            let latest = collection
                .find_package_by_name(&name, selector)
                .map(|info| info.version().clone());
            let has_update = match (&latest, installed_entry) {
                (Some(latest), Some((current, _))) => latest > current,
                _ => false,
            };

            let (source, repository) = accumulator.classify();
            let mut versions = accumulator.versions;
            if let Some((current, _)) = installed_entry {
                versions.push(current.clone());
            }
            versions.sort();
            versions.dedup();
            versions.reverse();

            let display_name = accumulator
                .display_name
                .or_else(|| installed_entry.and_then(|(_, display)| display.clone()))
                .unwrap_or_else(|| name.clone());

            Some(PackageRow {
                installed_version: installed_entry.map(|(version, _)| version.to_string()),
                latest_version: latest.map(|version| version.to_string()),
                versions: versions.iter().map(Version::to_string).collect(),
                display_name,
                name,
                source,
                repository,
                has_update,
            })
        })
        .collect();

    rows.sort_by(|a, b| {
        b.installed_version
            .is_some()
            .cmp(&a.installed_version.is_some())
            .then_with(|| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            })
    });

    Ok(ProjectPackages {
        unity_version: unity.to_string(),
        rows,
    })
}
