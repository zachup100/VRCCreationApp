use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

const FILE_NAME: &str = "presets.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PresetPackage {
    pub name: String,
    pub version: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub packages: Vec<PresetPackage>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize)]
struct PresetFile {
    version: u32,
    presets: Vec<Preset>,
}

impl Default for PresetFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            presets: Vec::new(),
        }
    }
}

pub struct PresetStore {
    folder: PathBuf,
    data: PresetFile,
}

impl PresetStore {
    pub fn load(folder: PathBuf) -> Self {
        let file = folder.join(FILE_NAME);
        let data = match fs::read_to_string(&file) {
            Ok(text) => match serde_json::from_str::<PresetFile>(&text) {
                Ok(data) => data,
                Err(_) => {
                    let _ = fs::rename(&file, folder.join(format!("{FILE_NAME}.corrupt")));
                    PresetFile::default()
                }
            },
            Err(_) => PresetFile::default(),
        };
        Self { folder, data }
    }

    pub fn list(&self) -> &[Preset] {
        &self.data.presets
    }

    pub fn find(&self, id: &str) -> Option<&Preset> {
        self.data.presets.iter().find(|preset| preset.id == id)
    }

    pub fn name_taken(&self, kind: &str, name: &str, except_id: Option<&str>) -> bool {
        let wanted = name.to_lowercase();
        self.data.presets.iter().any(|preset| {
            preset.kind == kind
                && preset.name.to_lowercase() == wanted
                && Some(preset.id.as_str()) != except_id
        })
    }

    pub fn upsert(&mut self, preset: Preset) {
        match self
            .data
            .presets
            .iter_mut()
            .find(|existing| existing.id == preset.id)
        {
            Some(existing) => *existing = preset,
            None => self.data.presets.push(preset),
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.data.presets.len();
        self.data.presets.retain(|preset| preset.id != id);
        self.data.presets.len() != before
    }

    pub fn save(&self) -> io::Result<()> {
        fs::create_dir_all(&self.folder)?;
        let text = serde_json::to_string_pretty(&self.data)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
        let temp = self.folder.join(format!("{FILE_NAME}.tmp"));
        fs::write(&temp, text)?;
        fs::rename(&temp, self.folder.join(FILE_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str, name: &str, kind: &str) -> Preset {
        Preset {
            id: id.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            packages: vec![PresetPackage {
                name: "com.example.pkg".to_string(),
                version: None,
                display_name: Some("Example".to_string()),
            }],
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn names_are_unique_per_kind_ignoring_case() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = PresetStore::load(dir.path().to_path_buf());
        store.upsert(preset("1", "My Setup", "world"));
        assert!(store.name_taken("world", "my setup", None));
        assert!(!store.name_taken("avatar", "my setup", None));
        assert!(!store.name_taken("world", "my setup", Some("1")));
        assert!(!store.name_taken("world", "other", None));
    }

    #[test]
    fn upsert_replaces_an_existing_preset() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = PresetStore::load(dir.path().to_path_buf());
        store.upsert(preset("1", "First", "world"));
        store.upsert(preset("1", "Renamed", "world"));
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.find("1").unwrap().name, "Renamed");
    }

    #[test]
    fn remove_reports_whether_something_was_removed() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = PresetStore::load(dir.path().to_path_buf());
        store.upsert(preset("1", "First", "world"));
        assert!(store.remove("1"));
        assert!(!store.remove("1"));
        assert!(store.list().is_empty());
    }

    #[test]
    fn saved_presets_load_back_with_display_names() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = PresetStore::load(dir.path().to_path_buf());
        store.upsert(preset("1", "First", "avatar"));
        store.save().unwrap();
        let loaded = PresetStore::load(dir.path().to_path_buf());
        assert_eq!(loaded.list().len(), 1);
        assert_eq!(loaded.list()[0].kind, "avatar");
        assert_eq!(loaded.list()[0].packages[0].display_name.as_deref(), Some("Example"));
    }

    #[test]
    fn presets_saved_before_display_names_existed_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("presets.json"),
            r#"{"version":1,"presets":[{"id":"1","name":"Old","kind":"world","packages":[{"name":"a","version":null}],"createdAt":1,"updatedAt":1}]}"#,
        )
        .unwrap();
        let store = PresetStore::load(dir.path().to_path_buf());
        assert_eq!(store.list().len(), 1);
        assert!(store.list()[0].packages[0].display_name.is_none());
    }

    #[test]
    fn a_corrupt_file_is_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("presets.json"), "{").unwrap();
        let store = PresetStore::load(dir.path().to_path_buf());
        assert!(store.list().is_empty());
        assert!(dir.path().join("presets.json.corrupt").exists());
    }
}