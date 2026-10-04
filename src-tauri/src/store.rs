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
