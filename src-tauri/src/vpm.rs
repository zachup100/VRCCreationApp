use crate::hosts;
use crate::download::ProgressHttp;
use crate::limits::{remove_incomplete_zips, LimitedHttp};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
    pub block_private_hosts: Arc<AtomicBool>,
    pub lock: Mutex<()>,
    pub ops: Mutex<()>,
    collection: Mutex<Option<Arc<PackageCollection>>>,
}

impl VpmContext {
    pub fn new(folder: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        let block_private_hosts = Arc::new(AtomicBool::new(true));
        let redirect_flag = block_private_hosts.clone();
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                if redirect_flag.load(Ordering::Relaxed) && hosts::is_private_url(attempt.url()) {
                    attempt.error(hosts::BLOCKED_MESSAGE)
                } else if attempt.previous().len() >= 10 {
                    attempt.error("Too many redirects")
                } else {
                    attempt.follow()
                }
            }))
            .user_agent(concat!("VRCCreationApp/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            io: DefaultEnvironmentIo::new(folder.into_boxed_path()),
            http,
            block_private_hosts,
            lock: Mutex::new(()),
            ops: Mutex::new(()),
            collection: Mutex::new(None),
        })
    }

    pub fn limited_http(&self) -> LimitedHttp<'_> {
        LimitedHttp::new(&self.http, self.private_hosts_blocked())
    }

    pub fn progress_http(&self, app: tauri::AppHandle) -> ProgressHttp {
        remove_incomplete_zips(&self.io.resolve(std::path::Path::new("Repos")));
        ProgressHttp::new(self.http.clone(), app, self.private_hosts_blocked())
    }

    pub fn private_hosts_blocked(&self) -> bool {
        self.block_private_hosts.load(Ordering::Relaxed)
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
            None => PackageCollection::load(&settings, &self.io, Some(&self.limited_http()))
                .await
                .map_err(|error| format!("Could not load packages: {error}"))?,
        };

        let collection = Arc::new(collection);
        *slot = Some(collection.clone());
        Ok(collection)
    }
}
