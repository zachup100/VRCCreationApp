use futures::io::AsyncRead;
use indexmap::IndexMap;
use crate::hosts::ensure_allowed;
use pin_project_lite::pin_project;
use reqwest::Url;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use vrc_get_vpm::HttpClient;

pub const MAX_REPOSITORY_BYTES: usize = 24 * 1024 * 1024;
pub const MAX_PACKAGE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const REPOSITORY_TIMEOUT: Duration = Duration::from_secs(60);
pub const TOO_LARGE_MESSAGE: &str = "The repository listing is too large";
pub const PACKAGE_TOO_LARGE_MESSAGE: &str = "The package download is too large";

pub async fn read_capped(mut response: reqwest::Response) -> Result<Vec<u8>, String> {
    if let Some(length) = response.content_length() {
        if length > MAX_REPOSITORY_BYTES as u64 {
            return Err(TOO_LARGE_MESSAGE.to_string());
        }
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if body.len() + chunk.len() > MAX_REPOSITORY_BYTES {
            return Err(TOO_LARGE_MESSAGE.to_string());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pin_project! {
    pub struct CappedRead<R> {
        #[pin]
        inner: R,
        read: u64,
        cap: u64,
        started: Instant,
        deadline: Duration,
        message: &'static str,
    }
}

impl<R> CappedRead<R> {
    pub fn new(inner: R, cap: u64, deadline: Duration, message: &'static str) -> Self {
        Self {
            inner,
            read: 0,
            cap,
            started: Instant::now(),
            deadline,
            message,
        }
    }
}

impl<R: AsyncRead> AsyncRead for CappedRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        if this.started.elapsed() > *this.deadline {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "The repository download took too long",
            )));
        }
        let result = this.inner.poll_read(cx, buf);
        if let Poll::Ready(Ok(count)) = &result {
            *this.read += *count as u64;
            if *this.read > *this.cap {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    *this.message,
                )));
            }
        }
        result
    }
}

pub struct LimitedHttp<'a> {
    client: &'a reqwest::Client,
    block_private: bool,
}

impl<'a> LimitedHttp<'a> {
    pub fn new(client: &'a reqwest::Client, block_private: bool) -> Self {
        Self {
            client,
            block_private,
        }
    }

    fn wrap<R: AsyncRead>(inner: R) -> CappedRead<R> {
        CappedRead::new(inner, MAX_REPOSITORY_BYTES as u64, REPOSITORY_TIMEOUT, TOO_LARGE_MESSAGE)
    }
}

pub async fn check_host(url: &Url, block_private: bool) -> io::Result<()> {
    ensure_allowed(url, block_private)
        .await
        .map_err(|message| io::Error::new(io::ErrorKind::PermissionDenied, message))
}

pub fn remove_incomplete_zips(repos: &std::path::Path) -> usize {
    let Ok(packages) = std::fs::read_dir(repos) else {
        return 0;
    };
    let mut removed = 0;
    for package in packages.filter_map(Result::ok) {
        if !package.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(package.path()) else {
            continue;
        };
        for file in files.filter_map(Result::ok) {
            let path = file.path();
            let is_zip = path.extension().map(|extension| extension == "zip").unwrap_or(false);
            let finished = path.with_extension("zip.sha256").is_file();
            let is_file = file.file_type().map(|kind| kind.is_file()).unwrap_or(false);
            if is_zip && is_file && !finished && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

impl HttpClient for LimitedHttp<'_> {
    async fn get(
        &self,
        url: &Url,
        headers: &IndexMap<&str, &str>,
    ) -> io::Result<impl AsyncRead + Send> {
        check_host(url, self.block_private).await?;
        let stream = HttpClient::get(self.client, url, headers).await?;
        Ok(Self::wrap(stream))
    }

    async fn get_with_etag(
        &self,
        url: &Url,
        headers: &IndexMap<Box<str>, Box<str>>,
        current_etag: Option<&str>,
    ) -> io::Result<Option<(impl AsyncRead + Send, Option<Box<str>>)>> {
        check_host(url, self.block_private).await?;
        let response = HttpClient::get_with_etag(self.client, url, headers, current_etag).await?;
        Ok(response.map(|(stream, etag)| (Self::wrap(stream), etag)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::io::{AsyncReadExt, Cursor};

    #[test]
    fn a_body_within_the_cap_is_read_in_full() {
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 100]), 100, Duration::from_secs(5), TOO_LARGE_MESSAGE);
        let mut out = Vec::new();
        block_on(reader.read_to_end(&mut out)).unwrap();
        assert_eq!(out.len(), 100);
    }

    #[test]
    fn a_body_over_the_cap_is_rejected() {
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 101]), 100, Duration::from_secs(5), TOO_LARGE_MESSAGE);
        let mut out = Vec::new();
        let error = block_on(reader.read_to_end(&mut out)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn the_error_carries_the_message_it_was_given() {
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 5]), 4, Duration::MAX, PACKAGE_TOO_LARGE_MESSAGE);
        let mut out = Vec::new();
        let error = block_on(reader.read_to_end(&mut out)).unwrap_err();
        assert_eq!(error.to_string(), PACKAGE_TOO_LARGE_MESSAGE);
    }

    #[test]
    fn private_hosts_are_refused_before_any_request() {
        let url: Url = "http://169.254.169.254/latest".parse().unwrap();
        let error = block_on(check_host(&url, true)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(block_on(check_host(&url, false)).is_ok());
    }

    #[test]
    fn only_zips_without_a_checksum_file_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("com.example.pkg");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("done.zip"), "x").unwrap();
        std::fs::write(package.join("done.zip.sha256"), "h").unwrap();
        std::fs::write(package.join("partial.zip"), "x").unwrap();
        std::fs::write(package.join("notes.txt"), "x").unwrap();
        std::fs::write(dir.path().join("vrc-official.json"), "{}").unwrap();
        assert_eq!(remove_incomplete_zips(dir.path()), 1);
        assert!(package.join("done.zip").exists());
        assert!(package.join("done.zip.sha256").exists());
        assert!(!package.join("partial.zip").exists());
        assert!(package.join("notes.txt").exists());
        assert!(dir.path().join("vrc-official.json").exists());
    }

    #[test]
    fn a_read_after_the_deadline_times_out() {
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 10]), 100, Duration::ZERO, TOO_LARGE_MESSAGE);
        std::thread::sleep(Duration::from_millis(5));
        let mut out = Vec::new();
        let error = block_on(reader.read_to_end(&mut out)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }
}
