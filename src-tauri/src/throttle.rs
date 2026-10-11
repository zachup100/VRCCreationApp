use crate::hosts::{resolve_allowed, BLOCKED_MESSAGE};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

pub const LIMIT_CHOICES: [u32; 8] = [0, 1, 2, 4, 8, 16, 24, 32];
pub const BYTES_PER_MEGABYTE: u64 = 1_000_000;

const ALLOWED_SUFFIXES: [&str; 2] = ["unity3d.com", "unity.com"];
const HEAD_LIMIT: usize = 8 * 1024;
const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_CONNECTIONS: usize = 128;
const CHUNK: usize = 16 * 1024;
const IDLE_LIMIT: Duration = Duration::from_secs(5 * 60);

pub fn limit_allowed(megabytes: u32) -> bool {
    LIMIT_CHOICES.contains(&megabytes)
}

pub fn host_allowed(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    ALLOWED_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
}

pub fn parse_connect(head: &str) -> Option<(String, u16)> {
    let line = head.lines().next()?;
    let mut parts = line.split_whitespace();
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    if method != "CONNECT" || !version.starts_with("HTTP/") || parts.next().is_some() {
        return None;
    }
    let (host, port) = target.rsplit_once(':')?;
    let valid = !host.is_empty()
        && host
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '.' || character == '-');
    if !valid {
        return None;
    }
    Some((host.to_string(), port.parse().ok()?))
}

pub struct Limiter {
    rate: u64,
    next: Mutex<Instant>,
}

impl Limiter {
    pub fn new(rate: u64) -> Self {
        Self {
            rate: rate.max(1),
            next: Mutex::new(Instant::now()),
        }
    }

    pub fn reserve(&self, bytes: usize) -> Duration {
        let now = Instant::now();
        let Ok(mut next) = self.next.lock() else {
            return Duration::ZERO;
        };
        let start = (*next).max(now);
        *next = start + Duration::from_secs_f64(bytes as f64 / self.rate as f64);
        start.saturating_duration_since(now)
    }

    pub async fn take(&self, bytes: usize) {
        let wait = self.reserve(bytes);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub rate: u64,
    pub block_private: bool,
    pub port: u16,
    pub allow: fn(&str) -> bool,
    pub idle: Duration,
}

impl Config {
    pub fn unity(rate: u64, block_private: bool) -> Self {
        Self {
            rate,
            block_private,
            port: 443,
            allow: host_allowed,
            idle: IDLE_LIMIT,
        }
    }
}

struct Activity {
    base: Instant,
    last: AtomicU64,
}

impl Activity {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            last: AtomicU64::new(0),
        }
    }

    fn touch(&self) {
        self.last
            .store(self.base.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    fn idle_for(&self) -> Duration {
        let now = self.base.elapsed().as_millis() as u64;
        Duration::from_millis(now.saturating_sub(self.last.load(Ordering::Relaxed)))
    }
}

pub struct Throttle {
    port: u16,
    transferred: Arc<AtomicU64>,
    _stop: watch::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl Throttle {
    pub async fn start(config: Config) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let (stop, stopped) = watch::channel(());
        let limiter = Arc::new(Limiter::new(config.rate));
        let transferred = Arc::new(AtomicU64::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        let counter = transferred.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((client, _)) = listener.accept().await else {
                    break;
                };
                if active.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    continue;
                }
                active.fetch_add(1, Ordering::SeqCst);
                let (config, limiter, counter, stopped, active) = (
                    config.clone(),
                    limiter.clone(),
                    counter.clone(),
                    stopped.clone(),
                    active.clone(),
                );
                tokio::spawn(async move {
                    handle(client, config, limiter, counter, stopped).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        Ok(Self {
            port,
            transferred,
            _stop: stop,
            task,
        })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn counter(&self) -> Arc<AtomicU64> {
        self.transferred.clone()
    }
}

impl Drop for Throttle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_head(client: &mut TcpStream) -> Option<(String, Vec<u8>)> {
    let read = async {
        let mut data: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            if let Some(end) = data.windows(4).position(|window| window == b"\r\n\r\n") {
                let rest = data.split_off(end + 4);
                return Some((String::from_utf8_lossy(&data).into_owned(), rest));
            }
            if data.len() > HEAD_LIMIT {
                return None;
            }
            match client.read(&mut buffer).await {
                Ok(0) | Err(_) => return None,
                Ok(count) => data.extend_from_slice(&buffer[..count]),
            }
        }
    };
    tokio::time::timeout(HEAD_TIMEOUT, read).await.ok().flatten()
}

async fn refuse(client: &mut TcpStream, status: &str) {
    let reply = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let _ = client.write_all(reply.as_bytes()).await;
}

async fn handle(
    mut client: TcpStream,
    config: Config,
    limiter: Arc<Limiter>,
    transferred: Arc<AtomicU64>,
    mut stopped: watch::Receiver<()>,
) {
    let Some((head, rest)) = read_head(&mut client).await else {
        return;
    };
    let host = match parse_connect(&head) {
        Some((host, port)) if port == config.port && (config.allow)(&host) => host,
        _ => return refuse(&mut client, "403 Forbidden").await,
    };
    let addresses = match resolve_allowed(&host, config.port, config.block_private).await {
        Ok(addresses) => addresses,
        Err(message) if message == BLOCKED_MESSAGE => return refuse(&mut client, "403 Forbidden").await,
        Err(_) => return refuse(&mut client, "502 Bad Gateway").await,
    };
    let upstream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&addresses[..])).await {
        Ok(Ok(stream)) => stream,
        _ => return refuse(&mut client, "502 Bad Gateway").await,
    };
    if client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
        .is_err()
    {
        return;
    }
    let (mut client_read, mut client_write) = client.into_split();
    let (mut upstream_read, mut upstream_write) = upstream.into_split();
    if !rest.is_empty() && upstream_write.write_all(&rest).await.is_err() {
        return;
    }
    let activity = Activity::new();
    activity.touch();
    let upward = async {
        let mut buffer = vec![0u8; CHUNK];
        loop {
            let count = match client_read.read(&mut buffer).await {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            activity.touch();
            if upstream_write.write_all(&buffer[..count]).await.is_err() {
                break;
            }
        }
        let _ = upstream_write.shutdown().await;
    };
    let downward = async {
        let mut buffer = vec![0u8; CHUNK];
        loop {
            let count = match upstream_read.read(&mut buffer).await {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            activity.touch();
            limiter.take(count).await;
            if client_write.write_all(&buffer[..count]).await.is_err() {
                break;
            }
            transferred.fetch_add(count as u64, Ordering::Relaxed);
            activity.touch();
        }
        let _ = client_write.shutdown().await;
    };
    let watchdog = async {
        let tick = (config.idle / 4).clamp(Duration::from_millis(50), Duration::from_secs(10));
        loop {
            tokio::time::sleep(tick).await;
            if activity.idle_for() > config.idle {
                break;
            }
        }
    };
    let stop = async {
        let _ = stopped.changed().await;
    };
    let ends = futures::future::select(Box::pin(watchdog), Box::pin(stop));
    futures::future::select(Box::pin(futures::future::join(upward, downward)), ends).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn local(host: &str) -> bool {
        host == "127.0.0.1"
    }

    async fn ask(proxy: &Throttle, request: &str) -> String {
        let address = proxy.url().trim_start_matches("http://").to_string();
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut reply = Vec::new();
        let _ = stream.read_to_end(&mut reply).await;
        String::from_utf8_lossy(&reply).into_owned()
    }

    #[test]
    fn only_unity_hosts_are_allowed() {
        assert!(host_allowed("download.unity3d.com"));
        assert!(host_allowed("DOWNLOAD.UNITY3D.COM."));
        assert!(host_allowed("unity.com"));
        assert!(host_allowed("services.api.unity.com"));
        assert!(!host_allowed("evil.com"));
        assert!(!host_allowed("unity3d.com.evil.com"));
        assert!(!host_allowed("notunity3d.com"));
        assert!(!host_allowed("127.0.0.1"));
        assert!(!host_allowed(""));
    }

    #[test]
    fn connect_requests_are_parsed_strictly() {
        assert_eq!(
            parse_connect("CONNECT download.unity3d.com:443 HTTP/1.1\r\nHost: x\r\n\r\n"),
            Some(("download.unity3d.com".to_string(), 443))
        );
        assert_eq!(parse_connect("GET http://unity.com/ HTTP/1.1\r\n\r\n"), None);
        assert_eq!(parse_connect("CONNECT unity.com HTTP/1.1\r\n\r\n"), None);
        assert_eq!(parse_connect("CONNECT unity.com:abc HTTP/1.1\r\n\r\n"), None);
        assert_eq!(parse_connect("CONNECT [::1]:443 HTTP/1.1\r\n\r\n"), None);
        assert_eq!(parse_connect("CONNECT a b:443 HTTP/1.1\r\n\r\n"), None);
        assert_eq!(parse_connect("CONNECT unity.com:443 FTP\r\n\r\n"), None);
        assert_eq!(parse_connect(""), None);
    }

    #[test]
    fn only_listed_limits_are_valid() {
        for value in [0, 1, 2, 4, 8, 16, 24, 32] {
            assert!(limit_allowed(value));
        }
        for value in [3, 5, 33, 100, u32::MAX] {
            assert!(!limit_allowed(value));
        }
    }

    #[test]
    fn the_limiter_spaces_out_reservations() {
        let limiter = Limiter::new(1000);
        assert!(limiter.reserve(500) < Duration::from_millis(50));
        let second = limiter.reserve(500);
        assert!(second >= Duration::from_millis(400) && second <= Duration::from_millis(520));
        let third = limiter.reserve(500);
        assert!(third >= Duration::from_millis(900) && third <= Duration::from_millis(1020));
    }

    #[test]
    fn a_tunnel_to_an_allowed_host_is_slowed_to_the_limit() {
        run(async {
            let upstream = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = upstream.local_addr().unwrap().port();
            tokio::spawn(async move {
                let (mut stream, _) = upstream.accept().await.unwrap();
                stream.write_all(&vec![7u8; 200_000]).await.unwrap();
            });
            let proxy = Throttle::start(Config {
                rate: 400_000,
                block_private: false,
                port,
                allow: local,
                idle: Duration::from_secs(60),
            })
            .await
            .unwrap();
            let address = proxy.url().trim_start_matches("http://").to_string();
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream
                .write_all(format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let started = Instant::now();
            let mut all = Vec::new();
            stream.read_to_end(&mut all).await.unwrap();
            let head = b"HTTP/1.1 200 Connection Established\r\n\r\n";
            assert!(all.starts_with(head));
            assert_eq!(all.len() - head.len(), 200_000);
            assert!(started.elapsed() >= Duration::from_millis(350), "{:?}", started.elapsed());
            assert!(started.elapsed() < Duration::from_secs(5));
        });
    }

    #[test]
    fn other_hosts_ports_and_methods_are_refused() {
        run(async {
            let proxy = Throttle::start(Config::unity(1_000_000, false)).await.unwrap();
            for request in [
                "CONNECT evil.com:443 HTTP/1.1\r\n\r\n",
                "CONNECT download.unity3d.com:80 HTTP/1.1\r\n\r\n",
                "GET http://download.unity3d.com/ HTTP/1.1\r\n\r\n",
                "CONNECT 127.0.0.1:443 HTTP/1.1\r\n\r\n",
            ] {
                let reply = ask(&proxy, request).await;
                assert!(reply.starts_with("HTTP/1.1 403"), "{request}: {reply}");
            }
        });
    }

    #[test]
    fn private_addresses_are_refused_even_for_allowed_names() {
        run(async {
            let proxy = Throttle::start(Config {
                rate: 1_000_000,
                block_private: true,
                port: 443,
                allow: |host| host == "127.0.0.1",
                idle: Duration::from_secs(60),
            })
            .await
            .unwrap();
            let reply = ask(&proxy, "CONNECT 127.0.0.1:443 HTTP/1.1\r\n\r\n").await;
            assert!(reply.starts_with("HTTP/1.1 403"), "{reply}");
        });
    }

    async fn silent_upstream() -> u16 {
        let upstream = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = upstream.local_addr().unwrap().port();
        tokio::spawn(async move {
            let mut held = Vec::new();
            loop {
                let Ok((stream, _)) = upstream.accept().await else {
                    break;
                };
                held.push(stream);
            }
        });
        port
    }

    async fn open_tunnel(proxy: &Throttle, port: u16) -> TcpStream {
        let address = proxy.url().trim_start_matches("http://").to_string();
        let mut stream = TcpStream::connect(address).await.unwrap();
        stream
            .write_all(format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut reply = vec![0u8; "HTTP/1.1 200 Connection Established\r\n\r\n".len()];
        stream.read_exact(&mut reply).await.unwrap();
        assert!(reply.starts_with(b"HTTP/1.1 200"));
        stream
    }

    #[test]
    fn an_idle_tunnel_is_closed() {
        run(async {
            let port = silent_upstream().await;
            let proxy = Throttle::start(Config {
                rate: 1_000_000,
                block_private: false,
                port,
                allow: local,
                idle: Duration::from_millis(300),
            })
            .await
            .unwrap();
            let mut stream = open_tunnel(&proxy, port).await;
            let mut buffer = [0u8; 16];
            let result = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await;
            assert!(matches!(result, Ok(Ok(0)) | Ok(Err(_))), "{result:?}");
        });
    }

    #[test]
    fn open_tunnels_end_when_the_proxy_is_dropped() {
        run(async {
            let port = silent_upstream().await;
            let proxy = Throttle::start(Config {
                rate: 1_000_000,
                block_private: false,
                port,
                allow: local,
                idle: Duration::from_secs(300),
            })
            .await
            .unwrap();
            let mut stream = open_tunnel(&proxy, port).await;
            drop(proxy);
            let mut buffer = [0u8; 16];
            let result = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await;
            assert!(matches!(result, Ok(Ok(0)) | Ok(Err(_))), "{result:?}");
        });
    }

    #[test]
    fn the_counter_reports_bytes_sent_down_the_tunnel() {
        run(async {
            let upstream = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = upstream.local_addr().unwrap().port();
            tokio::spawn(async move {
                let (mut stream, _) = upstream.accept().await.unwrap();
                stream.write_all(&vec![1u8; 50_000]).await.unwrap();
            });
            let proxy = Throttle::start(Config {
                rate: 10_000_000,
                block_private: false,
                port,
                allow: local,
                idle: Duration::from_secs(60),
            })
            .await
            .unwrap();
            let counter = proxy.counter();
            assert_eq!(counter.load(Ordering::Relaxed), 0);
            let mut stream = open_tunnel(&proxy, port).await;
            let mut all = Vec::new();
            stream.read_to_end(&mut all).await.unwrap();
            assert_eq!(all.len(), 50_000);
            assert_eq!(counter.load(Ordering::Relaxed), 50_000);
        });
    }

    #[test]
    fn dropping_the_proxy_stops_the_listener() {
        run(async {
            let proxy = Throttle::start(Config::unity(1_000_000, false)).await.unwrap();
            let address = proxy.url().trim_start_matches("http://").to_string();
            drop(proxy);
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(TcpStream::connect(address).await.is_err());
        });
    }
}
