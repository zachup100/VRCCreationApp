use crate::commands::packages::{gather, hidden_ids};
use crate::prefs::Prefs;
use crate::presets::{Preset, PresetPackage, PresetStore};
use crate::store::now_millis;
use crate::vpm::VpmContext;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::State;
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection, VersionSelector};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInput {
    id: Option<String>,
    name: String,
    kind: String,
    packages: Vec<PresetPackage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailablePackage {
    name: String,
    display_name: String,
    versions: Vec<String>,
    latest_version: Option<String>,
    source: &'static str,
    repository: Option<String>,
    requires_sdk: Option<&'static str>,
}

fn lock<'a>(
    state: &'a State<'_, Mutex<PresetStore>>,
) -> Result<std::sync::MutexGuard<'a, PresetStore>, String> {
    state
        .lock()
        .map_err(|_| "Presets are unavailable".to_string())
}

#[tauri::command]
pub fn list_presets(state: State<'_, Mutex<PresetStore>>) -> Result<Vec<Preset>, String> {
    let store = lock(&state)?;
    let mut presets = store.list().to_vec();
    presets.sort_by_key(|preset| preset.name.to_lowercase());
    Ok(presets)
}

#[tauri::command]
pub fn save_preset(
    input: PresetInput,
    state: State<'_, Mutex<PresetStore>>,
) -> Result<Preset, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Enter a preset name".to_string());
    }
    if name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err("The preset name is too long or contains invalid characters".to_string());
    }
    if input.kind != "world" && input.kind != "avatar" {
        return Err("Choose a template type".to_string());
    }

    let mut seen = HashSet::new();
    let mut packages = Vec::new();
    for package in input.packages {
        if let Some(version) = &package.version {
            Version::from_str(version).map_err(|_| format!("Invalid version {version}"))?;
        }
        if seen.insert(package.name.clone()) {
            packages.push(package);
        }
    }
    if packages.is_empty() {
        return Err("Select at least one package".to_string());
    }

    let mut store = lock(&state)?;
    if store.name_taken(&input.kind, &name, input.id.as_deref()) {
        return Err(format!("A {} preset named \"{name}\" already exists", input.kind));
    }
    let now = now_millis();
    let existing = input.id.as_deref().and_then(|id| store.find(id)).cloned();
    if input.id.is_some() && existing.is_none() {
        return Err("This preset no longer exists".to_string());
    }
    let preset = Preset {
        id: existing
            .as_ref()
            .map(|preset| preset.id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string()),
        name,
        kind: input.kind,
        packages,
        created_at: existing.map(|preset| preset.created_at).unwrap_or(now),
        updated_at: now,
    };
    store.upsert(preset.clone());
    store.save().map_err(|error| error.to_string())?;
    Ok(preset)
}

#[tauri::command]
pub fn delete_preset(id: String, state: State<'_, Mutex<PresetStore>>) -> Result<(), String> {
    let mut store = lock(&state)?;
    if store.remove(&id) {
        store.save().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn available_packages(
    refresh: bool,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<Vec<AvailablePackage>, String> {
    let _busy = vpm.try_busy()?;
    let allow_prerelease = prefs
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())?
        .settings()
        .show_prerelease;
    let hidden = hidden_ids(&prefs)?;
    let collection = vpm.collection(refresh).await?;

    let mut rows: Vec<AvailablePackage> = gather(&collection, &hidden, allow_prerelease)
        .into_iter()
        .filter(|(_, accumulator)| accumulator.visible)
        .map(|(name, accumulator)| {
            let (source, repository) = accumulator.classify();
            let mut versions = accumulator.versions;
            versions.sort();
            versions.dedup();
            versions.reverse();
            let requires_sdk = match name.as_str() {
                "com.vrchat.avatars" => Some("avatar"),
                "com.vrchat.worlds" => Some("world"),
                _ => collection
                    .find_package_by_name(&name, VersionSelector::latest_for(None, allow_prerelease))
                    .and_then(|info| {
                        let deps = info.vpm_dependencies();
                        if deps.contains_key("com.vrchat.avatars") {
                            Some("avatar")
                        } else if deps.contains_key("com.vrchat.worlds") {
                            Some("world")
                        } else {
                            None
                        }
                    }),
            };
            AvailablePackage {
                requires_sdk,
                display_name: accumulator.display_name.unwrap_or_else(|| name.clone()),
                latest_version: versions.first().map(Version::to_string),
                versions: versions.iter().map(Version::to_string).collect(),
                name,
                source,
                repository,
            }
        })
        .collect();
    rows.sort_by_key(|row| row.display_name.to_lowercase());
    Ok(rows)
}
