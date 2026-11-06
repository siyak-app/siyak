//! The only HTTP client in the app: a fixed host allow-list plus a connection
//! log without content (spec §14.1). The webview never talks to the network.
//!
//! Redirects are followed here, by hand, so that every hop is checked against
//! the allow-list and logged; reqwest's own redirect handling is off. Only the
//! host, time, purpose, status and body size of a request are logged, never
//! its path, query or content.
//!
//! Embedding APIs are reached with [`Net::post_json`]: their hosts are on the
//! allow-list, and the address of an OpenAI-compatible server is allowed only
//! because the user entered it ([`user_server_url`]). POST requests follow no
//! redirects, so a key is never sent on to another host.

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
    // Embedding APIs (spec §7.4); the AI layer adds its own hosts (spec §8).
    "api.openai.com",
    "api.voyageai.com",
    "generativelanguage.googleapis.com",
];

/// Hosts an OpenAI-compatible server may use plain HTTP on: this computer.
const LOOPBACK_HOSTS: &[&str] = &["localhost", "127.0.0.1", "[::1]"];

/// Most redirects followed for one request.
const MAX_REDIRECTS: usize = 5;

/// Why a request is made; recorded in the connection log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Fetching `manifest.json`.
    Manifest,
    /// Downloading a pack file.
    Pack,
    /// Embedding a query or passages with an API (spec §7.4).
    Embedding,
}

impl Purpose {
    /// The value stored in `network_log.purpose`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Pack => "pack",
            Self::Embedding => "embed",
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

/// Writes the connection log to `user.db` on a background thread with its own
/// connection, so a request never waits for (or deadlocks on) the shared one.
pub struct UserDbLog {
    tx: std::sync::mpsc::Sender<(String, Purpose, Option<u16>, u64)>,
}

impl UserDbLog {
    /// Starts the writer for the (already migrated) `user.db` at `path`.
    pub fn spawn(path: &std::path::Path) -> AppResult<Self> {
        let conn = crate::db::connect(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let (tx, rx) = std::sync::mpsc::channel::<(String, Purpose, Option<u16>, u64)>();
        std::thread::Builder::new()
            .name("network-log".into())
            .spawn(move || {
                for (host, purpose, status, bytes) in rx {
                    let result =
                        crate::db::user::log_request(&conn, &host, purpose.as_str(), status, bytes);
                    if let Err(err) = result {
                        eprintln!("writing the connection log failed: {err}");
                    }
                }
            })
            .map_err(|err| AppError::new(AppErrorCode::IoFailed, err.to_string()))?;
        Ok(Self { tx })
    }
}

impl NetLogSink for UserDbLog {
    fn record(&self, host: &str, purpose: Purpose, status: Option<u16>, bytes: u64) {
        // Fails only once the writer thread has gone, i.e. at exit.
        let _ = self.tx.send((host.to_owned(), purpose, status, bytes));
    }
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

/// Checks the address of an OpenAI-compatible server the user entered: HTTPS,
/// or plain HTTP to this computer (Ollama, LM Studio), with no credentials in
/// it. Returns it without a trailing slash.
pub fn user_server_url(address: &str) -> AppResult<Url> {
    let invalid = |why: &str| {
        AppError::new(
            AppErrorCode::HostNotAllowed,
            format!("server address: {why}"),
        )
    };
    let mut url = Url::parse(address.trim()).map_err(|err| invalid(&err.to_string()))?;
    let host = url.host_str().ok_or_else(|| invalid("no host"))?.to_owned();
    match url.scheme() {
        "https" => {}
        "http" if LOOPBACK_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(&host)) => {}
        "http" => return Err(invalid("plain HTTP is allowed only to this computer")),
        _ => return Err(invalid("not an HTTP address")),
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("credentials in the address"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid("a query or fragment"));
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Ok(url)
}

/// Whether `url` is on the server at `server` (same scheme, host and port).
fn same_server(url: &Url, server: &Url) -> bool {
    url.scheme() == server.scheme()
        && url.host_str() == server.host_str()
        && url.port_or_known_default() == server.port_or_known_default()
        && url.username().is_empty()
        && url.password().is_none()
}

/// The app's HTTP client.
#[derive(Clone)]
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

    /// Sends a JSON POST request and reads the response body (at most `limit`
    /// bytes) whatever its status. `url` must be on the allow-list, or on
    /// `user_server`: the checked address of the user's OpenAI-compatible
    /// server ([`user_server_url`]). Redirects are not followed.
    ///
    /// `headers` carry the API key; neither they nor the body are logged.
    pub async fn post_json(
        &self,
        url: &Url,
        user_server: Option<&Url>,
        purpose: Purpose,
        headers: &[(&str, &str)],
        body: &serde_json::Value,
        limit: usize,
    ) -> AppResult<(StatusCode, Vec<u8>)> {
        let host = match user_server {
            Some(server) if same_server(url, server) => {
                url.host_str().unwrap_or_default().to_owned()
            }
            _ => self.check(url)?,
        };
        let body = serde_json::to_vec(body)
            .map_err(|err| AppError::new(AppErrorCode::InvalidInput, err.to_string()))?;
        let mut request = self
            .client
            .post(url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        for (name, value) in headers {
            let mut value = HeaderValue::from_str(value).map_err(|_| {
                AppError::new(AppErrorCode::InvalidInput, format!("invalid {name} header"))
            })?;
            value.set_sensitive(true);
            request = request.header(*name, value);
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
        let mut response = Response {
            inner: response,
            host,
            purpose,
            received: AtomicU64::new(0),
            log: Arc::clone(&self.log),
        };
        if status.is_redirection() {
            return Err(AppError::new(
                AppErrorCode::NetworkFailed,
                format!("{}: answered with a redirect ({status})", response.host),
            ));
        }
        let body = response.bytes_limited(limit).await?;
        Ok((status, body))
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
        assert!(allow.permits(&url("https://GitHub.com/siyak-app/siyak-data/releases")));
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

    #[test]
    fn user_db_log_writes_entries_in_the_background() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("user.db");
        drop(crate::db::user::open(&path).unwrap());
        let log = UserDbLog::spawn(&path).unwrap();
        log.record("github.com", Purpose::Pack, Some(200), 42);
        drop(log);
        let conn = crate::db::connect(&path).unwrap();
        for _ in 0..100 {
            let entries = crate::db::user::network_log(&conn, 10).unwrap();
            if let [entry] = entries.as_slice() {
                assert_eq!(
                    (entry.host.as_str(), entry.status),
                    ("github.com", Some(200))
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("the entry was not written");
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

    #[test]
    fn production_allow_list_has_the_embedding_apis() {
        let allow = AllowList::production();
        for host in [
            "api.openai.com",
            "api.voyageai.com",
            "generativelanguage.googleapis.com",
        ] {
            assert!(allow.permits(&url(&format!("https://{host}/v1/embeddings"))));
        }
        assert!(!allow.permits(&url("https://localhost:11434/v1/embeddings")));
    }

    #[test]
    fn user_servers_need_https_unless_on_this_computer() {
        for ok in [
            "http://localhost:11434/v1/",
            "http://127.0.0.1:1234/v1",
            "http://[::1]:8080",
            "https://llm.example.org/v1",
        ] {
            let parsed = user_server_url(ok).unwrap();
            assert!(
                !parsed.path().ends_with('/') || parsed.path() == "/",
                "{parsed}"
            );
        }
        assert_eq!(
            user_server_url(" http://localhost:11434/v1/ ")
                .unwrap()
                .as_str(),
            "http://localhost:11434/v1"
        );
        for bad in [
            "http://192.168.1.5:11434/v1",
            "http://localhost.example.org/v1",
            "ftp://localhost/",
            "https://user:pw@llm.example.org/",
            "https://llm.example.org/v1?key=1",
            "localhost:11434",
            "",
        ] {
            let err = user_server_url(bad).unwrap_err();
            assert_eq!(err.code, AppErrorCode::HostNotAllowed, "{bad}");
        }
    }

    fn post_router() -> Router {
        use axum::routing::post;
        Router::new()
            .route(
                "/echo",
                post(|headers: HeaderMap, body: String| async move {
                    let auth = headers
                        .get(header::AUTHORIZATION)
                        .map(|v| v.to_str().unwrap().to_owned())
                        .unwrap_or_default();
                    format!("{auth}|{body}")
                }),
            )
            .route(
                "/moved",
                post(|| async {
                    (
                        AxumStatus::TEMPORARY_REDIRECT,
                        [(header::LOCATION, "/echo")],
                    )
                }),
            )
    }

    #[tokio::test]
    async fn posts_json_with_headers_and_logs_without_content() {
        let base = serve(post_router()).await;
        let (net, log) = local_net();
        let (status, body) = net
            .post_json(
                &url(&format!("{base}/echo")),
                None,
                Purpose::Embedding,
                &[("authorization", "Bearer sk-1")],
                &serde_json::json!({ "input": ["sabır"] }),
                1000,
            )
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            String::from_utf8(body).unwrap(),
            r#"Bearer sk-1|{"input":["sabır"]}"#
        );
        let entries = log.0.lock().unwrap().clone();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "127.0.0.1");
        assert_eq!(entries[0].1, "embed");
        assert_eq!(entries[0].2, Some(200));
    }

    #[tokio::test]
    async fn posts_follow_no_redirects() {
        let base = serve(post_router()).await;
        let (net, log) = local_net();
        let err = net
            .post_json(
                &url(&format!("{base}/moved")),
                None,
                Purpose::Embedding,
                &[("authorization", "Bearer sk-1")],
                &serde_json::json!({}),
                1000,
            )
            .await
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::NetworkFailed);
        assert_eq!(
            log.0.lock().unwrap().len(),
            1,
            "the echo route was not called"
        );
    }

    #[tokio::test]
    async fn posts_reach_the_user_server_and_nothing_else() {
        let base = serve(post_router()).await;
        let log = Arc::new(test_support::MemoryLog::default());
        // An empty allow-list: only the user's server is reachable.
        let net =
            Net::with_allow_list(AllowList::for_tests(&[]), log as Arc<dyn NetLogSink>).unwrap();
        let server = user_server_url(&base.replace("127.0.0.1", "localhost")).unwrap();
        let echo = url(&format!("{server}/echo"));
        net.post_json(
            &echo,
            Some(&server),
            Purpose::Embedding,
            &[],
            &serde_json::json!({}),
            1000,
        )
        .await
        .unwrap();
        let elsewhere = url("https://example.org/echo");
        let err = net
            .post_json(
                &elsewhere,
                Some(&server),
                Purpose::Embedding,
                &[],
                &serde_json::json!({}),
                1000,
            )
            .await
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::HostNotAllowed);
    }
}
