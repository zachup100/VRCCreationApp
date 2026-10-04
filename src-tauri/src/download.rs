use futures::io::AsyncRead;
use pin_project_lite::pin_project;
use indexmap::IndexMap;
use reqwest::Url;
use serde::Serialize;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use vrc_get_vpm::HttpClient;

const EVENT: &str = "download-progress";
const EMIT_INTERVAL: Duration = Duration::from_millis(150);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    bytes: u64,
    name: String,
}

pub struct ProgressHttp {
    client: reqwest::Client,
    app: AppHandle,
    total: Arc<AtomicU64>,
}

impl ProgressHttp {
    pub fn new(client: reqwest::Client, app: AppHandle) -> Self {
        let _ = app.emit(
            EVENT,
            DownloadProgress {
                bytes: 0,
                name: String::new(),
            },
        );
        Self {
            client,
            app,
            total: Arc::new(AtomicU64::new(0)),
        }
    }

    fn wrap<R: AsyncRead>(&self, url: &Url, inner: R) -> CountingRead<R> {
        let name = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|segment| !segment.is_empty())
            .unwrap_or_else(|| url.host_str().unwrap_or(""))
            .to_string();
        CountingRead {
            inner,
            app: self.app.clone(),
            total: self.total.clone(),
            name,
            last_emit: Instant::now() - EMIT_INTERVAL,
        }
    }
}

pin_project! {
    struct CountingRead<R> {
        #[pin]
        inner: R,
        app: AppHandle,
        total: Arc<AtomicU64>,
        name: String,
        last_emit: Instant,
    }
}

impl<R: AsyncRead> AsyncRead for CountingRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let result = this.inner.poll_read(cx, buf);
        if let Poll::Ready(Ok(read)) = &result {
            let total = this.total.fetch_add(*read as u64, Ordering::Relaxed) + *read as u64;
            if this.last_emit.elapsed() >= EMIT_INTERVAL {
                *this.last_emit = Instant::now();
                let _ = this.app.emit(
                    EVENT,
                    DownloadProgress {
                        bytes: total,
                        name: this.name.clone(),
                    },
                );
            }
        }
        result
    }
}

impl HttpClient for ProgressHttp {
    async fn get(
        &self,
        url: &Url,
        headers: &IndexMap<&str, &str>,
    ) -> io::Result<impl AsyncRead + Send> {
        let stream = HttpClient::get(&self.client, url, headers).await?;
        Ok(self.wrap(url, stream))
    }

    async fn get_with_etag(
        &self,
        url: &Url,
        headers: &IndexMap<Box<str>, Box<str>>,
        current_etag: Option<&str>,
    ) -> io::Result<Option<(impl AsyncRead + Send, Option<Box<str>>)>> {
        let response = HttpClient::get_with_etag(&self.client, url, headers, current_etag).await?;
        Ok(response.map(|(stream, etag)| (self.wrap(url, stream), etag)))
    }
}
