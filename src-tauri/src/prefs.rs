use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

const FILE_NAME: &str = "preferences.json";

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PrefsFile {
    #[serde(default)]
    hidden_repositories: Vec<String>,
}

pub struct Prefs {
    folder: PathBuf,
    data: PrefsFile,
}

impl Prefs {
    pub fn load(folder: PathBuf) -> Self {
        let file = folder.join(FILE_NAME);
        let data = fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { folder, data }
    }

    pub fn hidden(&self) -> &[String] {
        &self.data.hidden_repositories
    }

    pub fn is_hidden(&self, id: &str) -> bool {
        self.data.hidden_repositories.iter().any(|item| item == id)
    }

    pub fn set_hidden(&mut self, id: &str, hidden: bool) {
        self.data.hidden_repositories.retain(|item| item != id);
        if hidden {
            self.data.hidden_repositories.push(id.to_string());
        }
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
