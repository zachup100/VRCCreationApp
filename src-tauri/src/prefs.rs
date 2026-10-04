use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

const FILE_NAME: &str = "preferences.json";

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub default_project_location: Option<String>,
    #[serde(default)]
    pub backup_location: Option<String>,
    #[serde(default)]
    pub show_prerelease: bool,
    #[serde(default)]
    pub default_unity_args: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PrefsFile {
    #[serde(default)]
    hidden_repositories: Vec<String>,
    #[serde(default)]
    hidden_packages: Vec<String>,
    #[serde(flatten)]
    settings: AppSettings,
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

    pub fn hidden_package_set(&self) -> std::collections::HashSet<String> {
        self.data.hidden_packages.iter().cloned().collect()
    }

    pub fn set_packages_hidden(&mut self, names: &[String], hidden: bool) {
        self.data
            .hidden_packages
            .retain(|existing| !names.contains(existing));
        if hidden {
            for name in names {
                if !self.data.hidden_packages.contains(name) {
                    self.data.hidden_packages.push(name.clone());
                }
            }
        }
    }

    pub fn show_all_packages(&mut self) {
        self.data.hidden_packages.clear();
    }

    pub fn settings(&self) -> &AppSettings {
        &self.data.settings
    }

    pub fn settings_mut(&mut self) -> &mut AppSettings {
        &mut self.data.settings
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

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let prefs = Prefs::load(dir.path().to_path_buf());
        assert!(prefs.settings().default_project_location.is_none());
        assert!(!prefs.settings().show_prerelease);
        assert!(prefs.settings().default_unity_args.is_empty());
        assert!(prefs.hidden().is_empty());
    }

    #[test]
    fn files_from_before_settings_existed_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("preferences.json"), r#"{"hiddenRepositories":["com.example.repo"]}"#).unwrap();
        let prefs = Prefs::load(dir.path().to_path_buf());
        assert!(prefs.is_hidden("com.example.repo"));
        assert!(!prefs.settings().show_prerelease);
    }

    #[test]
    fn hiding_and_showing_a_repository_toggles_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut prefs = Prefs::load(dir.path().to_path_buf());
        prefs.set_hidden("a", true);
        prefs.set_hidden("a", true);
        assert_eq!(prefs.hidden().len(), 1);
        assert!(prefs.is_hidden("a"));
        prefs.set_hidden("a", false);
        assert!(!prefs.is_hidden("a"));
    }

    #[test]
    fn settings_and_hidden_repositories_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut prefs = Prefs::load(dir.path().to_path_buf());
        prefs.set_hidden("a", true);
        let settings = prefs.settings_mut();
        settings.show_prerelease = true;
        settings.default_unity_args = "-x".to_string();
        settings.backup_location = Some("/backups".to_string());
        prefs.save().unwrap();
        let loaded = Prefs::load(dir.path().to_path_buf());
        assert!(loaded.is_hidden("a"));
        assert!(loaded.settings().show_prerelease);
        assert_eq!(loaded.settings().default_unity_args, "-x");
        assert_eq!(loaded.settings().backup_location.as_deref(), Some("/backups"));
    }

    #[test]
    fn packages_can_be_hidden_and_shown_in_bulk() {
        let dir = tempfile::tempdir().unwrap();
        let mut prefs = Prefs::load(dir.path().to_path_buf());
        let names = vec!["a".to_string(), "b".to_string()];
        prefs.set_packages_hidden(&names, true);
        prefs.set_packages_hidden(&["a".to_string()], true);
        assert_eq!(prefs.hidden_package_set().len(), 2);
        prefs.set_packages_hidden(&["a".to_string()], false);
        assert!(!prefs.hidden_package_set().contains("a"));
        assert!(prefs.hidden_package_set().contains("b"));
        prefs.show_all_packages();
        assert!(prefs.hidden_package_set().is_empty());
    }

    #[test]
    fn hidden_packages_survive_a_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut prefs = Prefs::load(dir.path().to_path_buf());
        prefs.set_packages_hidden(&["com.example.pkg".to_string()], true);
        prefs.save().unwrap();
        let loaded = Prefs::load(dir.path().to_path_buf());
        assert!(loaded.hidden_package_set().contains("com.example.pkg"));
    }
}
