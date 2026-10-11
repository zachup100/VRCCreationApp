use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

const SCHEME: &str = "vcc";
const MAX_HEADERS: usize = 10;
const MAX_PENDING: usize = 5;

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct LinkHeader {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub struct AddRepositoryLink {
    pub url: String,
    pub headers: Vec<LinkHeader>,
}

#[derive(Default)]
pub struct PendingLinks(Mutex<Vec<AddRepositoryLink>>);

impl PendingLinks {
    fn push(&self, link: AddRepositoryLink) {
        if let Ok(mut pending) = self.0.lock() {
            if pending.len() >= MAX_PENDING {
                pending.remove(0);
            }
            pending.push(link);
        }
    }

    fn take_oldest(&self) -> Option<AddRepositoryLink> {
        let mut links = self.0.lock().ok()?;
        if links.is_empty() {
            None
        } else {
            Some(links.remove(0))
        }
    }

    fn count(&self) -> usize {
        self.0.lock().map(|links| links.len()).unwrap_or(0)
    }
}

pub fn parse_add_repository(link: &str) -> Option<AddRepositoryLink> {
    let parsed: reqwest::Url = link.trim().parse().ok()?;
    if !parsed.scheme().eq_ignore_ascii_case(SCHEME) {
        return None;
    }
    if parsed.host_str().map(str::to_ascii_lowercase).as_deref() != Some("vpm") {
        return None;
    }
    if parsed.path().trim_end_matches('/') != "/addRepo" {
        return None;
    }

    let mut url: Option<String> = None;
    let mut headers = Vec::new();
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "url" => {
                if url.is_some() {
                    return None;
                }
                let target: reqwest::Url = value.parse().ok()?;
                if !matches!(target.scheme(), "http" | "https") || target.host_str().is_none() {
                    return None;
                }
                url = Some(target.to_string());
            }
            "headers[]" => {
                let (name, header_value) = value.split_once(':')?;
                let name = name.trim();
                if name.is_empty() || headers.len() >= MAX_HEADERS {
                    return None;
                }
                headers.push(LinkHeader {
                    name: name.to_string(),
                    value: header_value.trim_start().to_string(),
                });
            }
            _ => {}
        }
    }
    Some(AddRepositoryLink { url: url?, headers })
}

pub fn handle_args<I: IntoIterator<Item = String>>(app: &AppHandle, args: I) {
    let prefix = format!("{SCHEME}://");
    for arg in args {
        if !arg.to_ascii_lowercase().starts_with(&prefix) {
            continue;
        }
        if let Some(link) = parse_add_repository(&arg) {
            app.state::<PendingLinks>().push(link);
            let _ = app.emit("deep-link-add-repository", ());
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeepLinkStatus {
    supported: bool,
    registered: bool,
    other_handler: bool,
}

#[cfg(windows)]
fn other_handler_registered(app: &AppHandle) -> bool {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;
    let command: Result<String, _> = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(format!("{SCHEME}\\shell\\open\\command"))
        .and_then(|key| key.get_value(""));
    let _ = app;
    command.is_ok()
}

#[cfg(not(windows))]
fn other_handler_registered(_app: &AppHandle) -> bool {
    false
}

fn status(app: &AppHandle) -> DeepLinkStatus {
    let supported = cfg!(any(windows, target_os = "linux"));
    if !supported {
        return DeepLinkStatus {
            supported,
            registered: false,
            other_handler: false,
        };
    }
    let registered = app.deep_link().is_registered(SCHEME).unwrap_or(false);
    DeepLinkStatus {
        supported,
        registered,
        other_handler: !registered && other_handler_registered(app),
    }
}

#[tauri::command]
pub async fn deep_link_status(app: AppHandle) -> Result<DeepLinkStatus, String> {
    tokio::task::spawn_blocking(move || status(&app))
        .await
        .map_err(|_| "Could not read the link registration".to_string())
}

#[tauri::command]
pub async fn deep_link_register(app: AppHandle) -> Result<DeepLinkStatus, String> {
    tokio::task::spawn_blocking(move || {
        if !cfg!(any(windows, target_os = "linux")) {
            return Err("Link handling is not supported on this system".to_string());
        }
        app.deep_link()
            .register(SCHEME)
            .map_err(|error| format!("Could not register the link handler: {error}"))?;
        let current = status(&app);
        if !current.registered {
            return Err("The link handler was written but does not point to this app".to_string());
        }
        Ok(current)
    })
    .await
    .map_err(|_| "Registering the link handler was interrupted".to_string())?
}

#[tauri::command]
pub async fn deep_link_unregister(app: AppHandle) -> Result<DeepLinkStatus, String> {
    tokio::task::spawn_blocking(move || {
        if !status(&app).registered {
            return Err("This app is not the registered handler, so nothing was removed".to_string());
        }
        app.deep_link()
            .unregister(SCHEME)
            .map_err(|error| format!("Could not remove the link handler: {error}"))?;
        Ok(status(&app))
    })
    .await
    .map_err(|_| "Removing the link handler was interrupted".to_string())?
}

#[tauri::command]
pub fn deep_link_take_add_repository(pending: tauri::State<'_, PendingLinks>) -> Option<AddRepositoryLink> {
    pending.take_oldest()
}

#[tauri::command]
pub fn deep_link_pending_count(pending: tauri::State<'_, PendingLinks>) -> usize {
    pending.count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(name: &str) -> AddRepositoryLink {
        parse_add_repository(&format!("vcc://vpm/addRepo?url=https%3A%2F%2F{name}.com")).unwrap()
    }

    #[test]
    fn links_are_taken_oldest_first_and_counted() {
        let pending = PendingLinks::default();
        for name in ["a", "b", "c"] {
            pending.push(link(name));
        }
        assert_eq!(pending.count(), 3);
        assert_eq!(pending.take_oldest().unwrap().url, "https://a.com/");
        assert_eq!(pending.take_oldest().unwrap().url, "https://b.com/");
        assert_eq!(pending.count(), 1);
    }

    #[test]
    fn the_queue_drops_the_oldest_link_when_full() {
        let pending = PendingLinks::default();
        for index in 0..=MAX_PENDING {
            pending.push(link(&format!("site{index}")));
        }
        assert_eq!(pending.count(), MAX_PENDING);
        assert_eq!(pending.take_oldest().unwrap().url, "https://site1.com/");
    }

    #[test]
    fn a_plain_add_repo_link_is_read() {
        let link = parse_add_repository("vcc://vpm/addRepo?url=https%3A%2F%2Fexample.com%2Findex.json").unwrap();
        assert_eq!(link.url, "https://example.com/index.json");
        assert!(link.headers.is_empty());
    }

    #[test]
    fn headers_are_split_at_the_first_colon() {
        let link = parse_add_repository(
            "vcc://vpm/addRepo?url=https%3A%2F%2Fexample.com%2Fi.json&headers[]=Authorization%3ABearer%20a%3Ab&headers[]=X-Key%3A1",
        )
        .unwrap();
        assert_eq!(link.headers.len(), 2);
        assert_eq!(link.headers[0].name, "Authorization");
        assert_eq!(link.headers[0].value, "Bearer a:b");
        assert_eq!(link.headers[1].name, "X-Key");
    }

    #[test]
    fn other_schemes_hosts_and_paths_are_rejected() {
        assert!(parse_add_repository("https://vpm/addRepo?url=https%3A%2F%2Fa.com").is_none());
        assert!(parse_add_repository("vcc://other/addRepo?url=https%3A%2F%2Fa.com").is_none());
        assert!(parse_add_repository("vcc://vpm/removeRepo?url=https%3A%2F%2Fa.com").is_none());
        assert!(parse_add_repository("not a link").is_none());
    }

    #[test]
    fn bad_or_missing_urls_are_rejected() {
        assert!(parse_add_repository("vcc://vpm/addRepo").is_none());
        assert!(parse_add_repository("vcc://vpm/addRepo?url=file%3A%2F%2F%2Fetc%2Fpasswd").is_none());
        assert!(parse_add_repository("vcc://vpm/addRepo?url=javascript%3Aalert(1)").is_none());
        assert!(parse_add_repository(
            "vcc://vpm/addRepo?url=https%3A%2F%2Fa.com&url=https%3A%2F%2Fb.com"
        )
        .is_none());
    }

    #[test]
    fn a_header_without_a_colon_or_too_many_headers_is_rejected() {
        assert!(parse_add_repository("vcc://vpm/addRepo?url=https%3A%2F%2Fa.com&headers[]=nocolon").is_none());
        let many: String = (0..11).map(|i| format!("&headers[]=H{i}%3Av")).collect();
        assert!(parse_add_repository(&format!("vcc://vpm/addRepo?url=https%3A%2F%2Fa.com{many}")).is_none());
    }
}
