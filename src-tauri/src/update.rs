use crate::applog::log;
use crate::limits::check_host;
use crate::prefs::Prefs;
use crate::store::now_millis;
use crate::vpm::VpmContext;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, State};

const RELEASES_URL: &str = "https://api.github.com/repos/zachup100/VRCCreationApp/releases/latest";
const RELEASE_PREFIX: &str = "https://github.com/zachup100/VRCCreationApp/";
const API_HOST: &str = "api.github.com";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_REDIRECTS: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(20);
const DAY_MILLIS: i64 = 24 * 60 * 60 * 1000;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub current: String,
    pub latest: Option<String>,
    pub available: bool,
    pub url: Option<String>,
    pub checked_at: i64,
}

#[derive(Debug, PartialEq)]
pub struct Release {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct ReleaseJson {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

pub fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let text = text.strip_prefix('v').unwrap_or(text);
    let mut parts = text.split('.');
    let mut next = || -> Option<u32> {
        let part = parts.next()?;
        if part.is_empty() || part.len() > 9 || !part.chars().all(|character| character.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let result = (next()?, next()?, next()?);
    if parts.next().is_some() {
        return None;
    }
    Some(result)
}

pub fn parse_release(json: &str) -> Result<Option<Release>, String> {
    let release: ReleaseJson =
        serde_json::from_str(json).map_err(|_| "GitHub sent a reply this app could not read".to_string())?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
    if parse_version(version).is_none() {
        return Err("The latest release has a version number this app does not understand".to_string());
    }
    if !release.html_url.starts_with(RELEASE_PREFIX) {
        return Err("The latest release points somewhere unexpected".to_string());
    }
    Ok(Some(Release {
        version: version.to_string(),
        url: release.html_url,
    }))
}

pub fn status_for(current: &str, latest: Option<&str>, url: Option<&str>, checked_at: i64) -> UpdateStatus {
    let available = match (parse_version(current), latest.and_then(parse_version)) {
        (Some(have), Some(newest)) => newest > have,
        _ => false,
    };
    UpdateStatus {
        current: current.to_string(),
        latest: latest.filter(|text| !text.is_empty()).map(str::to_string),
        available,
        url: if available { url.filter(|text| !text.is_empty()).map(str::to_string) } else { None },
        checked_at,
    }
}

pub fn should_check(manual: bool, disabled: bool, last: i64, now: i64) -> bool {
    if manual {
        return true;
    }
    !disabled && (last <= 0 || now < last || now - last >= DAY_MILLIS)
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if url.scheme() == "https" && url.host_str() == Some(API_HOST) && attempt.previous().len() < MAX_REDIRECTS {
                attempt.follow()
            } else {
                attempt.error("The update check was redirected somewhere unexpected")
            }
        }))
        .user_agent(concat!("VRCCreationApp/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(TIMEOUT)
        .build()
        .map_err(|error| error.to_string())
}

async fn fetch_latest(blocked: bool) -> Result<Option<Release>, String> {
    let url = Url::parse(RELEASES_URL).map_err(|error| error.to_string())?;
    check_host(&url, blocked).await.map_err(|error| error.to_string())?;
    let mut response = client()?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| format!("Could not reach GitHub: {error}"))?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("GitHub is limiting requests right now. Try again later.".to_string());
    }
    if !status.is_success() {
        return Err(format!("GitHub answered with an error ({})", status.as_u16()));
    }
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if body.len() + chunk.len() > MAX_BYTES {
            return Err("GitHub sent a reply that is too large".to_string());
        }
        body.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(body).map_err(|_| "GitHub sent a reply this app could not read".to_string())?;
    parse_release(&text)
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    manual: bool,
    vpm: State<'_, VpmContext>,
    prefs: State<'_, Mutex<Prefs>>,
) -> Result<UpdateStatus, String> {
    let current = app.package_info().version.to_string();
    let (disabled, last, cached_version, cached_url) = {
        let guard = prefs.lock().map_err(|_| "Preferences are unavailable".to_string())?;
        let (last, version, url) = guard.update_cache();
        (guard.settings().update_check_disabled, last, version, url)
    };
    if !should_check(manual, disabled, last, now_millis()) {
        return Ok(status_for(&current, Some(&cached_version), Some(&cached_url), last));
    }
    match fetch_latest(vpm.private_hosts_blocked()).await {
        Ok(found) => {
            let now = now_millis();
            let (version, url) = found
                .map(|release| (release.version, release.url))
                .unwrap_or_default();
            {
                let mut guard = prefs.lock().map_err(|_| "Preferences are unavailable".to_string())?;
                guard.set_update_cache(now, &version, &url);
                guard.save().map_err(|error| error.to_string())?;
            }
            Ok(status_for(&current, Some(&version), Some(&url), now))
        }
        Err(message) if manual => Err(message),
        Err(message) => {
            log(&app, "warning", format!("The automatic update check failed: {message}"));
            Ok(status_for(&current, Some(&cached_version), Some(&cached_url), last))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, url: &str, draft: bool, prerelease: bool) -> String {
        format!(
            "{{\"tag_name\":\"{tag}\",\"html_url\":\"{url}\",\"draft\":{draft},\"prerelease\":{prerelease},\"body\":\"notes\",\"assets\":[]}}"
        )
    }

    const GOOD_URL: &str = "https://github.com/zachup100/VRCCreationApp/releases/tag/v1.2.0";

    #[test]
    fn versions_are_three_plain_numbers() {
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("v1.12.3"), Some((1, 12, 3)));
        for bad in ["", "1", "1.0", "1.0.0.0", "1.0.0-beta", "a.b.c", "1..0", "1.0.x", "v", "1.0.0 ", "-1.0.0", "1234567890.0.0"] {
            assert_eq!(parse_version(bad), None, "{bad}");
        }
    }

    #[test]
    fn newer_means_a_higher_number_in_order() {
        let newer = |current: &str, latest: &str| status_for(current, Some(latest), Some(GOOD_URL), 1).available;
        assert!(newer("1.0.0", "1.0.1"));
        assert!(newer("1.0.9", "1.1.0"));
        assert!(newer("1.9.9", "2.0.0"));
        assert!(newer("1.2.0", "1.10.0"));
        assert!(!newer("1.0.0", "1.0.0"));
        assert!(!newer("1.1.0", "1.0.9"));
        assert!(!newer("2.0.0", "1.99.99"));
        assert!(!newer("1.0.0", "garbage"));
        assert!(!newer("garbage", "1.0.0"));
    }

    #[test]
    fn the_link_is_only_offered_when_an_update_is_available() {
        let behind = status_for("1.0.0", Some("1.2.0"), Some(GOOD_URL), 5);
        assert!(behind.available);
        assert_eq!(behind.url.as_deref(), Some(GOOD_URL));
        assert_eq!(behind.latest.as_deref(), Some("1.2.0"));
        let current = status_for("1.2.0", Some("1.2.0"), Some(GOOD_URL), 5);
        assert!(!current.available);
        assert_eq!(current.url, None);
        let none = status_for("1.0.0", Some(""), Some(""), 0);
        assert!(!none.available && none.latest.is_none() && none.url.is_none());
    }

    #[test]
    fn a_good_release_is_read() {
        let parsed = parse_release(&release("v1.2.0", GOOD_URL, false, false)).unwrap().unwrap();
        assert_eq!(parsed, Release { version: "1.2.0".to_string(), url: GOOD_URL.to_string() });
        let plain = parse_release(&release("1.2.0", GOOD_URL, false, false)).unwrap().unwrap();
        assert_eq!(plain.version, "1.2.0");
    }

    #[test]
    fn drafts_and_prereleases_are_ignored() {
        assert_eq!(parse_release(&release("v9.0.0", GOOD_URL, true, false)).unwrap(), None);
        assert_eq!(parse_release(&release("v9.0.0", GOOD_URL, false, true)).unwrap(), None);
    }

    #[test]
    fn unexpected_releases_are_refused() {
        for url in [
            "https://evil.com/releases",
            "http://github.com/zachup100/VRCCreationApp/releases/tag/v1.2.0",
            "https://github.com/zachup100/other/releases/tag/v1.2.0",
            "https://github.com.evil.com/zachup100/VRCCreationApp/",
            "javascript:alert(1)",
            "",
        ] {
            assert!(parse_release(&release("v1.2.0", url, false, false)).is_err(), "{url}");
        }
        for tag in ["latest", "v1.2", "v1.2.0-rc1", "", "v1.2.0 ; rm"] {
            assert!(parse_release(&release(tag, GOOD_URL, false, false)).is_err(), "{tag}");
        }
        for text in ["", "[]", "null", "{}", "not json", "{\"tag_name\":5}"] {
            assert!(parse_release(text).is_err(), "{text}");
        }
    }

    #[test]
    fn automatic_checks_run_at_most_once_a_day_and_never_when_switched_off() {
        let day = DAY_MILLIS;
        assert!(should_check(false, false, 0, 1000));
        assert!(!should_check(false, false, 1000, 1000 + day - 1));
        assert!(should_check(false, false, 1000, 1000 + day));
        assert!(should_check(false, false, 5000, 100));
        assert!(!should_check(false, true, 0, 1000));
        assert!(!should_check(false, true, 1000, 1000 + 10 * day));
        assert!(should_check(true, true, 1000, 1001));
        assert!(should_check(true, false, 1000, 1001));
    }
}
