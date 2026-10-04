use crate::applog::log;
use crate::prefs::Prefs;
use crate::vpm::{VpmContext, CURATED_ID, LOCAL_USER_ID, OFFICIAL_ID};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use vrc_get_vpm::environment::{add_remote_repo, AddRepositoryErr, AddUserPackageResult, Settings};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::repository::RemoteRepository;
use vrc_get_vpm::VersionSelector;

const OFFICIAL_URL: &str = "https://packages.vrchat.com/official?download";
const CURATED_URL: &str = "https://packages.vrchat.com/curated?download";
const MAX_HEADERS: usize = 10;
const MAX_REPOSITORY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Deserialize)]
pub struct HeaderInput {
    name: String,
    value: String,
}

fn header_map(headers: &[HeaderInput]) -> Result<IndexMap<Box<str>, Box<str>>, String> {
    if headers.len() > MAX_HEADERS {
        return Err("Too many headers".to_string());
    }
    let mut map = IndexMap::new();
    for header in headers {
        let name = header.name.trim();
        if reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err()
            || reqwest::header::HeaderValue::from_str(&header.value).is_err()
        {
            return Err("A header name or value is not valid".to_string());
        }
        map.insert(name.into(), header.value.as_str().into());
    }
    Ok(map)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinRepository {
    id: &'static str,
    name: &'static str,
    hidden: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityRepository {
    index: usize,
    id: String,
    url: Option<String>,
    name: String,
    author: Option<String>,
    hidden: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoriesInfo {
    builtin: Vec<BuiltinRepository>,
    community: Vec<CommunityRepository>,
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RepositoryPreview {
    BadUrl,
    Duplicated { by_url: bool, name: String },
    DownloadError { message: String },
    Success {
        id: String,
        url: String,
        name: String,
        author: Option<String>,
        packages: Vec<String>,
    },
}

fn author_of(map: &Map<String, Value>) -> Option<String> {
    match map.get("author")? {
        Value::String(name) if !name.is_empty() => Some(name.clone()),
        Value::Object(object) => object
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

fn cached_author(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')).ok()? {
        Value::Object(map) => author_of(&map),
        _ => None,
    }
}

fn parse_http_url(url: &str) -> Option<reqwest::Url> {
    let parsed: reqwest::Url = url.trim().parse().ok()?;
    match parsed.scheme() {
        "http" | "https" if parsed.host_str().is_some() => Some(parsed),
        _ => None,
    }
}

fn lock_prefs<'a>(
    state: &'a State<'_, Mutex<Prefs>>,
) -> Result<std::sync::MutexGuard<'a, Prefs>, String> {
    state
        .lock()
        .map_err(|_| "Preferences are unavailable".to_string())
}

#[tauri::command]
pub async fn repositories_info(
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<RepositoriesInfo, String> {
    let settings = {
        let _guard = vpm.lock.lock().await;
        Settings::load(&vpm.io)
            .await
            .map_err(|error| format!("Could not read repository settings: {error}"))?
    };
    let prefs = lock_prefs(&prefs)?;

    let builtin = vec![
        BuiltinRepository {
            id: OFFICIAL_ID,
            name: "Official",
            hidden: prefs.is_hidden(OFFICIAL_ID),
        },
        BuiltinRepository {
            id: CURATED_ID,
            name: "Curated",
            hidden: prefs.is_hidden(CURATED_ID),
        },
        BuiltinRepository {
            id: LOCAL_USER_ID,
            name: "Local User Packages",
            hidden: prefs.is_hidden(LOCAL_USER_ID),
        },
    ];

    let community = settings
        .get_user_repos()
        .iter()
        .enumerate()
        .filter(|(_, repo)| repo.id().is_some() || repo.url().is_some())
        .map(|(index, repo)| {
            let id = repo
                .id()
                .or(repo.url().map(|url| url.as_str()))
                .unwrap_or_default()
                .to_string();
            CommunityRepository {
                index,
                name: repo.name().unwrap_or(&id).to_string(),
                url: repo.url().map(|url| url.to_string()),
                author: cached_author(repo.local_path()),
                hidden: prefs.is_hidden(&id),
                id,
            }
        })
        .collect();

    Ok(RepositoriesInfo { builtin, community })
}

#[tauri::command]
pub async fn repository_preview(
    url: String,
    headers: Vec<HeaderInput>,
    vpm: State<'_, VpmContext>,
) -> Result<RepositoryPreview, String> {
    let Some(parsed) = parse_http_url(&url) else {
        return Ok(RepositoryPreview::BadUrl);
    };

    let (known_urls, known_ids) = {
        let _guard = vpm.lock.lock().await;
        let settings = Settings::load(&vpm.io)
            .await
            .map_err(|error| format!("Could not read repository settings: {error}"))?;
        let mut urls: Vec<(String, String)> = vec![
            (OFFICIAL_URL.to_string(), "Official".to_string()),
            (CURATED_URL.to_string(), "Curated".to_string()),
        ];
        let mut ids: Vec<(String, String)> = vec![
            (OFFICIAL_ID.to_string(), "Official".to_string()),
            (CURATED_ID.to_string(), "Curated".to_string()),
        ];
        for repo in settings.get_user_repos() {
            let label = repo
                .name()
                .or(repo.id())
                .unwrap_or("Existing repository")
                .to_string();
            if let Some(url) = repo.url() {
                urls.push((url.to_string(), label.clone()));
            }
            if let Some(id) = repo.id() {
                ids.push((id.to_string(), label));
            }
        }
        (urls, ids)
    };

    if let Some((_, name)) = known_urls.iter().find(|(known, _)| known == parsed.as_str()) {
        return Ok(RepositoryPreview::Duplicated {
            by_url: true,
            name: name.clone(),
        });
    }

    let header_values = match header_map(&headers) {
        Ok(map) => map,
        Err(message) => return Ok(RepositoryPreview::DownloadError { message }),
    };
    let mut request = vpm.http.get(parsed.clone());
    for (name, value) in &header_values {
        request = request.header(name.as_ref(), value.as_ref());
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(RepositoryPreview::DownloadError {
                message: error.to_string(),
            })
        }
    };
    if !response.status().is_success() {
        return Ok(RepositoryPreview::DownloadError {
            message: format!("The server responded with {}", response.status()),
        });
    }
    let bytes = match response.bytes().await {
        Ok(bytes) if bytes.len() <= MAX_REPOSITORY_BYTES => bytes,
        Ok(_) => {
            return Ok(RepositoryPreview::DownloadError {
                message: "The repository listing is too large".to_string(),
            })
        }
        Err(error) => {
            return Ok(RepositoryPreview::DownloadError {
                message: error.to_string(),
            })
        }
    };

    let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let map = match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) => map,
        _ => {
            return Ok(RepositoryPreview::DownloadError {
                message: "The URL did not return a valid VPM repository listing".to_string(),
            })
        }
    };
    let author = author_of(&map);
    let repository = match RemoteRepository::parse(map) {
        Ok(repository) => repository,
        Err(error) => {
            return Ok(RepositoryPreview::DownloadError {
                message: format!("The repository listing is invalid: {error}"),
            })
        }
    };

    let url_text = repository
        .url()
        .map(|url| url.to_string())
        .unwrap_or_else(|| parsed.to_string());
    let id = repository
        .id()
        .map(str::to_string)
        .unwrap_or_else(|| url_text.clone());

    if let Some((_, name)) = known_ids.iter().find(|(known, _)| *known == id) {
        return Ok(RepositoryPreview::Duplicated {
            by_url: false,
            name: name.clone(),
        });
    }

    let mut packages: Vec<String> = repository
        .get_packages()
        .filter_map(|package| package.get_latest(VersionSelector::latest_for(None, true)))
        .map(|manifest| {
            manifest
                .display_name()
                .unwrap_or_else(|| manifest.name())
                .to_string()
        })
        .collect();
    packages.sort_by_key(|name| name.to_lowercase());

    Ok(RepositoryPreview::Success {
        name: repository.name().unwrap_or(&id).to_string(),
        id,
        url: url_text,
        author,
        packages,
    })
}

#[tauri::command]
pub async fn repository_add(
    app: AppHandle,
    url: String,
    headers: Vec<HeaderInput>,
    vpm: State<'_, VpmContext>,
) -> Result<(), String> {
    let parsed = parse_http_url(&url).ok_or_else(|| "The URL is not valid".to_string())?;
    let parsed_label = parsed.to_string();
    let _busy = vpm.try_busy()?;
    let _guard = vpm.lock.lock().await;
    let mut settings = Settings::load(&vpm.io)
        .await
        .map_err(|error| format!("Could not read repository settings: {error}"))?;
    add_remote_repo(&mut settings, parsed, None, header_map(&headers)?, &vpm.io, &vpm.http)
        .await
        .map_err(|error| format!("Could not add the repository: {error}"))?;
    settings
        .save(&vpm.io)
        .await
        .map_err(|error| format!("Could not save repository settings: {error}"))?;
    vpm.invalidate().await;
    log(&app, "success", format!("Added repository {parsed_label}"));
    Ok(())
}

#[tauri::command]
pub async fn repository_remove(
    app: AppHandle,
    index: usize,
    expected_id: String,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    {
        let _guard = vpm.lock.lock().await;
        let mut settings = Settings::load(&vpm.io)
            .await
            .map_err(|error| format!("Could not read repository settings: {error}"))?;
        let actual = settings
            .get_user_repos()
            .get(index)
            .map(|repo| repo.id().or(repo.url().map(|url| url.as_str())).map(str::to_string));
        if actual.flatten().as_deref() != Some(expected_id.as_str()) {
            return Err("The repository list changed. Refresh and try again.".to_string());
        }
        settings.remove_repo_at_index(index);
        settings
            .save(&vpm.io)
            .await
            .map_err(|error| format!("Could not save repository settings: {error}"))?;
    }
    vpm.invalidate().await;
    log(&app, "info", format!("Removed repository {expected_id}"));
    let mut prefs = lock_prefs(&prefs)?;
    prefs.set_hidden(&expected_id, false);
    prefs.save().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn repository_set_hidden(
    id: String,
    hidden: bool,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<(), String> {
    let _busy = vpm.try_busy()?;
    let mut prefs = lock_prefs(&prefs)?;
    prefs.set_hidden(&id, hidden);
    prefs.save().map_err(|error| error.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageImportResult {
    repositories_found: usize,
    repositories_added: usize,
    repositories_skipped: usize,
    folders_found: usize,
    folders_added: usize,
    failures: Vec<String>,
}

#[derive(Serialize, Clone)]
struct ImportProgress {
    done: usize,
    total: usize,
}

struct RepositoryCandidate {
    url: Option<reqwest::Url>,
    headers: IndexMap<Box<str>, Box<str>>,
    name: Option<String>,
    path: std::path::PathBuf,
}

#[tauri::command]
pub async fn import_packages_from_vcc(
    app: AppHandle,
    vpm: State<'_, VpmContext>,
) -> Result<PackageImportResult, String> {
    let _busy = vpm.try_busy()?;
    let vcc_io = DefaultEnvironmentIo::new_default();
    let settings_file = vcc_io.resolve(std::path::Path::new("settings.json"));
    if !settings_file.is_file() {
        return Err(format!(
            "No VCC settings found at {}",
            settings_file.display()
        ));
    }

    let (candidates, folders): (Vec<RepositoryCandidate>, Vec<std::path::PathBuf>) = {
        let vcc_settings = Settings::load(&vcc_io)
            .await
            .map_err(|error| format!("Could not read the VCC settings: {error}"))?;
        (
            vcc_settings
                .get_user_repos()
                .iter()
                .map(|repo| RepositoryCandidate {
                    url: repo.url().cloned(),
                    headers: repo.headers().clone(),
                    name: repo.name().map(str::to_string),
                    path: repo.local_path().to_path_buf(),
                })
                .collect(),
            vcc_settings.user_package_folders().to_vec(),
        )
    };

    let total = candidates.len() + folders.len();
    let mut done = 0;
    let mut result = PackageImportResult {
        repositories_found: candidates.len(),
        repositories_added: 0,
        repositories_skipped: 0,
        folders_found: folders.len(),
        folders_added: 0,
        failures: Vec::new(),
    };
    let _ = app.emit("vcc-import-progress", ImportProgress { done, total });

    let _guard = vpm.lock.lock().await;
    let mut settings = Settings::load(&vpm.io)
        .await
        .map_err(|error| format!("Could not read repository settings: {error}"))?;

    for candidate in candidates {
        let label = candidate
            .name
            .clone()
            .or_else(|| candidate.url.as_ref().map(|url| url.to_string()))
            .unwrap_or_else(|| candidate.path.display().to_string());
        match candidate.url {
            Some(url) => {
                let outcome = add_remote_repo(
                    &mut settings,
                    url,
                    candidate.name.as_deref(),
                    candidate.headers,
                    &vpm.io,
                    &vpm.http,
                )
                .await;
                match outcome {
                    Ok(()) => result.repositories_added += 1,
                    Err(AddRepositoryErr::AlreadyAdded) => result.repositories_skipped += 1,
                    Err(error) => result.failures.push(format!("{label}: {error}")),
                }
            }
            None => {
                if settings.add_local_repo(&candidate.path, candidate.name.as_deref()) {
                    result.repositories_added += 1;
                } else {
                    result.repositories_skipped += 1;
                }
            }
        }
        done += 1;
        let _ = app.emit("vcc-import-progress", ImportProgress { done, total });
    }

    for folder in folders {
        match settings.add_user_package(&folder, &vpm.io).await {
            AddUserPackageResult::Success => result.folders_added += 1,
            AddUserPackageResult::AlreadyAdded => {}
            AddUserPackageResult::NonAbsolute | AddUserPackageResult::BadPackage => result
                .failures
                .push(format!("{}: not a valid package folder", folder.display())),
        }
        done += 1;
        let _ = app.emit("vcc-import-progress", ImportProgress { done, total });
    }

    if result.repositories_added > 0 || result.folders_added > 0 {
        settings
            .save(&vpm.io)
            .await
            .map_err(|error| format!("Could not save repository settings: {error}"))?;
        vpm.invalidate().await;
    }
    log(
        &app,
        "success",
        format!(
            "Imported from VCC: {} of {} repositories and {} of {} package folders added",
            result.repositories_added, result.repositories_found, result.folders_added, result.folders_found
        ),
    );
    for failure in &result.failures {
        log(&app, "warning", failure.clone());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(name: &str, value: &str) -> HeaderInput {
        HeaderInput {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn valid_headers_are_trimmed_and_kept_in_order() {
        let map = header_map(&[header(" Authorization ", "Bearer abc"), header("X-Token", "1")]).unwrap();
        let keys: Vec<&str> = map.keys().map(|key| key.as_ref()).collect();
        assert_eq!(keys, vec!["Authorization", "X-Token"]);
        assert_eq!(map.get("Authorization").map(|value| value.as_ref()), Some("Bearer abc"));
    }

    #[test]
    fn invalid_header_names_and_values_are_rejected() {
        assert!(header_map(&[header("bad name", "x")]).is_err());
        assert!(header_map(&[header("", "x")]).is_err());
        assert!(header_map(&[header("X-Ok", "line\nbreak")]).is_err());
    }

    #[test]
    fn too_many_headers_are_rejected() {
        let many: Vec<HeaderInput> = (0..11).map(|index| header(&format!("X-{index}"), "v")).collect();
        assert!(header_map(&many).is_err());
        assert!(header_map(&many[..10]).is_ok());
    }

    #[test]
    fn only_http_and_https_urls_with_a_host_are_accepted() {
        assert!(parse_http_url("https://example.github.io/vpm/index.json").is_some());
        assert!(parse_http_url("  http://example.com/x  ").is_some());
        assert!(parse_http_url("ftp://example.com/x").is_none());
        assert!(parse_http_url("file:///etc/passwd").is_none());
        assert!(parse_http_url("not a url").is_none());
        assert!(parse_http_url("").is_none());
    }

    #[test]
    fn the_author_can_be_a_string_or_an_object() {
        let string: Map<String, Value> = serde_json::from_str(r#"{"author":"Someone"}"#).unwrap();
        assert_eq!(author_of(&string).as_deref(), Some("Someone"));
        let object: Map<String, Value> = serde_json::from_str(r#"{"author":{"name":"Named","url":"x"}}"#).unwrap();
        assert_eq!(author_of(&object).as_deref(), Some("Named"));
        let empty: Map<String, Value> = serde_json::from_str(r#"{"author":""}"#).unwrap();
        assert_eq!(author_of(&empty), None);
        let missing: Map<String, Value> = serde_json::from_str("{}").unwrap();
        assert_eq!(author_of(&missing), None);
    }
}