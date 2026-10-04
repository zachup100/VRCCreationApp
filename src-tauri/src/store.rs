use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const FILE_NAME: &str = "projects.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoredProject {
    pub path: String,
    pub unity_version: Option<String>,
    pub category: String,
    pub favorite: bool,
    pub added_at: i64,
    pub last_modified: i64,
    #[serde(default)]
    pub launch_args: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    version: u32,
    projects: Vec<StoredProject>,
}

impl Default for StoreFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            projects: Vec::new(),
        }
    }
}

pub struct ProjectStore {
    folder: PathBuf,
    data: StoreFile,
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn path_key(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    if cfg!(windows) {
        trimmed.replace('/', "\\").to_lowercase()
    } else {
        trimmed.to_string()
    }
}

impl ProjectStore {
    pub fn load(folder: PathBuf) -> Self {
        let file = folder.join(FILE_NAME);
        let data = match fs::read_to_string(&file) {
            Ok(text) => match serde_json::from_str::<StoreFile>(&text) {
                Ok(data) => data,
                Err(_) => {
                    let _ = fs::rename(&file, folder.join(format!("{FILE_NAME}.corrupt")));
                    StoreFile::default()
                }
            },
            Err(_) => StoreFile::default(),
        };
        Self { folder, data }
    }

    pub fn projects(&self) -> &[StoredProject] {
        &self.data.projects
    }

    pub fn contains(&self, path: &str) -> bool {
        let key = path_key(path);
        self.data
            .projects
            .iter()
            .any(|project| path_key(&project.path) == key)
    }

    pub fn find(&self, path: &str) -> Option<&StoredProject> {
        let key = path_key(path);
        self.data
            .projects
            .iter()
            .find(|project| path_key(&project.path) == key)
    }

    pub fn touch(&mut self, path: &str) {
        let key = path_key(path);
        if let Some(project) = self
            .data
            .projects
            .iter_mut()
            .find(|project| path_key(&project.path) == key)
        {
            project.last_modified = now_millis();
        }
    }

    pub fn set_launch_args(&mut self, path: &str, args: Option<String>) -> bool {
        let key = path_key(path);
        match self
            .data
            .projects
            .iter_mut()
            .find(|project| path_key(&project.path) == key)
        {
            Some(project) => {
                project.launch_args = args;
                true
            }
            None => false,
        }
    }

    pub fn update_info(&mut self, path: &str, unity_version: Option<String>, category: String) {
        let key = path_key(path);
        if let Some(project) = self
            .data
            .projects
            .iter_mut()
            .find(|project| path_key(&project.path) == key)
        {
            project.unity_version = unity_version;
            project.category = category;
            project.last_modified = now_millis();
        }
    }

    pub fn insert(&mut self, project: StoredProject) -> bool {
        if self.contains(&project.path) {
            return false;
        }
        self.data.projects.push(project);
        true
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let key = path_key(path);
        let before = self.data.projects.len();
        self.data
            .projects
            .retain(|project| path_key(&project.path) != key);
        self.data.projects.len() != before
    }

    pub fn set_favorite(&mut self, path: &str, favorite: bool) -> bool {
        let key = path_key(path);
        match self
            .data
            .projects
            .iter_mut()
            .find(|project| path_key(&project.path) == key)
        {
            Some(project) => {
                project.favorite = favorite;
                true
            }
            None => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project(path: &str) -> StoredProject {
        StoredProject {
            path: path.to_string(),
            unity_version: Some("2022.3.22f1".to_string()),
            category: "world".to_string(),
            favorite: false,
            added_at: 1,
            last_modified: 1,
            launch_args: None,
        }
    }

    #[test]
    fn path_key_ignores_trailing_separators() {
        assert_eq!(path_key("/a/b/"), path_key("/a/b"));
        assert_eq!(path_key("/a/b\\"), path_key("/a/b"));
    }

    #[test]
    fn inserting_the_same_path_twice_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ProjectStore::load(dir.path().to_path_buf());
        assert!(store.insert(project("/p/one")));
        assert!(!store.insert(project("/p/one")));
        assert!(!store.insert(project("/p/one/")));
        assert_eq!(store.projects().len(), 1);
    }

    #[test]
    fn remove_and_find_use_normalised_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ProjectStore::load(dir.path().to_path_buf());
        store.insert(project("/p/one"));
        assert!(store.find("/p/one/").is_some());
        assert!(store.contains("/p/one"));
        assert!(store.remove("/p/one/"));
        assert!(!store.remove("/p/one"));
        assert!(store.find("/p/one").is_none());
    }

    #[test]
    fn updates_change_only_the_matching_project() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ProjectStore::load(dir.path().to_path_buf());
        store.insert(project("/p/one"));
        store.insert(project("/p/two"));
        assert!(store.set_favorite("/p/one", true));
        assert!(!store.set_favorite("/p/missing", true));
        assert!(store.set_launch_args("/p/two", Some("-x".to_string())));
        store.update_info("/p/two", Some("2019.4.31f1".to_string()), "avatar".to_string());
        assert!(store.find("/p/one").unwrap().favorite);
        assert!(!store.find("/p/two").unwrap().favorite);
        assert_eq!(store.find("/p/two").unwrap().launch_args.as_deref(), Some("-x"));
        assert_eq!(store.find("/p/two").unwrap().unity_version.as_deref(), Some("2019.4.31f1"));
        assert_eq!(store.find("/p/two").unwrap().category, "avatar");
        assert_eq!(store.find("/p/one").unwrap().category, "world");
    }

    #[test]
    fn saved_projects_load_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ProjectStore::load(dir.path().to_path_buf());
        store.insert(project("/p/one"));
        store.set_launch_args("/p/one", Some("-a -b".to_string()));
        store.save().unwrap();
        let loaded = ProjectStore::load(dir.path().to_path_buf());
        assert_eq!(loaded.projects().len(), 1);
        assert_eq!(loaded.projects()[0].path, "/p/one");
        assert_eq!(loaded.projects()[0].launch_args.as_deref(), Some("-a -b"));
        assert!(!dir.path().join("projects.json.tmp").exists());
    }

    #[test]
    fn older_files_without_launch_args_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("projects.json"),
            r#"{"version":1,"projects":[{"path":"/p/one","unityVersion":null,"category":"other","favorite":false,"addedAt":1,"lastModified":2}]}"#,
        )
        .unwrap();
        let store = ProjectStore::load(dir.path().to_path_buf());
        assert_eq!(store.projects().len(), 1);
        assert!(store.projects()[0].launch_args.is_none());
    }

    #[test]
    fn a_corrupt_file_is_set_aside_and_an_empty_store_starts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("projects.json"), "this is not json").unwrap();
        let store = ProjectStore::load(dir.path().to_path_buf());
        assert!(store.projects().is_empty());
        assert!(dir.path().join("projects.json.corrupt").exists());
        assert!(!dir.path().join("projects.json").exists());
    }
}