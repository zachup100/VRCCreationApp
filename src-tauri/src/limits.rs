use futures::io::AsyncRead;
use indexmap::IndexMap;
use pin_project_lite::pin_project;
use reqwest::Url;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use vrc_get_vpm::HttpClient;

pub const MAX_REPOSITORY_BYTES: usize = 24 * 1024 * 1024;
pub const REPOSITORY_TIMEOUT: Duration = Duration::from_secs(60);
pub const TOO_LARGE_MESSAGE: &str = "The repository listing is too large";

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
        read: usize,
        cap: usize,
        started: Instant,
        deadline: Duration,
    }
}

impl<R> CappedRead<R> {
    pub fn new(inner: R, cap: usize, deadline: Duration) -> Self {
        Self {
            inner,
            read: 0,
            cap,
            started: Instant::now(),
            deadline,
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
            *this.read += *count;
            if *this.read > *this.cap {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    TOO_LARGE_MESSAGE,
                )));
            }
        }
        result
    }
}

pub struct LimitedHttp<'a> {
    client: &'a reqwest::Client,
}

impl<'a> LimitedHttp<'a> {
    pub fn new(client: &'a reqwest::Client) -> Self {
        Self { client }
    }

    fn wrap<R: AsyncRead>(inner: R) -> CappedRead<R> {
        CappedRead::new(inner, MAX_REPOSITORY_BYTES, REPOSITORY_TIMEOUT)
    }
}

impl HttpClient for LimitedHttp<'_> {
    async fn get(
        &self,
        url: &Url,
        headers: &IndexMap<&str, &str>,
    ) -> io::Result<impl AsyncRead + Send> {
        let stream = HttpClient::get(self.client, url, headers).await?;
        Ok(Self::wrap(stream))
    }

    async fn get_with_etag(
        &self,
        url: &Url,
        headers: &IndexMap<Box<str>, Box<str>>,
        current_etag: Option<&str>,
    ) -> io::Result<Option<(impl AsyncRead + Send, Option<Box<str>>)>> {
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
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 100]), 100, Duration::from_secs(5));
        let mut out = Vec::new();
        block_on(reader.read_to_end(&mut out)).unwrap();
        assert_eq!(out.len(), 100);
    }

    #[test]
    fn a_body_over_the_cap_is_rejected() {
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 101]), 100, Duration::from_secs(5));
        let mut out = Vec::new();
        let error = block_on(reader.read_to_end(&mut out)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_read_after_the_deadline_times_out() {
        let mut reader = CappedRead::new(Cursor::new(vec![1u8; 10]), 100, Duration::ZERO);
        std::thread::sleep(Duration::from_millis(5));
        let mut out = Vec::new();
        let error = block_on(reader.read_to_end(&mut out)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }
}
