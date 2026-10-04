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

pub(crate) fn build_preset(input: PresetInput, store: &PresetStore, now: i64) -> Result<Preset, String> {
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

    if store.name_taken(&input.kind, &name, input.id.as_deref()) {
        return Err(format!("A {} preset named \"{name}\" already exists", input.kind));
    }
    let existing = input.id.as_deref().and_then(|id| store.find(id)).cloned();
    if input.id.is_some() && existing.is_none() {
        return Err("This preset no longer exists".to_string());
    }
    Ok(Preset {
        id: existing
            .as_ref()
            .map(|preset| preset.id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string()),
        name,
        kind: input.kind,
        packages,
        created_at: existing.map(|preset| preset.created_at).unwrap_or(now),
        updated_at: now,
    })
}

#[tauri::command]
pub fn save_preset(
    input: PresetInput,
    state: State<'_, Mutex<PresetStore>>,
) -> Result<Preset, String> {
    let mut store = lock(&state)?;
    let preset = build_preset(input, &store, now_millis())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, version: Option<&str>) -> PresetPackage {
        PresetPackage {
            name: name.to_string(),
            version: version.map(str::to_string),
            display_name: None,
        }
    }

    fn input(id: Option<&str>, name: &str, kind: &str, packages: Vec<PresetPackage>) -> PresetInput {
        PresetInput {
            id: id.map(str::to_string),
            name: name.to_string(),
            kind: kind.to_string(),
            packages,
        }
    }

    fn empty_store() -> (tempfile::TempDir, PresetStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = PresetStore::load(dir.path().to_path_buf());
        (dir, store)
    }

    #[test]
    fn a_valid_preset_is_trimmed_and_given_an_id() {
        let (_dir, store) = empty_store();
        let preset = build_preset(input(None, "  My Setup  ", "world", vec![package("a", None)]), &store, 100).unwrap();
        assert_eq!(preset.name, "My Setup");
        assert_eq!(preset.kind, "world");
        assert!(!preset.id.is_empty());
        assert_eq!(preset.created_at, 100);
        assert_eq!(preset.updated_at, 100);
    }

    #[test]
    fn bad_names_kinds_and_packages_are_rejected() {
        let (_dir, store) = empty_store();
        let ok = || vec![package("a", None)];
        assert!(build_preset(input(None, "   ", "world", ok()), &store, 1).is_err());
        assert!(build_preset(input(None, &"x".repeat(61), "world", ok()), &store, 1).is_err());
        assert!(build_preset(input(None, "a\nb", "world", ok()), &store, 1).is_err());
        assert!(build_preset(input(None, "Name", "scene", ok()), &store, 1).is_err());
        assert!(build_preset(input(None, "Name", "world", vec![]), &store, 1).is_err());
        assert!(build_preset(input(None, "Name", "world", vec![package("a", Some("not-a-version"))]), &store, 1).is_err());
        assert!(build_preset(input(None, &"x".repeat(60), "world", ok()), &store, 1).is_ok());
    }

    #[test]
    fn duplicate_package_names_are_dropped_keeping_the_first() {
        let (_dir, store) = empty_store();
        let preset = build_preset(
            input(None, "Name", "avatar", vec![package("a", Some("1.0.0")), package("b", None), package("a", Some("2.0.0"))]),
            &store,
            1,
        )
        .unwrap();
        assert_eq!(preset.packages.len(), 2);
        assert_eq!(preset.packages[0].version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn names_may_repeat_across_types_but_not_within_one() {
        let (_dir, mut store) = empty_store();
        let first = build_preset(input(None, "Shared", "world", vec![package("a", None)]), &store, 1).unwrap();
        store.upsert(first);
        assert!(build_preset(input(None, "shared", "world", vec![package("a", None)]), &store, 2).is_err());
        assert!(build_preset(input(None, "Shared", "avatar", vec![package("a", None)]), &store, 2).is_ok());
    }

    #[test]
    fn editing_keeps_the_id_and_creation_time() {
        let (_dir, mut store) = empty_store();
        let created = build_preset(input(None, "Name", "world", vec![package("a", None)]), &store, 10).unwrap();
        let id = created.id.clone();
        store.upsert(created);
        let edited = build_preset(input(Some(&id), "Name", "world", vec![package("a", None), package("b", None)]), &store, 20).unwrap();
        assert_eq!(edited.id, id);
        assert_eq!(edited.created_at, 10);
        assert_eq!(edited.updated_at, 20);
        assert_eq!(edited.packages.len(), 2);
    }

    #[test]
    fn editing_a_preset_that_no_longer_exists_is_an_error() {
        let (_dir, store) = empty_store();
        assert!(build_preset(input(Some("gone"), "Name", "world", vec![package("a", None)]), &store, 1).is_err());
    }
}