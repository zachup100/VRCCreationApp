use crate::applog::log;
use crate::editors;
use crate::limits::check_host;
use crate::prefs::Prefs;
use crate::throttle::{limit_allowed, Config, Throttle, BYTES_PER_MEGABYTE};
use crate::vpm::VpmContext;
use base64::Engine;
use reqwest::Url;
use serde::Serialize;
use sha2::{Digest, Sha512};
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::OwnedMutexGuard;

const EVENT: &str = "unity-install";
const FEED_URL: &str = "https://public-cdn.cloud.unity3d.com/hub/prod/latest.yml";
const FEED_BASE: &str = "https://public-cdn.cloud.unity3d.com/hub/prod/";
const MAX_FEED_BYTES: usize = 64 * 1024;
const MAX_HUB_BYTES: u64 = 500 * 1024 * 1024;
const FEED_TIMEOUT: Duration = Duration::from_secs(30);
const EDITOR_LIMIT: Duration = Duration::from_secs(4 * 60 * 60);
const SETUP_LIMIT: Duration = Duration::from_secs(15 * 60);
const SIGNATURE_LIMIT: Duration = Duration::from_secs(60);
const EMIT_INTERVAL: Duration = Duration::from_millis(150);
const DECLINED_CODE: i32 = 1223;
const LINE_LIMIT: usize = 200;
const TAIL_LINES: usize = 6;

const SIGNATURE_SCRIPT: &str = "$s = Get-AuthenticodeSignature -LiteralPath $env:VRCC_FILE; if ($s.Status -eq 'Valid' -and $s.SignerCertificate -ne $null -and $s.SignerCertificate.GetNameInfo('SimpleName', $false) -like 'Unity Technologies*') { exit 0 } else { exit 1 }";
const SETUP_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; try { $p = Start-Process -FilePath $env:VRCC_FILE -ArgumentList '/S' -Verb RunAs -Wait -PassThru; exit $p.ExitCode } catch { exit 1223 }";

pub struct Recommended {
    pub version: &'static str,
    pub changeset: &'static str,
    pub label: &'static str,
    pub note: &'static str,
}

pub const RECOMMENDED: [Recommended; 2] = [
    Recommended {
        version: "2022.3.22f1",
        changeset: "887be4894c44",
        label: "Current VRChat version",
        note: "Used for new worlds and avatars.",
    },
    Recommended {
        version: "2019.4.31f1",
        changeset: "bd5abf232a62",
        label: "Legacy VRChat version",
        note: "For older projects that have not been upgraded.",
    },
];

pub fn recommended(version: &str) -> Option<&'static Recommended> {
    RECOMMENDED.iter().find(|entry| entry.version == version)
}

pub fn can_install_hub() -> bool {
    cfg!(windows)
}

fn hub_candidates() -> Vec<PathBuf> {
    let mut found = Vec::new();
    if cfg!(windows) {
        for variable in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
            if let Some(base) = std::env::var_os(variable) {
                found.push(PathBuf::from(base).join("Unity Hub").join("Unity Hub.exe"));
            }
        }
        if let Some(base) = std::env::var_os("LOCALAPPDATA") {
            found.push(
                PathBuf::from(base)
                    .join("Programs")
                    .join("Unity Hub")
                    .join("Unity Hub.exe"),
            );
        }
    } else if cfg!(target_os = "macos") {
        found.push(PathBuf::from("/Applications/Unity Hub.app/Contents/MacOS/Unity Hub"));
    } else {
        for path in ["/opt/unityhub/unityhub", "/usr/bin/unityhub", "/usr/local/bin/unityhub"] {
            found.push(PathBuf::from(path));
        }
    }
    found
}

pub fn find_hub() -> Option<PathBuf> {
    hub_candidates().into_iter().find(|path| path.is_file())
}

pub fn launch_flags() -> Vec<String> {
    if cfg!(target_os = "linux") {
        vec!["--ozone-platform=headless".to_string()]
    } else {
        Vec::new()
    }
}

pub fn editor_args(entry: &Recommended, android: bool, ios: bool) -> Vec<String> {
    let mut args: Vec<String> = [
        "--",
        "--headless",
        "install",
        "--version",
        entry.version,
        "--changeset",
        entry.changeset,
    ]
    .iter()
    .map(|part| part.to_string())
    .collect();
    if android {
        args.extend(["--module", "android"].iter().map(|part| part.to_string()));
    }
    if ios {
        args.extend(["--module", "ios"].iter().map(|part| part.to_string()));
    }
    if android || ios {
        args.push("--childModules".to_string());
    }
    args
}

pub fn sanitize_line(line: &str) -> String {
    let mut cleaned = String::new();
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            match characters.peek() {
                Some('[') => {
                    characters.next();
                    for next in characters.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(_) => {
                    characters.next();
                }
                None => {}
            }
        } else if !character.is_control() {
            cleaned.push(character);
        }
    }
    cleaned.trim().chars().take(LINE_LIMIT).collect()
}

pub fn parse_status(line: &str) -> Option<(String, String, Option<f64>)> {
    let rest = line.strip_prefix('[')?;
    let (name, status) = rest.split_once(']')?;
    let status = status.trim();
    if name.trim().is_empty() || status.is_empty() {
        return None;
    }
    let percent = status
        .split_whitespace()
        .last()
        .and_then(|word| word.strip_suffix('%'))
        .and_then(|number| number.parse::<f64>().ok())
        .filter(|value| (0.0..=100.0).contains(value));
    Some((name.trim().to_string(), status.to_string(), percent))
}

#[derive(Default)]
pub struct Progress {
    items: Vec<(String, f64)>,
}

impl Progress {
    pub fn update(&mut self, line: &str) -> Option<String> {
        let (name, status, percent) = parse_status(line)?;
        let index = match self.items.iter().position(|(known, _)| *known == name) {
            Some(index) => index,
            None => {
                self.items.push((name.clone(), 0.0));
                self.items.len() - 1
            }
        };
        if let Some(value) = percent {
            self.items[index].1 = value;
        }
        Some(format!("{name}: {status}"))
    }

    pub fn percent(&self) -> u32 {
        if self.items.is_empty() {
            return 0;
        }
        let total: f64 = self.items.iter().map(|(_, value)| value).sum();
        ((total / self.items.len() as f64).floor() as u32).min(99)
    }
}

pub fn system_tool(relative: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let root = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir"))?;
    Some(PathBuf::from(root).join("System32").join(relative))
}

#[cfg(unix)]
fn kill_tree(child: &mut std::process::Child) {
    let group = format!("-{}", child.id());
    let _ = Command::new("kill")
        .args(["-KILL", "--", &group])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
fn kill_tree(child: &mut std::process::Child) {
    if let Some(tool) = system_tool("taskkill.exe") {
        let mut command = Command::new(tool);
        command
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_window(&mut command);
        let _ = command.status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub enum RunEnd {
    Exited(Option<i32>),
    Cancelled,
    TimedOut,
}

pub fn run_streaming(
    mut command: Command,
    cancel: Option<&AtomicBool>,
    limit: Duration,
    mut on_line: impl FnMut(&str),
) -> std::io::Result<RunEnd> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn()?;
    let (sender, receiver) = mpsc::channel::<String>();
    if let Some(stdout) = child.stdout.take() {
        let sender = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sender = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(sender);

    let started = Instant::now();
    loop {
        match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => on_line(&line),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let status = child.wait()?;
                return Ok(RunEnd::Exited(status.code()));
            }
        }
        if cancel.map(|flag| flag.load(Ordering::SeqCst)).unwrap_or(false) {
            kill_tree(&mut child);
            return Ok(RunEnd::Cancelled);
        }
        if started.elapsed() > limit {
            kill_tree(&mut child);
            return Ok(RunEnd::TimedOut);
        }
        if let Some(status) = child.try_wait()? {
            while let Ok(line) = receiver.recv_timeout(Duration::from_millis(300)) {
                on_line(&line);
            }
            return Ok(RunEnd::Exited(status.code()));
        }
    }
}

#[derive(Debug)]
pub struct HubFeed {
    pub path: String,
    pub sha512: Vec<u8>,
    pub size: u64,
}

fn valid_installer_path(path: &str) -> bool {
    let mut parts = path.split('/');
    let (Some(folder), Some(file), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let folder_ok = !folder.is_empty()
        && folder.chars().all(|character| character.is_ascii_digit() || character == '.');
    let file_ok = file.starts_with("UnityHubSetup-")
        && file.ends_with(".exe")
        && file
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '.' || character == '-');
    folder_ok && file_ok
}

pub fn parse_feed(text: &str, arch: &str) -> Result<HubFeed, String> {
    let mut entries: Vec<(String, Option<String>, Option<u64>)> = Vec::new();
    for raw in text.lines() {
        if !raw.starts_with(' ') {
            continue;
        }
        let line = raw.trim();
        if let Some(value) = line.strip_prefix("- url:") {
            entries.push((value.trim().to_string(), None, None));
        } else if let Some(value) = line.strip_prefix("sha512:") {
            if let Some(last) = entries.last_mut() {
                last.1 = Some(value.trim().to_string());
            }
        } else if let Some(value) = line.strip_prefix("size:") {
            if let Some(last) = entries.last_mut() {
                last.2 = value.trim().parse().ok();
            }
        }
    }
    let suffix = format!("-{arch}.exe");
    let (path, hash, size) = entries
        .into_iter()
        .find(|(path, _, _)| path.ends_with(&suffix))
        .ok_or_else(|| "Unity's download list has no installer for this computer".to_string())?;
    if !valid_installer_path(&path) {
        return Err("Unity's download list holds an unexpected address".to_string());
    }
    let size = size.ok_or_else(|| "Unity's download list has no file size".to_string())?;
    if size == 0 || size > MAX_HUB_BYTES {
        return Err("The Unity Hub installer has an unexpected size".to_string());
    }
    let sha512 = base64::engine::general_purpose::STANDARD
        .decode(hash.ok_or_else(|| "Unity's download list has no checksum".to_string())?)
        .ok()
        .filter(|bytes| bytes.len() == 64)
        .ok_or_else(|| "Unity's download list has an unreadable checksum".to_string())?;
    Ok(HubFeed { path, sha512, size })
}

fn arch_suffix() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

pub enum Stop {
    Cancelled,
    Failed(String),
}

impl From<String> for Stop {
    fn from(message: String) -> Self {
        Stop::Failed(message)
    }
}

impl From<&str> for Stop {
    fn from(message: &str) -> Self {
        Stop::Failed(message.to_string())
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallEvent {
    kind: &'static str,
    phase: &'static str,
    bytes: u64,
    total: u64,
    line: String,
    version: String,
    message: String,
}

pub struct Session {
    app: AppHandle,
    lock: Option<OwnedMutexGuard<()>>,
    pub cancel: Arc<AtomicBool>,
    kind: &'static str,
    pub version: String,
    finished: bool,
}

fn outcome_text(kind: &str, version: &str, result: &Result<(), Stop>) -> (&'static str, String, &'static str) {
    match result {
        Ok(()) => (
            "finished",
            if kind == "hub" {
                "Unity Hub is installed".to_string()
            } else {
                format!("Unity {version} is installed")
            },
            "info",
        ),
        Err(Stop::Cancelled) => ("cancelled", "The install was cancelled".to_string(), "info"),
        Err(Stop::Failed(message)) => ("failed", message.clone(), "warning"),
    }
}

impl Session {
    pub fn begin(
        app: &AppHandle,
        vpm: &VpmContext,
        kind: &'static str,
        version: &str,
    ) -> Result<Self, String> {
        let lock = vpm.try_busy_owned()?;
        vpm.installing.store(true, Ordering::SeqCst);
        let cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut slot) = vpm.install_cancel.lock() {
            *slot = Some(cancel.clone());
        }
        let session = Self {
            app: app.clone(),
            lock: Some(lock),
            cancel,
            kind,
            version: version.to_string(),
            finished: false,
        };
        session.emit("starting", 0, 0, "");
        Ok(session)
    }

    pub fn emit(&self, phase: &'static str, bytes: u64, total: u64, line: &str) {
        let _ = self.app.emit(
            EVENT,
            InstallEvent {
                kind: self.kind,
                phase,
                bytes,
                total,
                line: line.to_string(),
                version: self.version.clone(),
                message: String::new(),
            },
        );
    }

    fn release(&mut self) {
        if self.lock.take().is_some() {
            if let Some(vpm) = self.app.try_state::<VpmContext>() {
                vpm.installing.store(false, Ordering::SeqCst);
                if let Ok(mut slot) = vpm.install_cancel.lock() {
                    *slot = None;
                }
            }
        }
    }

    fn finish(&mut self, result: &Result<(), Stop>) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.release();
        let (phase, message, level) = outcome_text(self.kind, &self.version, result);
        log(&self.app, level, message.clone());
        let _ = self.app.emit(
            EVENT,
            InstallEvent {
                kind: self.kind,
                phase,
                bytes: 0,
                total: 0,
                line: String::new(),
                version: self.version.clone(),
                message,
            },
        );
    }

    pub fn end(mut self, result: Result<(), Stop>) {
        self.finish(&result);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.finish(&Err(Stop::Failed("The install stopped unexpectedly".to_string())));
    }
}

pub fn request_cancel(vpm: &VpmContext) -> bool {
    match vpm.install_cancel.lock() {
        Ok(slot) => match slot.as_ref() {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        },
        Err(_) => false,
    }
}

pub async fn run_editor_install(session: Session, android: bool, ios: bool) {
    let result = editor_install(&session, android, ios).await;
    session.end(result);
}

async fn editor_install(session: &Session, android: bool, ios: bool) -> Result<(), Stop> {
    let entry = recommended(&session.version).ok_or("That version is not on the recommended list")?;
    let hub = find_hub().ok_or("Unity Hub was not found")?;
    log(
        &session.app,
        "info",
        format!("Installing Unity {} through Unity Hub", entry.version),
    );
    session.emit("installing", 0, 0, "Starting Unity Hub");

    let limit = {
        let prefs = session.app.state::<Mutex<Prefs>>();
        let value = prefs
            .lock()
            .map(|guard| guard.settings().unity_download_limit)
            .unwrap_or(0);
        if limit_allowed(value) {
            value
        } else {
            0
        }
    };
    let throttle = if limit > 0 {
        let blocked = session.app.state::<VpmContext>().private_hosts_blocked();
        let config = Config::unity(limit as u64 * BYTES_PER_MEGABYTE, blocked);
        Some(
            Throttle::start(config)
                .await
                .map_err(|error| format!("Could not start the download speed limiter: {error}"))?,
        )
    } else {
        None
    };

    let mut command = Command::new(&hub);
    command.args(launch_flags()).args(editor_args(entry, android, ios));
    if let Some(proxy) = &throttle {
        for name in ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"] {
            command.env(name, proxy.url());
        }
        for name in ["NO_PROXY", "no_proxy"] {
            command.env(name, "");
        }
    }
    hide_window(&mut command);

    let cancel = session.cancel.clone();
    let app = session.app.clone();
    let version = session.version.clone();
    let (outcome, tail) = tokio::task::spawn_blocking(move || {
        let mut tail: Vec<String> = Vec::new();
        let mut progress = Progress::default();
        let mut shown = String::new();
        let mut last = Instant::now() - EMIT_INTERVAL;
        let outcome = run_streaming(command, Some(&cancel), EDITOR_LIMIT, |raw| {
            let line = sanitize_line(raw);
            if line.is_empty() {
                return;
            }
            match progress.update(&line) {
                Some(status) => shown = status,
                None => {
                    if line.ends_with(':') {
                        return;
                    }
                    if tail.len() == TAIL_LINES {
                        tail.remove(0);
                    }
                    tail.push(line.clone());
                    shown = line;
                }
            }
            if last.elapsed() >= EMIT_INTERVAL {
                last = Instant::now();
                let percent = progress.percent() as u64;
                let _ = app.emit(
                    EVENT,
                    InstallEvent {
                        kind: "editor",
                        phase: "installing",
                        bytes: percent,
                        total: if percent > 0 { 100 } else { 0 },
                        line: shown.clone(),
                        version: version.clone(),
                        message: String::new(),
                    },
                );
            }
        });
        (outcome, tail)
    })
    .await
    .map_err(|_| "The install stopped unexpectedly")?;

    match outcome {
        Err(error) => return Err(Stop::Failed(format!("Could not start Unity Hub: {error}"))),
        Ok(RunEnd::Cancelled) => return Err(Stop::Cancelled),
        Ok(RunEnd::TimedOut) => return Err("The install took too long and was stopped".into()),
        Ok(RunEnd::Exited(Some(0))) => {}
        Ok(RunEnd::Exited(code)) => {
            return Err(Stop::Failed(failure_text(
                &format!(
                    "Unity Hub stopped with code {}",
                    code.map(|value| value.to_string()).unwrap_or_else(|| "unknown".to_string())
                ),
                &tail,
            )))
        }
    }

    let prefs = session.app.state::<Mutex<Prefs>>();
    let listed = editors::all(&prefs)
        .await
        .map(|found| found.iter().any(|editor| editor.version == session.version))
        .unwrap_or(false);
    if !listed {
        return Err(Stop::Failed(failure_text(
            &format!(
                "Unity Hub finished, but Unity {} is not listed yet. Rescan in a moment",
                session.version
            ),
            &tail,
        )));
    }
    Ok(())
}

fn failure_text(headline: &str, tail: &[String]) -> String {
    match tail.last() {
        Some(last) => format!("{headline}. Unity Hub said: {last}"),
        None => format!("{headline}."),
    }
}

pub async fn run_hub_install(session: Session) {
    let folder = session
        .app
        .path()
        .app_data_dir()
        .ok()
        .map(|base| base.join("hub-installer"));
    let result = match &folder {
        Some(folder) => {
            let _ = std::fs::remove_dir_all(folder);
            let result = hub_install(&session, folder).await;
            let _ = std::fs::remove_dir_all(folder);
            result
        }
        None => Err(Stop::Failed("The app data folder is unavailable".to_string())),
    };
    session.end(result);
}

async fn hub_install(session: &Session, folder: &Path) -> Result<(), Stop> {
    let vpm = session.app.state::<VpmContext>();
    log(&session.app, "info", "Installing Unity Hub".to_string());
    session.emit("downloading", 0, 0, "Checking the latest Unity Hub");

    let feed_text = fetch_feed(&vpm).await?;
    let feed = parse_feed(&feed_text, arch_suffix())?;
    let file_name = feed.path.rsplit('/').next().unwrap_or("UnityHubSetup.exe").to_string();

    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let setup = folder.join(&file_name);
    download_setup(session, &vpm, &feed, &setup).await?;

    session.emit("verifying", feed.size, feed.size, "Checking the installer");
    let target = setup.clone();
    let signed = tokio::task::spawn_blocking(move || verify_signature(&target))
        .await
        .map_err(|_| "The signature check stopped unexpectedly")?;
    signed?;

    session.emit("installing", 0, 0, "Waiting for the Unity Hub installer");
    let target = setup.clone();
    let code = tokio::task::spawn_blocking(move || run_setup(&target))
        .await
        .map_err(|_| "The installer stopped unexpectedly")??;
    match code {
        0 => {}
        DECLINED_CODE => {
            return Err("Windows did not get permission to install Unity Hub. Try again and accept the prompt.".into())
        }
        other => return Err(Stop::Failed(format!("The Unity Hub installer stopped with code {other}"))),
    }
    if find_hub().is_none() {
        return Err("The installer finished, but Unity Hub was not found afterwards".into());
    }
    Ok(())
}

async fn fetch_feed(vpm: &VpmContext) -> Result<String, Stop> {
    let url = Url::parse(FEED_URL).map_err(|error| error.to_string())?;
    check_host(&url, vpm.private_hosts_blocked())
        .await
        .map_err(|error| error.to_string())?;
    let mut response = vpm
        .http
        .get(url)
        .timeout(FEED_TIMEOUT)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("Could not reach Unity's download list: {error}"))?;
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if body.len() + chunk.len() > MAX_FEED_BYTES {
            return Err("Unity's download list is too large".into());
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| Stop::Failed("Unity's download list is unreadable".to_string()))
}

async fn download_setup(
    session: &Session,
    vpm: &VpmContext,
    feed: &HubFeed,
    destination: &Path,
) -> Result<(), Stop> {
    let url = Url::parse(&format!("{FEED_BASE}{}", feed.path)).map_err(|error| error.to_string())?;
    check_host(&url, vpm.private_hosts_blocked())
        .await
        .map_err(|error| error.to_string())?;
    let mut response = vpm
        .http
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("Could not download Unity Hub: {error}"))?;
    let mut file = std::fs::File::create(destination).map_err(|error| error.to_string())?;
    let mut hasher = Sha512::new();
    let mut bytes = 0u64;
    let mut last = Instant::now() - EMIT_INTERVAL;
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if session.cancel.load(Ordering::SeqCst) {
            return Err(Stop::Cancelled);
        }
        bytes += chunk.len() as u64;
        if bytes > feed.size {
            return Err("The Unity Hub download is larger than expected".into());
        }
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(|error| error.to_string())?;
        if last.elapsed() >= EMIT_INTERVAL {
            last = Instant::now();
            session.emit("downloading", bytes, feed.size, "Downloading Unity Hub");
        }
    }
    file.sync_all().map_err(|error| error.to_string())?;
    if bytes != feed.size {
        return Err("The Unity Hub download was incomplete".into());
    }
    if hasher.finalize().as_slice() != feed.sha512.as_slice() {
        return Err("The Unity Hub download did not match its checksum and was deleted".into());
    }
    Ok(())
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

fn powershell(script: &str, file: &Path) -> Result<Command, Stop> {
    let tool = system_tool("WindowsPowerShell\\v1.0\\powershell.exe")
        .ok_or("The Windows system folder was not found")?;
    let mut command = Command::new(tool);
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("VRCC_FILE", file);
    hide_window(&mut command);
    Ok(command)
}

fn verify_signature(file: &Path) -> Result<(), Stop> {
    if !cfg!(windows) {
        return Err("Installing Unity Hub is only supported on Windows".into());
    }
    match run_streaming(powershell(SIGNATURE_SCRIPT, file)?, None, SIGNATURE_LIMIT, |_| {}) {
        Ok(RunEnd::Exited(Some(0))) => Ok(()),
        Ok(RunEnd::Exited(_)) => Err("The Unity Hub installer is not signed by Unity Technologies and was not run".into()),
        Ok(_) => Err("The signature check took too long, so the installer was not run".into()),
        Err(error) => Err(Stop::Failed(format!("Could not check the installer signature: {error}"))),
    }
}

fn run_setup(file: &Path) -> Result<i32, Stop> {
    if !cfg!(windows) {
        return Err("Installing Unity Hub is only supported on Windows".into());
    }
    match run_streaming(powershell(SETUP_SCRIPT, file)?, None, SETUP_LIMIT, |_| {}) {
        Ok(RunEnd::Exited(Some(code))) => Ok(code),
        Ok(RunEnd::Exited(None)) => Err("The Unity Hub installer ended unexpectedly".into()),
        Ok(_) => Err("The Unity Hub installer took too long".into()),
        Err(error) => Err(Stop::Failed(format!("Could not start the Unity Hub installer: {error}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = "version: 3.22.2\nfiles:\n  - url: 3.22.2/UnityHubSetup-3.22.2-x64.exe\n    sha512: EWyODdFc2c4AzBm8y3pNopoLL+p2DZQE8yI+lzaJ+fTMKH/mMLlZMgXw4/VZ0YgCyHD/Ai6ebP0OTOEiwDOEdA==\n    size: 193623232\n  - url: 3.22.2/UnityHubSetup-3.22.2-arm64.exe\n    sha512: xS9wBSrgc7owknRp9ugLhdFNviHvCPrIIaTKseIKPKgtrWbZCBufzBk/G3VxdiyYF5CkTU1xXDMhOnFzVdiirw==\n    size: 184024664\nreleaseDate: 2026-10-02T20:18:59.543Z\nsha512: ignored\n";

    #[test]
    fn the_feed_gives_the_installer_for_each_architecture() {
        let x64 = parse_feed(FEED, "x64").unwrap();
        assert_eq!(x64.path, "3.22.2/UnityHubSetup-3.22.2-x64.exe");
        assert_eq!(x64.size, 193623232);
        assert_eq!(x64.sha512.len(), 64);
        let arm = parse_feed(FEED, "arm64").unwrap();
        assert_eq!(arm.path, "3.22.2/UnityHubSetup-3.22.2-arm64.exe");
    }

    #[test]
    fn a_feed_without_the_architecture_is_rejected() {
        assert!(parse_feed(FEED, "riscv").is_err());
    }

    #[test]
    fn unexpected_installer_addresses_are_rejected() {
        for path in [
            "../evil/UnityHubSetup-1.0-x64.exe",
            "3.22.2/../UnityHubSetup-3.22.2-x64.exe",
            "https://evil.com/UnityHubSetup-3.22.2-x64.exe",
            "3.22.2/other.exe",
            "3.22.2/UnityHubSetup-3.22.2-x64.exe/extra",
            "UnityHubSetup-3.22.2-x64.exe",
            "3.22.2\\UnityHubSetup-3.22.2-x64.exe",
        ] {
            let text = format!("files:\n  - url: {path}\n    sha512: {}\n    size: 100\n", "A".repeat(86) + "==");
            assert!(parse_feed(&text, "x64").is_err(), "{path}");
        }
    }

    #[test]
    fn a_bad_size_or_checksum_is_rejected() {
        let hash = base64::engine::general_purpose::STANDARD.encode([1u8; 64]);
        let ok = format!("files:\n  - url: 3.0/UnityHubSetup-3.0-x64.exe\n    sha512: {hash}\n    size: 100\n");
        assert!(parse_feed(&ok, "x64").is_ok());
        let too_big = ok.replace("size: 100", &format!("size: {}", MAX_HUB_BYTES + 1));
        assert!(parse_feed(&too_big, "x64").is_err());
        let zero = ok.replace("size: 100", "size: 0");
        assert!(parse_feed(&zero, "x64").is_err());
        let short = ok.replace(&hash, "AAAA");
        assert!(parse_feed(&short, "x64").is_err());
        let missing = "files:\n  - url: 3.0/UnityHubSetup-3.0-x64.exe\n";
        assert!(parse_feed(missing, "x64").is_err());
    }

    #[test]
    fn only_listed_versions_are_installable() {
        assert!(recommended("2022.3.22f1").is_some());
        assert!(recommended("2019.4.31f1").is_some());
        assert!(recommended("2022.3.22f1 --evil").is_none());
        assert!(recommended("6000.0.0f1").is_none());
    }

    #[test]
    fn editor_arguments_come_from_the_list() {
        let entry = recommended("2022.3.22f1").unwrap();
        let plain = editor_args(entry, false, false);
        assert_eq!(
            plain,
            ["--", "--headless", "install", "--version", "2022.3.22f1", "--changeset", "887be4894c44"]
        );
        let android = editor_args(entry, true, false);
        assert_eq!(&android[plain.len()..], ["--module", "android", "--childModules"]);
        let ios = editor_args(entry, false, true);
        assert_eq!(&ios[plain.len()..], ["--module", "ios", "--childModules"]);
        let both = editor_args(entry, true, true);
        assert_eq!(&both[plain.len()..], ["--module", "android", "--module", "ios", "--childModules"]);
    }

    #[test]
    fn linux_runs_hub_without_a_window_and_other_systems_pass_nothing() {
        let flags = launch_flags();
        if cfg!(target_os = "linux") {
            assert_eq!(flags, ["--ozone-platform=headless"]);
        } else {
            assert!(flags.is_empty());
        }
    }

    #[test]
    fn output_lines_are_cleaned_and_shortened() {
        assert_eq!(sanitize_line("  \u{1b}[32mhello\u{7}  "), "hello");
        assert_eq!(sanitize_line("\u{1b}[10A\u{1b}[0JProgress:"), "Progress:");
        assert_eq!(sanitize_line("a\u{1b}[2Kb\u{1b}7c"), "abc");
        assert_eq!(sanitize_line("tail\u{1b}"), "tail");
        assert_eq!(sanitize_line(&"x".repeat(500)).len(), LINE_LIMIT);
        assert_eq!(sanitize_line("\r\n"), "");
    }

    #[test]
    fn hub_status_lines_give_a_name_and_a_percent() {
        let (name, status, percent) = parse_status("[Unity (2019.4.31f1)] downloading 6.99%").unwrap();
        assert_eq!(name, "Unity (2019.4.31f1)");
        assert_eq!(status, "downloading 6.99%");
        assert_eq!(percent, Some(6.99));
        assert_eq!(parse_status("[Unity (2019.4.31f1)] in progress...").unwrap().2, None);
        assert!(parse_status("Progress:").is_none());
        assert!(parse_status("[] downloading 5%").is_none());
        assert!(parse_status("[Unity] downloading 500%").unwrap().2.is_none());
    }

    #[test]
    fn overall_progress_averages_every_item_and_stays_below_a_hundred() {
        let mut progress = Progress::default();
        assert_eq!(progress.percent(), 0);
        assert_eq!(progress.update("Progress:"), None);
        assert!(progress.update("[Unity] in progress...").is_some());
        assert!(progress.update("[Android Build Support] in progress...").is_some());
        progress.update("[Unity] downloading 50%");
        assert_eq!(progress.percent(), 25);
        progress.update("[Unity] downloading 100%");
        progress.update("[Android Build Support] downloading 100%");
        assert_eq!(progress.percent(), 99);
    }

    #[cfg(unix)]
    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[cfg(unix)]
    #[test]
    fn output_lines_and_the_exit_code_are_reported() {
        let mut lines = Vec::new();
        let end = run_streaming(shell("echo one; echo two >&2; exit 3"), None, Duration::from_secs(10), |line| {
            lines.push(line.to_string())
        })
        .unwrap();
        assert!(matches!(end, RunEnd::Exited(Some(3))));
        lines.sort();
        assert_eq!(lines, ["one", "two"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_cancelled_run_is_stopped() {
        let flag = Arc::new(AtomicBool::new(false));
        let setter = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            setter.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();
        let end = run_streaming(shell("sleep 30"), Some(&flag), Duration::from_secs(60), |_| {}).unwrap();
        assert!(matches!(end, RunEnd::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[cfg(unix)]
    fn process_alive(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(unix)]
    #[test]
    fn cancelling_stops_the_programs_a_run_started_too() {
        let flag = Arc::new(AtomicBool::new(false));
        let setter = flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            setter.store(true, Ordering::SeqCst);
        });
        let mut pid = String::new();
        let end = run_streaming(shell("sleep 60 & echo $!; wait"), Some(&flag), Duration::from_secs(60), |line| {
            pid = line.trim().to_string()
        })
        .unwrap();
        assert!(matches!(end, RunEnd::Cancelled));
        assert!(!pid.is_empty());
        let mut gone = false;
        for _ in 0..30 {
            if !process_alive(&pid) {
                gone = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(gone, "process {pid} is still running");
    }

    #[cfg(unix)]
    #[test]
    fn the_time_limit_stops_the_programs_a_run_started_too() {
        let mut pid = String::new();
        let end = run_streaming(shell("sleep 60 & echo $!; wait"), None, Duration::from_millis(600), |line| {
            pid = line.trim().to_string()
        })
        .unwrap();
        assert!(matches!(end, RunEnd::TimedOut));
        let mut gone = false;
        for _ in 0..30 {
            if !process_alive(&pid) {
                gone = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(gone, "process {pid} is still running");
    }

    #[test]
    fn outcomes_are_described_for_every_ending() {
        assert_eq!(outcome_text("editor", "2022.3.22f1", &Ok(())).1, "Unity 2022.3.22f1 is installed");
        assert_eq!(outcome_text("hub", "", &Ok(())).1, "Unity Hub is installed");
        assert_eq!(outcome_text("hub", "", &Err(Stop::Cancelled)).0, "cancelled");
        let failed = outcome_text("editor", "x", &Err(Stop::Failed("It broke".to_string())));
        assert_eq!((failed.0, failed.1.as_str(), failed.2), ("failed", "It broke", "warning"));
    }

    #[test]
    fn system_tools_come_from_the_windows_folder_only() {
        let found = system_tool("taskkill.exe");
        if cfg!(windows) {
            assert!(found.unwrap().ends_with("System32\\taskkill.exe"));
        } else {
            assert!(found.is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_run_past_its_limit_is_stopped() {
        let end = run_streaming(shell("sleep 30"), None, Duration::from_millis(300), |_| {}).unwrap();
        assert!(matches!(end, RunEnd::TimedOut));
    }
}
