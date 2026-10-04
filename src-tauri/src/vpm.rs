use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::Mutex;
use vrc_get_vpm::io::DefaultEnvironmentIo;

pub struct VpmContext {
    pub io: DefaultEnvironmentIo,
    pub http: reqwest::Client,
    pub lock: Mutex<()>,
}

impl VpmContext {
    pub fn new(folder: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("VRCreationApp/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            io: DefaultEnvironmentIo::new(folder.into_boxed_path()),
            http,
            lock: Mutex::new(()),
        })
    }
}
