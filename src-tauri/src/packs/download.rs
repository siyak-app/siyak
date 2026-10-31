//! Resumable, cancellable pack downloads to
//! `<app_data>/downloads/<id>-<version>.part` with sha256 verification
//! (spec §6.4 steps 4–5).
//!
//! The server is not trusted any more than the pack: a body longer than the
//! manifest's `size` is refused, and nothing is installed until the sha256 of
//! the whole file matches the manifest. A dropped connection is retried with
//! an HTTP `Range` request that continues the partial file; a server that
//! ignores the range sends the whole file again, which replaces it. Partial
//! files only survive within a session: [`super::install::cleanup`] empties
//! `downloads/` at startup.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use super::manifest::{PackEntry, PackSource, Phase};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::{Net, Purpose};

/// Reports progress: phase, bytes done, bytes in total.
pub type Progress<'a> = &'a (dyn Fn(Phase, u64, u64) + Send + Sync);

/// Returns the free space of the volume holding a path (injectable for tests).
pub type DiskSpace = fn(&Path) -> io::Result<u64>;

/// Free space required on top of the pack itself.
const DISK_MARGIN: u64 = 64 << 20;

/// Attempts per download; later attempts resume the partial file.
const ATTEMPTS: u32 = 3;

/// Read buffer for copying and hashing.
const CHUNK: usize = 1 << 20;

/// Bytes a pack needs on disk while it is installed: the download, the
/// decompressed temporary file and the rows copied into `content.db`.
pub fn required_space(entry: &PackEntry) -> u64 {
    entry
        .size
        .saturating_add(entry.installed_size.saturating_mul(2))
        .saturating_add(DISK_MARGIN)
}

/// Fails with [`AppErrorCode::DiskSpaceInsufficient`] unless `dir`'s volume
/// has room for `entry` (spec: fail early, before downloading).
pub fn check_disk_space(dir: &Path, entry: &PackEntry, disk: DiskSpace) -> AppResult<()> {
    let available = disk(dir).map_err(|err| {
        AppError::new(
            AppErrorCode::IoFailed,
            format!("free space of {}: {err}", dir.display()),
        )
    })?;
    let required = required_space(entry);
    if available < required {
        return Err(AppError::new(
            AppErrorCode::DiskSpaceInsufficient,
            format!("{}: needs {required} bytes, {available} free", entry.id),
        ));
    }
    Ok(())
}

/// The partial download of `entry`.
pub fn part_path(downloads: &Path, entry: &PackEntry) -> PathBuf {
    downloads.join(format!("{}-{}.part", entry.id, entry.version))
}

/// Fetches `entry` from `source` into `downloads` and verifies its sha256.
/// Returns the path of the verified file; the caller deletes it after
/// installing. On cancellation the partial file is deleted; after a network
/// error it is kept so a retry resumes it.
pub async fn fetch(
    net: &Net,
    source: &PackSource,
    entry: &PackEntry,
    downloads: &Path,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<PathBuf> {
    fs::create_dir_all(downloads).await.map_err(io_failed)?;
    let part = part_path(downloads, entry);
    let result = match source.local_file(entry) {
        Some(file) => copy_local(&file, &part, entry, cancel, progress).await,
        None => download(net, &part, entry, cancel, progress).await,
    };
    if let Err(err) = result {
        if err.code != AppErrorCode::NetworkFailed {
            let _ = fs::remove_file(&part).await;
        }
        return Err(err);
    }
    if let Err(err) = verify(&part, entry, cancel, progress).await {
        let _ = fs::remove_file(&part).await;
        return Err(err);
    }
    let suffix = entry
        .file_name()
        .strip_prefix(entry.id.as_str())
        .unwrap_or(".pack");
    let done = downloads.join(format!("{}-{}{suffix}", entry.id, entry.version));
    fs::rename(&part, &done).await.map_err(io_failed)?;
    Ok(done)
}

async fn download(
    net: &Net,
    part: &Path,
    entry: &PackEntry,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match download_once(net, part, entry, cancel, progress).await {
            Err(err) if err.code == AppErrorCode::NetworkFailed && attempt < ATTEMPTS => {
                tokio::select! {
                    () = cancel.cancelled() => return Err(cancelled()),
                    () = tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))) => {}
                }
            }
            result => return result,
        }
    }
}

async fn download_once(
    net: &Net,
    part: &Path,
    entry: &PackEntry,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let mut have = fs::metadata(part).await.map(|m| m.len()).unwrap_or(0);
    if have > entry.size {
        have = 0;
    }
    if have == entry.size {
        return Ok(());
    }
    let mut response = tokio::select! {
        () = cancel.cancelled() => return Err(cancelled()),
        response = net.get(&entry.url, Purpose::Pack, Some(have)) => response?,
    };
    let status = response.status();
    let append = if status == StatusCode::PARTIAL_CONTENT && have > 0 {
        let expected = format!("bytes {have}-");
        if !response
            .header("content-range")
            .is_some_and(|range| range.starts_with(&expected))
        {
            return Err(AppError::new(
                AppErrorCode::DownloadFailed,
                format!("{}: unexpected Content-Range", response.host()),
            ));
        }
        true
    } else if status == StatusCode::OK {
        false
    } else {
        return Err(AppError::new(
            AppErrorCode::DownloadFailed,
            format!("{}: HTTP {status}", response.host()),
        ));
    };
    if !append {
        have = 0;
    }
    if response
        .content_length()
        .is_some_and(|len| have.saturating_add(len) > entry.size)
    {
        return Err(too_large(entry));
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(part)
        .await
        .map_err(io_failed)?;
    progress(Phase::Download, have, entry.size);
    loop {
        let chunk = tokio::select! {
            () = cancel.cancelled() => return Err(cancelled()),
            chunk = response.chunk() => chunk,
        };
        let chunk = match chunk {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(err) => {
                file.flush().await.map_err(io_failed)?;
                return Err(err);
            }
        };
        have += chunk.len() as u64;
        if have > entry.size {
            return Err(too_large(entry));
        }
        file.write_all(&chunk).await.map_err(io_failed)?;
        progress(Phase::Download, have, entry.size);
    }
    file.flush().await.map_err(io_failed)?;
    if have < entry.size {
        return Err(AppError::new(
            AppErrorCode::NetworkFailed,
            format!("{}: connection closed after {have} bytes", entry.id),
        ));
    }
    Ok(())
}

/// Copies a pack from a local source with the same progress and checks.
async fn copy_local(
    from: &Path,
    part: &Path,
    entry: &PackEntry,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let mut input = File::open(from).await.map_err(|err| {
        AppError::new(
            AppErrorCode::DownloadFailed,
            format!("{}: {err}", from.display()),
        )
    })?;
    let mut output = File::create(part).await.map_err(io_failed)?;
    let mut buf = vec![0; CHUNK];
    let mut have = 0u64;
    progress(Phase::Download, 0, entry.size);
    loop {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let n = input.read(&mut buf).await.map_err(io_failed)?;
        if n == 0 {
            break;
        }
        have += n as u64;
        if have > entry.size {
            return Err(too_large(entry));
        }
        output.write_all(&buf[..n]).await.map_err(io_failed)?;
        progress(Phase::Download, have, entry.size);
    }
    output.flush().await.map_err(io_failed)?;
    Ok(())
}

/// Checks the sha256 of the downloaded file against the manifest.
async fn verify(
    path: &Path,
    entry: &PackEntry,
    cancel: &CancellationToken,
    progress: Progress<'_>,
) -> AppResult<()> {
    let mut file = File::open(path).await.map_err(io_failed)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0; CHUNK];
    let mut done = 0u64;
    progress(Phase::Verify, 0, entry.size);
    loop {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let n = file.read(&mut buf).await.map_err(io_failed)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        done += n as u64;
        progress(Phase::Verify, done, entry.size);
    }
    let actual = hex(&hasher.finalize());
    if actual != entry.sha256 {
        return Err(AppError::new(
            AppErrorCode::PackHashMismatch,
            format!("{}: sha256 {actual}, manifest {}", entry.id, entry.sha256),
        ));
    }
    Ok(())
}

/// Lowercase hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn io_failed(err: io::Error) -> AppError {
    AppError::new(AppErrorCode::IoFailed, err.to_string())
}

fn cancelled() -> AppError {
    AppError::new(AppErrorCode::InstallCancelled, "cancelled")
}

fn too_large(entry: &PackEntry) -> AppError {
    AppError::new(
        AppErrorCode::PackTooLarge,
        format!(
            "{}: larger than the manifest's {} bytes",
            entry.id, entry.size
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::body::Body;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode as AxumStatus, header};
    use axum::response::{IntoResponse, Response};
    use axum::routing::get;

    use super::*;
    use crate::net::test_support::{local_net, serve};
    use crate::packs::manifest::parse;
    use crate::packs::manifest::tests::{entry, manifest_json};

    const BODY_LEN: usize = 300_000;

    fn body() -> Vec<u8> {
        (0..BODY_LEN).map(|i| (i % 251) as u8).collect()
    }

    fn pack_entry(url: &str, body: &[u8]) -> PackEntry {
        let mut value = entry("en.test", "translation", "1.0.0");
        value["size"] = body.len().into();
        value["installed_size"] = 1000.into();
        value["sha256"] = hex(&Sha256::digest(body)).into();
        let manifest = parse(&serde_json::to_vec(&manifest_json(vec![value])).unwrap()).unwrap();
        let mut entry = manifest.packs[0].clone();
        // Tests serve over http; the manifest itself only allows https URLs.
        entry.url = url.to_owned();
        entry
    }

    #[derive(Clone)]
    struct Server {
        body: Arc<Vec<u8>>,
        requests: Arc<AtomicUsize>,
        /// The first response breaks off after this many bytes.
        cut_first_at: Option<usize>,
        honor_range: bool,
    }

    async fn serve_pack(State(server): State<Server>, headers: HeaderMap) -> Response {
        let n = server.requests.fetch_add(1, Ordering::SeqCst);
        let body = server.body.clone();
        let from = headers
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("bytes="))
            .and_then(|v| v.strip_suffix('-'))
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|_| server.honor_range);
        if let (0, Some(cut)) = (n, server.cut_first_at) {
            // Announce the whole body, send part of it, then fail the stream
            // (after a pause, so the first part reaches the client).
            use futures_util::StreamExt;
            let chunks: Vec<Result<bytes::Bytes, std::io::Error>> = vec![
                Ok(bytes::Bytes::copy_from_slice(&body[..cut])),
                Err(std::io::Error::other("connection lost")),
            ];
            let stream = futures_util::stream::iter(chunks).then(|item| async move {
                if item.is_err() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                item
            });
            return Response::builder()
                .header(header::CONTENT_LENGTH, body.len())
                .body(Body::from_stream(stream))
                .unwrap();
        }
        match from {
            Some(from) => (
                AxumStatus::PARTIAL_CONTENT,
                [(
                    header::CONTENT_RANGE,
                    format!("bytes {from}-{}/{}", body.len() - 1, body.len()),
                )],
                body[from..].to_vec(),
            )
                .into_response(),
            None => body.to_vec().into_response(),
        }
    }

    async fn server(cut_first_at: Option<usize>, honor_range: bool) -> (String, Server) {
        let server = Server {
            body: Arc::new(body()),
            requests: Arc::new(AtomicUsize::new(0)),
            cut_first_at,
            honor_range,
        };
        let router = Router::new()
            .route("/en.test.sqlite.zst", get(serve_pack))
            .route("/huge", get(|| async { vec![0u8; BODY_LEN + 10] }))
            .with_state(server.clone());
        (serve(router).await, server)
    }

    fn no_progress() -> impl Fn(Phase, u64, u64) + Send + Sync {
        |_, _, _| {}
    }

    #[tokio::test]
    async fn downloads_and_verifies() {
        let (base, _server) = server(None, true).await;
        let entry = pack_entry(&format!("{base}/en.test.sqlite.zst"), &body());
        let dir = tempfile::tempdir().unwrap();
        let (net, _log) = local_net();
        let phases = std::sync::Mutex::new(Vec::new());
        let progress = |phase, done, total| phases.lock().unwrap().push((phase, done, total));
        let path = fetch(
            &net,
            &PackSource::Remote,
            &entry,
            dir.path(),
            &CancellationToken::new(),
            &progress,
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body());
        assert!(!part_path(dir.path(), &entry).exists());
        let phases = phases.into_inner().unwrap();
        assert_eq!(
            phases.last(),
            Some(&(Phase::Verify, BODY_LEN as u64, BODY_LEN as u64))
        );
        assert!(phases.contains(&(Phase::Download, BODY_LEN as u64, BODY_LEN as u64)));
    }

    #[tokio::test]
    async fn resumes_an_interrupted_download_with_range() {
        let (base, server) = server(Some(100_000), true).await;
        let entry = pack_entry(&format!("{base}/en.test.sqlite.zst"), &body());
        let dir = tempfile::tempdir().unwrap();
        let (net, log) = local_net();
        let path = fetch(
            &net,
            &PackSource::Remote,
            &entry,
            dir.path(),
            &CancellationToken::new(),
            &no_progress(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), body());
        assert_eq!(server.requests.load(Ordering::SeqCst), 2);
        let statuses: Vec<_> = log.0.lock().unwrap().iter().map(|e| e.2).collect();
        assert_eq!(statuses, [Some(200), Some(206)]);
    }

    #[tokio::test]
    async fn restarts_when_the_server_ignores_the_range() {
        let (base, server) = server(Some(100_000), false).await;
        let entry = pack_entry(&format!("{base}/en.test.sqlite.zst"), &body());
        let dir = tempfile::tempdir().unwrap();
        let (net, _log) = local_net();
        let path = fetch(
            &net,
            &PackSource::Remote,
            &entry,
            dir.path(),
            &CancellationToken::new(),
            &no_progress(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), body());
        assert_eq!(server.requests.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn rejects_a_hash_mismatch_and_deletes_the_file() {
        let (base, _server) = server(None, true).await;
        let mut entry = pack_entry(&format!("{base}/en.test.sqlite.zst"), &body());
        entry.sha256 = "0".repeat(64);
        let dir = tempfile::tempdir().unwrap();
        let (net, _log) = local_net();
        let err = fetch(
            &net,
            &PackSource::Remote,
            &entry,
            dir.path(),
            &CancellationToken::new(),
            &no_progress(),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, AppErrorCode::PackHashMismatch);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn refuses_bodies_larger_than_the_manifest_says() {
        let (base, _server) = server(None, true).await;
        let entry = pack_entry(&format!("{base}/huge"), &body());
        let dir = tempfile::tempdir().unwrap();
        let (net, _log) = local_net();
        let err = fetch(
            &net,
            &PackSource::Remote,
            &entry,
            dir.path(),
            &CancellationToken::new(),
            &no_progress(),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, AppErrorCode::PackTooLarge);
    }

    #[tokio::test]
    async fn cancelling_stops_and_deletes_the_partial_file() {
        let (base, _server) = server(None, true).await;
        let entry = pack_entry(&format!("{base}/en.test.sqlite.zst"), &body());
        let dir = tempfile::tempdir().unwrap();
        let (net, _log) = local_net();
        let cancel = CancellationToken::new();
        let cancel_on_progress = |_: Phase, done: u64, _: u64| {
            if done > 0 {
                cancel.cancel();
            }
        };
        let err = fetch(
            &net,
            &PackSource::Remote,
            &entry,
            dir.path(),
            &cancel,
            &cancel_on_progress,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, AppErrorCode::InstallCancelled);
        assert!(!part_path(dir.path(), &entry).exists());
    }

    #[tokio::test]
    async fn copies_from_a_local_source_without_network() {
        let source_dir = tempfile::tempdir().unwrap();
        std::fs::write(source_dir.path().join("en.test.sqlite.zst"), body()).unwrap();
        let entry = pack_entry(
            "https://github.com/siyak/siyak-data/releases/download/t/en.test.sqlite.zst",
            &body(),
        );
        let dir = tempfile::tempdir().unwrap();
        let (net, log) = local_net();
        let path = fetch(
            &net,
            &PackSource::Local(source_dir.path().into()),
            &entry,
            dir.path(),
            &CancellationToken::new(),
            &no_progress(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), body());
        assert!(log.0.lock().unwrap().is_empty());
    }

    #[test]
    fn checks_free_disk_space_first() {
        let entry = pack_entry("https://github.com/x/en.test.sqlite.zst", &body());
        let plenty: DiskSpace = |_| Ok(u64::MAX);
        let little: DiskSpace = |_| Ok(1 << 20);
        let dir = Path::new(".");
        check_disk_space(dir, &entry, plenty).unwrap();
        assert_eq!(
            check_disk_space(dir, &entry, little).unwrap_err().code,
            AppErrorCode::DiskSpaceInsufficient
        );
        assert!(required_space(&entry) > entry.size + 2 * entry.installed_size);
    }
}
