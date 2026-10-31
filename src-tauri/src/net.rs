//! The only HTTP client in the app: a fixed host allow-list plus a connection
//! log without content (spec §14.1). The webview never talks to the network.
//!
//! Redirects are followed here, by hand, so that every hop is checked against
//! the allow-list and logged; reqwest's own redirect handling is off. Only the
//! host, time, purpose, status and body size of a request are logged, never
//! its path, query or content.
//!
//! TODO(M8): provider hosts, the user-entered OpenAI-compatible `base_url`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::header::{HeaderValue, LOCATION, RANGE};
use reqwest::{StatusCode, Url};

use crate::error::{AppError, AppErrorCode, AppResult};

/// Hosts the app may contact for manifests and packs (spec §14.1). GitHub
/// release downloads redirect from `github.com` to
/// `release-assets.githubusercontent.com` (checked 2026-10-22);
/// `objects.githubusercontent.com` is the older redirect target.
pub const ALLOWED_HOSTS: &[&str] = &[
    "quran.kemalozyon.com",
    "github.com",
    "release-assets.githubusercontent.com",
    "objects.githubusercontent.com",
];

/// Most redirects followed for one request.
const MAX_REDIRECTS: usize = 5;

/// Why a request is made; recorded in the connection log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Fetching `manifest.json`.
    Manifest,
    /// Downloading a pack file.
    Pack,
}

impl Purpose {
    /// The value stored in `network_log.purpose`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Pack => "pack",
        }
    }
}

/// Where connection log entries go (`user.db` in the app).
pub trait NetLogSink: Send + Sync {
    /// Records one request: `status` is `None` if no response arrived.
    fn record(&self, host: &str, purpose: Purpose, status: Option<u16>, bytes: u64);
}

/// A sink that drops every entry (tools and tests that need no log).
pub struct NoLog;

impl NetLogSink for NoLog {
    fn record(&self, _host: &str, _purpose: Purpose, _status: Option<u16>, _bytes: u64) {}
}

/// The hosts (and schemes) requests may go to.
#[derive(Debug, Clone)]
pub struct AllowList {
    hosts: Vec<String>,
    allow_http: bool,
}

impl AllowList {
    /// The app's allow-list: [`ALLOWED_HOSTS`] over HTTPS only.
    pub fn production() -> Self {
        Self {
            hosts: ALLOWED_HOSTS.iter().map(|h| (*h).to_owned()).collect(),
            allow_http: false,
        }
    }

    /// An allow-list for tests against a local server, which speaks plain HTTP.
    pub fn for_tests(hosts: &[&str]) -> Self {
        Self {
            hosts: hosts.iter().map(|h| (*h).to_owned()).collect(),
            allow_http: true,
        }
    }

    /// Whether `url` may be requested.
    pub fn permits(&self, url: &Url) -> bool {
        let scheme_ok = url.scheme() == "https" || (self.allow_http && url.scheme() == "http");
        scheme_ok
            && url.username().is_empty()
            && url.password().is_none()
            && url
                .host_str()
                .is_some_and(|host| self.hosts.iter().any(|h| h.eq_ignore_ascii_case(host)))
    }
}

/// The app's HTTP client.
pub struct Net {
    client: reqwest::Client,
    allow: AllowList,
    log: Arc<dyn NetLogSink>,
}

impl Net {
    /// Creates the client with the production allow-list.
    pub fn new(log: Arc<dyn NetLogSink>) -> AppResult<Self> {
        Self::with_allow_list(AllowList::production(), log)
    }

    /// Creates a client with a custom allow-list (tests use a local server).
    pub fn with_allow_list(allow: AllowList, log: Arc<dyn NetLogSink>) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("Siyak/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(30))
            .build()
            .map_err(|err| AppError::new(AppErrorCode::NetworkFailed, err.to_string()))?;
        Ok(Self { client, allow, log })
    }

    /// Sends a GET request, following redirects within the allow-list. With
    /// `range_from`, asks for the bytes from that offset (`Range: bytes=N-`).
    ///
    /// Returns the final response whatever its status; the body size is
    /// logged when the returned [`Response`] is dropped.
    pub async fn get(
        &self,
        url: &str,
        purpose: Purpose,
        range_from: Option<u64>,
    ) -> AppResult<Response> {
        let mut url = Url::parse(url).map_err(|err| {
            AppError::new(AppErrorCode::HostNotAllowed, format!("invalid URL: {err}"))
        })?;
        for _ in 0..=MAX_REDIRECTS {
            let host = self.check(&url)?;
            let mut request = self.client.get(url.clone());
            if let Some(from) = range_from.filter(|from| *from > 0) {
                request = request.header(RANGE, format!("bytes={from}-"));
            }
            let response = match request.send().await {
                Ok(response) => response,
                Err(err) => {
                    self.log.record(&host, purpose, None, 0);
                    return Err(AppError::new(
                        AppErrorCode::NetworkFailed,
                        format!("{host}: {}", without_url(&err)),
                    ));
                }
            };
            let status = response.status();
            if status.is_redirection() {
                self.log.record(&host, purpose, Some(status.as_u16()), 0);
                url = redirect_target(&url, response.headers().get(LOCATION))?;
                continue;
            }
            return Ok(Response {
                inner: response,
                host,
                purpose,
                received: AtomicU64::new(0),
                log: Arc::clone(&self.log),
            });
        }
        Err(AppError::new(
            AppErrorCode::NetworkFailed,
            format!("more than {MAX_REDIRECTS} redirects"),
        ))
    }

    fn check(&self, url: &Url) -> AppResult<String> {
        if !self.allow.permits(url) {
            return Err(AppError::new(
                AppErrorCode::HostNotAllowed,
                format!(
                    "{}://{} is not on the allow-list",
                    url.scheme(),
                    url.host_str().unwrap_or("")
                ),
            ));
        }
        Ok(url.host_str().unwrap_or_default().to_owned())
    }
}

/// The `Location` of a redirect, resolved against the request URL.
fn redirect_target(base: &Url, location: Option<&HeaderValue>) -> AppResult<Url> {
    location
        .and_then(|value| value.to_str().ok())
        .and_then(|value| base.join(value).ok())
        .ok_or_else(|| AppError::new(AppErrorCode::NetworkFailed, "redirect without a location"))
}

/// A reqwest error message without the URL (it may carry a signed query).
fn without_url(err: &reqwest::Error) -> String {
    let mut message = err.to_string();
    if let Some(url) = err.url() {
        message = message.replace(url.as_str(), "<url>");
    }
    message
}

/// A response from an allow-listed host. Logs its status and the number of
/// body bytes read when dropped.
pub struct Response {
    inner: reqwest::Response,
    host: String,
    purpose: Purpose,
    received: AtomicU64,
    log: Arc<dyn NetLogSink>,
}

impl Response {
    /// The HTTP status.
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    /// The host the response came from.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// A response header as text, if present and valid.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.inner.headers().get(name)?.to_str().ok()
    }

    /// The body length announced by the server, if any.
    pub fn content_length(&self) -> Option<u64> {
        self.inner.content_length()
    }

    /// The next chunk of the body, or `None` at its end.
    pub async fn chunk(&mut self) -> AppResult<Option<bytes::Bytes>> {
        let chunk = self.inner.chunk().await.map_err(|err| {
            AppError::new(
                AppErrorCode::NetworkFailed,
                format!("{}: {}", self.host, without_url(&err)),
            )
        })?;
        if let Some(chunk) = &chunk {
            self.received
                .fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
        Ok(chunk)
    }

    /// Reads the whole body, failing once it exceeds `limit` bytes.
    pub async fn bytes_limited(&mut self, limit: usize) -> AppResult<Vec<u8>> {
        let mut body = Vec::new();
        while let Some(chunk) = self.chunk().await? {
            if body.len() + chunk.len() > limit {
                return Err(AppError::new(
                    AppErrorCode::DownloadFailed,
                    format!("{}: response larger than {limit} bytes", self.host),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

impl Drop for Response {
    fn drop(&mut self) {
        self.log.record(
            &self.host,
            self.purpose,
            Some(self.inner.status().as_u16()),
            self.received.load(Ordering::Relaxed),
        );
    }
}

/// Test helpers shared by the network, manifest and download tests: a local
/// HTTP server and a log that keeps entries in memory.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    use super::*;

    /// One connection log entry: host, purpose, status and bytes received.
    pub(crate) type LogEntry = (String, &'static str, Option<u16>, u64);

    /// Connection log entries kept in memory.
    #[derive(Default)]
    pub(crate) struct MemoryLog(pub Mutex<Vec<LogEntry>>);

    impl NetLogSink for MemoryLog {
        fn record(&self, host: &str, purpose: Purpose, status: Option<u16>, bytes: u64) {
            self.0
                .lock()
                .unwrap()
                .push((host.to_owned(), purpose.as_str(), status, bytes));
        }
    }

    /// Serves `router` on `127.0.0.1` and returns its base URL (`http://127.0.0.1:port`).
    pub(crate) async fn serve(router: axum::Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        format!("http://{addr}")
    }

    /// A client that may only talk to `127.0.0.1`, plus its log.
    pub(crate) fn local_net() -> (Net, Arc<MemoryLog>) {
        let log = Arc::new(MemoryLog::default());
        let net = Net::with_allow_list(
            AllowList::for_tests(&["127.0.0.1"]),
            Arc::clone(&log) as Arc<dyn NetLogSink>,
        )
        .unwrap();
        (net, log)
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode as AxumStatus, header};
    use axum::response::IntoResponse;
    use axum::routing::get;

    use super::test_support::{local_net, serve};
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn production_allow_list_is_https_only_and_exact() {
        let allow = AllowList::production();
        assert!(allow.permits(&url("https://quran.kemalozyon.com/data/manifest.json")));
        assert!(allow.permits(&url("https://GitHub.com/siyak/siyak-data/releases")));
        assert!(allow.permits(&url(
            "https://release-assets.githubusercontent.com/github-production-release-asset/1"
        )));
        assert!(!allow.permits(&url("http://github.com/")));
        assert!(!allow.permits(&url("https://example.org/")));
        assert!(!allow.permits(&url("https://github.com.example.org/")));
        assert!(!allow.permits(&url("https://evil.github.com/")));
        assert!(!allow.permits(&url("https://user:pw@github.com/")));
        assert!(!allow.permits(&url("ftp://github.com/")));
    }

    fn router() -> Router {
        Router::new()
            .route("/file", get(|| async { "hello" }))
            .route(
                "/hop",
                get(|| async { (AxumStatus::FOUND, [(header::LOCATION, "/file?sig=secret")]) }),
            )
            .route(
                "/away",
                get(|| async {
                    (
                        AxumStatus::FOUND,
                        [(header::LOCATION, "http://localhost:9/file")],
                    )
                }),
            )
            .route(
                "/loop",
                get(|| async { (AxumStatus::FOUND, [(header::LOCATION, "/loop")]) }),
            )
            .route(
                "/range",
                get(|headers: HeaderMap| async move {
                    headers
                        .get(header::RANGE)
                        .map(|v| v.to_str().unwrap().to_owned())
                        .unwrap_or_default()
                        .into_response()
                }),
            )
    }

    #[tokio::test]
    async fn follows_allowed_redirects_and_logs_every_hop() {
        let base = serve(router()).await;
        let (net, log) = local_net();
        let mut response = net
            .get(&format!("{base}/hop"), Purpose::Pack, None)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.bytes_limited(100).await.unwrap(), b"hello");
        drop(response);
        let entries = log.0.lock().unwrap().clone();
        assert_eq!(
            entries,
            [
                ("127.0.0.1".to_owned(), "pack", Some(302), 0),
                ("127.0.0.1".to_owned(), "pack", Some(200), 5),
            ]
        );
    }

    #[tokio::test]
    async fn blocks_hosts_and_redirects_off_the_allow_list() {
        let base = serve(router()).await;
        let (net, log) = local_net();
        let err = net
            .get("https://example.org/manifest.json", Purpose::Manifest, None)
            .await
            .err()
            .unwrap();
        assert_eq!(err.code, AppErrorCode::HostNotAllowed);
        assert!(log.0.lock().unwrap().is_empty(), "nothing was sent");

        let err = net
            .get(&format!("{base}/away"), Purpose::Pack, None)
            .await
            .err()
            .unwrap();
        assert_eq!(err.code, AppErrorCode::HostNotAllowed);
        assert_eq!(log.0.lock().unwrap().len(), 1, "only the redirect hop");
    }

    #[tokio::test]
    async fn stops_redirect_loops() {
        let base = serve(router()).await;
        let (net, _log) = local_net();
        let err = net
            .get(&format!("{base}/loop"), Purpose::Pack, None)
            .await
            .err()
            .unwrap();
        assert_eq!(err.code, AppErrorCode::NetworkFailed);
    }

    #[tokio::test]
    async fn sends_a_range_header_when_resuming() {
        let base = serve(router()).await;
        let (net, _log) = local_net();
        let mut response = net
            .get(&format!("{base}/range"), Purpose::Pack, Some(42))
            .await
            .unwrap();
        assert_eq!(response.bytes_limited(100).await.unwrap(), b"bytes=42-");
    }

    #[tokio::test]
    async fn connection_failures_are_logged_without_status() {
        let (net, log) = local_net();
        // Port 9 (discard) is closed on test machines.
        let err = net
            .get("http://127.0.0.1:9/file", Purpose::Manifest, None)
            .await
            .err()
            .unwrap();
        assert_eq!(err.code, AppErrorCode::NetworkFailed);
        assert_eq!(
            log.0.lock().unwrap().clone(),
            [("127.0.0.1".to_owned(), "manifest", None, 0)]
        );
    }

    #[tokio::test]
    async fn limits_body_size() {
        let base = serve(router()).await;
        let (net, _log) = local_net();
        let mut response = net
            .get(&format!("{base}/file"), Purpose::Manifest, None)
            .await
            .unwrap();
        let err = response.bytes_limited(3).await.unwrap_err();
        assert_eq!(err.code, AppErrorCode::DownloadFailed);
    }
}
