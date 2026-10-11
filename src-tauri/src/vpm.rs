use crate::limits::LimitedHttp;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use vrc_get_vpm::environment::{PackageCollection, Settings};
use vrc_get_vpm::io::DefaultEnvironmentIo;

pub const OFFICIAL_ID: &str = "com.vrchat.repos.official";
pub const CURATED_ID: &str = "com.vrchat.repos.curated";
pub const LOCAL_USER_ID: &str = "local-user-packages";

pub struct VpmContext {
    pub io: DefaultEnvironmentIo,
    pub http: reqwest::Client,
    pub lock: Mutex<()>,
    pub ops: Mutex<()>,
    collection: Mutex<Option<Arc<PackageCollection>>>,
}

impl VpmContext {
    pub fn new(folder: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("VRCCreationApp/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            io: DefaultEnvironmentIo::new(folder.into_boxed_path()),
            http,
            lock: Mutex::new(()),
            ops: Mutex::new(()),
            collection: Mutex::new(None),
        })
    }

    pub fn try_busy(&self) -> Result<tokio::sync::MutexGuard<'_, ()>, String> {
        self.ops.try_lock().map_err(|_| {
            "A package change is in progress. Wait for it to finish before making other changes."
                .to_string()
        })
    }

    pub async fn invalidate(&self) {
        *self.collection.lock().await = None;
    }

    pub async fn collection(&self, force: bool) -> Result<Arc<PackageCollection>, String> {
        let mut slot = self.collection.lock().await;
        if !force {
            if let Some(existing) = slot.as_ref() {
                return Ok(existing.clone());
            }
        }

        let settings = Settings::load(&self.io)
            .await
            .map_err(|error| format!("Could not read repository settings: {error}"))?;

        let mut loaded = None;
        if !force {
            let cached = PackageCollection::load_cache(&settings, &self.io)
                .await
                .map_err(|error| format!("Could not read the package cache: {error}"))?;
            let has_official = cached
                .get_remote()
                .any(|repository| repository.id() == Some(OFFICIAL_ID));
            if has_official {
                loaded = Some(cached);
            }
        }

        let collection = match loaded {
            Some(collection) => collection,
            None => PackageCollection::load(&settings, &self.io, Some(&LimitedHttp::new(&self.http)))
                .await
                .map_err(|error| format!("Could not load packages: {error}"))?,
        };

        let collection = Arc::new(collection);
        *slot = Some(collection.clone());
        Ok(collection)
    }
}
