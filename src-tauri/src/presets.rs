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
