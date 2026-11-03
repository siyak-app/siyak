//! The pack queue (spec §6.4): requests are checked against the catalog, then
//! run one at a time: free-space check → download → verify → install. Progress,
//! completion and failure are reported through [`PackEvents`] (the
//! `pack://progress`, `pack://done` and `pack://error` events in the app).
//!
//! Installs, removals and `VACUUM` each use their own `content.db` connection,
//! serialized by one write lock, so the shared reader connection is never held
//! for the length of an install. The service knows nothing about Tauri; the
//! commands in `commands/packs.rs` pass it the installed packs and terms
//! acknowledgements they read from the shared connections.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use specta::Type;
use tokio_util::sync::CancellationToken;

use super::download::{self, DiskSpace};
use super::install::{self, Installed, VacuumResult};
use super::manifest::{
    self, Catalog, CatalogInput, InstalledPack, JobView, ManifestStore, PackEntry, PackView, Phase,
    Snapshot,
};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::Net;

/// Event with a [`PackProgress`] payload.
pub const PROGRESS_EVENT: &str = "pack://progress";
/// Event with a [`PackDone`] payload.
pub const DONE_EVENT: &str = "pack://done";
/// Event with a [`PackFailed`] payload.
pub const ERROR_EVENT: &str = "pack://error";

/// Shortest interval between two progress events of the same phase.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// A job was queued or moved on.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct PackProgress {
    /// Pack id.
    pub id: String,
    /// The job's state.
    pub job: JobView,
}

/// A pack was installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct PackDone {
    /// Pack id.
    pub id: String,
    /// The morphology pack it replaced, if any.
    pub replaced: Option<String>,
}

/// A job failed or was cancelled (`install_cancelled`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct PackFailed {
    /// Pack id.
    pub id: String,
    /// Why.
    pub error: AppError,
}

/// Where job updates go (Tauri events in the app).
pub trait PackEvents: Send + Sync {
    /// A job was queued or made progress.
    fn progress(&self, event: PackProgress);
    /// A job finished.
    fn done(&self, event: PackDone);
    /// A job failed or was cancelled.
    fn failed(&self, event: PackFailed);
}

/// Paths and helpers the service works with.
pub struct PacksConfig {
    /// `<app_data>`: holds `downloads/`, `tmp/`, `packs/` and `cache/`.
    pub data_dir: PathBuf,
    /// The content database file.
    pub content_path: PathBuf,
    /// Free space of a volume (`fs4::available_space` in the app).
    pub disk: DiskSpace,
}

/// The pack service shared by the pack commands.
pub struct Packs {
    inner: Arc<Inner>,
}

struct Inner {
    net: Net,
    store: ManifestStore,
    config: PacksConfig,
    events: Arc<dyn PackEvents>,
    queue: Mutex<Queue>,
    write: tokio::sync::Mutex<()>,
}

#[derive(Default)]
struct Queue {
    /// The running job (if any) is first.
    jobs: VecDeque<Job>,
    worker: bool,
}

struct Job {
    entry: PackEntry,
    cancel: CancellationToken,
    view: JobView,
}

impl Packs {
    /// Creates the service.
    pub fn new(
        net: Net,
        store: ManifestStore,
        config: PacksConfig,
        events: Arc<dyn PackEvents>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                net,
                store,
                config,
                events,
                queue: Mutex::new(Queue::default()),
                write: tokio::sync::Mutex::new(()),
            }),
        }
    }

    /// The manifest, fetched again if stale or `force` is set.
    pub async fn manifest(&self, force: bool) -> Snapshot {
        self.inner.store.load(&self.inner.net, force).await
    }

    /// The Packs screen for `snapshot`, with the current jobs.
    pub fn catalog(
        &self,
        snapshot: &Snapshot,
        installed: &[InstalledPack],
        acks: &[(String, String)],
    ) -> Catalog {
        let jobs: HashMap<String, JobView> = self
            .inner
            .queue()
            .jobs
            .iter()
            .map(|job| (job.entry.id.clone(), job.view.clone()))
            .collect();
        manifest::catalog(&CatalogInput {
            source: self.inner.store.source(),
            snapshot,
            installed,
            acks,
            jobs: &jobs,
            app_version: &manifest::app_version(),
        })
    }

    /// Queues `ids` (and the packs they require) after checking each one.
    /// Nothing is queued if any check fails. `confirm_replace` confirms that
    /// a morphology pack replaces the installed one.
    pub fn install(
        &self,
        catalog: &Catalog,
        snapshot: &Snapshot,
        ids: &[String],
        confirm_replace: bool,
    ) -> AppResult<()> {
        let manifest = snapshot.manifest.as_deref().ok_or_else(|| {
            snapshot
                .error
                .clone()
                .unwrap_or_else(|| AppError::new(AppErrorCode::ManifestUnavailable, "no manifest"))
        })?;
        let views: HashMap<&str, &PackView> =
            catalog.packs.iter().map(|p| (p.id.as_str(), p)).collect();
        let mut order: Vec<&str> = Vec::new();
        for id in ids {
            resolve(id, manifest, &views, &mut order, 0)?;
        }
        for id in &order {
            let view = views.get(id).copied().ok_or_else(|| not_found(id))?;
            check(view, confirm_replace)?;
        }

        let entries: Vec<PackEntry> = order
            .iter()
            .filter_map(|id| manifest.pack(id).cloned())
            .collect();
        let start_worker = {
            let mut queue = self.inner.queue();
            if let Some(busy) = entries.iter().find(|e| queue.contains(&e.id)) {
                return Err(busy_error(&busy.id));
            }
            for entry in entries {
                self.inner.events.progress(PackProgress {
                    id: entry.id.clone(),
                    job: JobView::Queued,
                });
                queue.jobs.push_back(Job {
                    entry,
                    cancel: CancellationToken::new(),
                    view: JobView::Queued,
                });
            }
            !std::mem::replace(&mut queue.worker, true)
        };
        if start_worker {
            tokio::spawn(run(Arc::clone(&self.inner)));
        }
        Ok(())
    }

    /// The members of bundle `id` that still need installing (and are not queued).
    pub fn bundle_members(&self, catalog: &Catalog, id: &str) -> AppResult<Vec<String>> {
        let bundle = catalog
            .bundles
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| not_found(id))?;
        let queue = self.inner.queue();
        Ok(bundle
            .missing
            .iter()
            .filter(|m| !queue.contains(m))
            .cloned()
            .collect())
    }

    /// Cancels a queued or running job.
    pub fn cancel(&self, id: &str) -> AppResult<()> {
        let mut queue = self.inner.queue();
        let index = queue
            .jobs
            .iter()
            .position(|job| job.entry.id == id)
            .ok_or_else(|| not_found(id))?;
        let running = index == 0 && matches!(queue.jobs[0].view, JobView::Running { .. });
        if running {
            // The worker reports the failure once the job has stopped.
            queue.jobs[0].cancel.cancel();
        } else {
            queue.jobs.remove(index);
            self.inner.events.failed(PackFailed {
                id: id.to_owned(),
                error: cancelled(),
            });
        }
        Ok(())
    }

    /// Removes an installed pack.
    pub async fn uninstall(&self, id: &str) -> AppResult<()> {
        if self.inner.queue().contains(id) {
            return Err(busy_error(id));
        }
        let _write = self.inner.write.lock().await;
        let (content, data_dir, id) = (
            self.inner.config.content_path.clone(),
            self.inner.config.data_dir.clone(),
            id.to_owned(),
        );
        blocking(move || {
            let mut conn = install::open_content(&content)?;
            install::uninstall(&mut conn, &data_dir, &id)
        })
        .await
    }

    /// Compacts `content.db` (suggested after a removal).
    pub async fn vacuum(&self) -> AppResult<VacuumResult> {
        let _write = self.inner.write.lock().await;
        let content = self.inner.config.content_path.clone();
        blocking(move || install::vacuum(&content)).await
    }

    /// Ids of queued and running jobs, in order.
    pub fn jobs(&self) -> Vec<String> {
        self.inner
            .queue()
            .jobs
            .iter()
            .map(|job| job.entry.id.clone())
            .collect()
    }
}

impl Inner {
    fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Updates the running job's view and reports it.
    fn report(&self, id: &str, view: JobView) {
        let mut queue = self.queue();
        if let Some(job) = queue.jobs.front_mut().filter(|job| job.entry.id == id) {
            job.view = view.clone();
        }
        drop(queue);
        self.events.progress(PackProgress {
            id: id.to_owned(),
            job: view,
        });
    }

    async fn process(
        self: &Arc<Self>,
        entry: &PackEntry,
        cancel: &CancellationToken,
    ) -> AppResult<Installed> {
        let data_dir = &self.config.data_dir;
        download::check_disk_space(data_dir, entry, self.config.disk)?;
        let progress = throttled_progress(Arc::clone(self), entry.id.clone());
        let file = download::fetch(
            &self.net,
            self.store.source(),
            entry,
            &install::downloads_dir(data_dir),
            cancel,
            &*progress,
        )
        .await?;

        let _write = self.write.lock().await;
        let (content, data_dir, entry, cancel, path) = (
            self.config.content_path.clone(),
            data_dir.clone(),
            entry.clone(),
            cancel.clone(),
            file.clone(),
        );
        let result = blocking(move || {
            let mut conn = install::open_content(&content)?;
            install::install_file(&mut conn, &path, &entry, &data_dir, &cancel, &*progress)
        })
        .await;
        let _ = tokio::fs::remove_file(&file).await;
        result
    }
}

impl Queue {
    fn contains(&self, id: &str) -> bool {
        self.jobs.iter().any(|job| job.entry.id == id)
    }
}

/// Runs queued jobs until the queue is empty.
async fn run(inner: Arc<Inner>) {
    loop {
        let (entry, cancel) = {
            let mut queue = inner.queue();
            let Some(job) = queue.jobs.front_mut() else {
                queue.worker = false;
                return;
            };
            // Marked running under the lock, so `cancel` sees it from now on.
            job.view = JobView::Running {
                phase: Phase::Download,
                done: 0.0,
                total: job.entry.size as f64,
            };
            (job.entry.clone(), job.cancel.clone())
        };
        inner.report(
            &entry.id,
            JobView::Running {
                phase: Phase::Download,
                done: 0.0,
                total: entry.size as f64,
            },
        );
        let result = inner.process(&entry, &cancel).await;
        {
            let mut queue = inner.queue();
            if queue
                .jobs
                .front()
                .is_some_and(|job| job.entry.id == entry.id)
            {
                queue.jobs.pop_front();
            }
        }
        match result {
            Ok(installed) => inner.events.done(PackDone {
                id: entry.id,
                replaced: installed.replaced,
            }),
            Err(error) => inner.events.failed(PackFailed {
                id: entry.id,
                error,
            }),
        }
    }
}

type SharedProgress = Arc<dyn Fn(Phase, u64, u64) + Send + Sync>;

fn throttled_progress(inner: Arc<Inner>, id: String) -> SharedProgress {
    let last: Mutex<Option<(Phase, Instant)>> = Mutex::new(None);
    Arc::new(move |phase, done, total| {
        let now = Instant::now();
        {
            let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
            let due = match *last {
                Some((p, at)) => p != phase || done >= total || now - at >= PROGRESS_INTERVAL,
                None => true,
            };
            if !due {
                return;
            }
            *last = Some((phase, now));
        }
        inner.report(
            &id,
            JobView::Running {
                phase,
                done: done as f64,
                total: total as f64,
            },
        );
    })
}

/// Adds `id`'s requirements, then `id`, to `order` (each once).
fn resolve<'m>(
    id: &'m str,
    manifest: &'m manifest::Manifest,
    views: &HashMap<&str, &PackView>,
    order: &mut Vec<&'m str>,
    depth: usize,
) -> AppResult<()> {
    if order.contains(&id) {
        return Ok(());
    }
    let entry = manifest.pack(id).ok_or_else(|| not_found(id))?;
    if depth > 8 {
        return Err(AppError::new(
            AppErrorCode::ManifestInvalid,
            format!("{id}: requirements nest too deeply"),
        ));
    }
    for required in &entry.requires {
        // Installed (and current) requirements are not installed again.
        let current = views
            .get(required.as_str())
            .is_some_and(|v| v.installed && !v.update_available);
        if !current {
            resolve(required, manifest, views, order, depth + 1)?;
        }
    }
    if !order.contains(&entry.id.as_str()) {
        order.push(entry.id.as_str());
    }
    Ok(())
}

/// Whether the catalog allows installing `view` now.
fn check(view: &PackView, confirm_replace: bool) -> AppResult<()> {
    let id = &view.id;
    let fail = |code, message: &str| Err(AppError::new(code, format!("{id}: {message}")));
    if view.bundled {
        return fail(AppErrorCode::PackBundled, "ships with the app");
    }
    if !view.in_manifest {
        return fail(AppErrorCode::NotFound, "not in the manifest");
    }
    if !view.supported {
        return fail(
            AppErrorCode::PackUnsupported,
            &format!("type {}", view.kind),
        );
    }
    if view.incompatible {
        return fail(
            AppErrorCode::PackIncompatible,
            &format!("needs app {}", view.min_app_version),
        );
    }
    if view.job.is_some() {
        return Err(busy_error(id));
    }
    if view.terms_ack_required && !view.terms_acknowledged {
        return fail(AppErrorCode::TermsNotAcknowledged, "terms not acknowledged");
    }
    if let Some(replaced) = &view.replaces
        && !confirm_replace
    {
        return fail(
            AppErrorCode::MorphologyReplaceRequired,
            &format!("replaces {replaced}"),
        );
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|err| AppError::new(AppErrorCode::IoFailed, format!("pack task: {err}")))?
}

fn not_found(id: &str) -> AppError {
    AppError::new(
        AppErrorCode::NotFound,
        format!("{id}: unknown pack or bundle"),
    )
}

fn busy_error(id: &str) -> AppError {
    AppError::new(AppErrorCode::PackBusy, format!("{id}: already queued"))
}

fn cancelled() -> AppError {
    AppError::new(AppErrorCode::InstallCancelled, "cancelled")
}

/// The free space of `path`'s volume.
pub fn available_space(path: &Path) -> std::io::Result<u64> {
    fs4::available_space(path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use sha2::{Digest, Sha256};

    use super::*;
    use crate::net::test_support::local_net;
    use crate::packs::download::hex;
    use crate::packs::manifest::tests::{entry, manifest_json};
    use crate::packs::manifest::{PackSource, installed_packs};
    use crate::packs::test_support::{compress, content_db, translation_db};

    struct Recorder {
        /// One line per state change: `<id> <state>`.
        events: Mutex<Vec<String>>,
        finished: tokio::sync::Semaphore,
    }

    impl PackEvents for Recorder {
        fn progress(&self, event: PackProgress) {
            let state = match event.job {
                JobView::Queued => "queued".to_owned(),
                JobView::Running { phase, .. } => format!("{phase:?}"),
            };
            let mut events = self.events.lock().unwrap();
            let line = format!("{} {state}", event.id);
            // Progress repeats within a phase; keep one line per state.
            if events.last() != Some(&line) {
                events.push(line);
            }
        }
        fn done(&self, event: PackDone) {
            self.events
                .lock()
                .unwrap()
                .push(format!("{} done", event.id));
            self.finished.add_permits(1);
        }
        fn failed(&self, event: PackFailed) {
            self.events
                .lock()
                .unwrap()
                .push(format!("{} {:?}", event.id, event.error.code));
            self.finished.add_permits(1);
        }
    }

    struct Env {
        dir: tempfile::TempDir,
        packs: Packs,
        events: Arc<Recorder>,
        acks: Vec<(String, String)>,
    }

    impl Env {
        /// A local pack source holding translation packs `tr.one` and `tr.two`
        /// (the second needs its terms acknowledged) and a bundle of both.
        fn new(disk: DiskSpace) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("source");
            fs::create_dir_all(&source).unwrap();
            let mut packs = Vec::new();
            for (id, terms) in [("tr.one", false), ("tr.two", true)] {
                let db = source.join(format!("{id}.sqlite"));
                drop(translation_db(&db, id, "1.0.0", &[(1, id)]));
                let zst = compress(&db);
                let data = fs::read(&zst).unwrap();
                let mut value = entry(id, "translation", "1.0.0");
                value["size"] = data.len().into();
                value["installed_size"] = fs::metadata(&db).unwrap().len().into();
                value["sha256"] = hex(&Sha256::digest(&data)).into();
                value["terms_ack"] = terms.into();
                packs.push(value);
            }
            let mut manifest = manifest_json(packs);
            manifest["bundles"][0]["packs"] = serde_json::json!(["tr.one", "tr.two"]);
            fs::write(
                source.join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();

            let data_dir = dir.path().join("data");
            fs::create_dir_all(&data_dir).unwrap();
            let content_path = data_dir.join("content.db");
            drop(content_db(&content_path));
            let events = Arc::new(Recorder {
                events: Mutex::default(),
                finished: tokio::sync::Semaphore::new(0),
            });
            let (net, _log) = local_net();
            let packs = Packs::new(
                net,
                ManifestStore::new(PackSource::Local(source), &data_dir),
                PacksConfig {
                    data_dir,
                    content_path,
                    disk,
                },
                Arc::clone(&events) as Arc<dyn PackEvents>,
            );
            Self {
                dir,
                packs,
                events,
                acks: Vec::new(),
            }
        }

        fn installed(&self) -> Vec<InstalledPack> {
            let conn = install::open_content(&self.packs.inner.config.content_path).unwrap();
            installed_packs(&conn).unwrap()
        }

        async fn catalog(&self) -> (Snapshot, Catalog) {
            let snapshot = self.packs.manifest(false).await;
            let catalog = self.packs.catalog(&snapshot, &self.installed(), &self.acks);
            (snapshot, catalog)
        }

        async fn install(&self, ids: &[&str]) -> AppResult<()> {
            let (snapshot, catalog) = self.catalog().await;
            let ids: Vec<String> = ids.iter().map(|s| (*s).to_owned()).collect();
            self.packs.install(&catalog, &snapshot, &ids, false)
        }

        async fn finished(&self, jobs: usize) {
            let finished = self.events.finished.acquire_many(jobs as u32);
            tokio::time::timeout(Duration::from_secs(10), finished)
                .await
                .unwrap()
                .unwrap()
                .forget();
        }

        fn events(&self) -> Vec<String> {
            self.events.events.lock().unwrap().clone()
        }

        fn is_installed(&self, id: &str) -> bool {
            self.installed().iter().any(|p| p.id == id)
        }
    }

    fn plenty(_: &Path) -> std::io::Result<u64> {
        Ok(u64::MAX)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn installs_a_pack_through_every_phase() {
        let env = Env::new(plenty);
        env.install(&["tr.one"]).await.unwrap();
        env.finished(1).await;
        assert_eq!(
            env.events(),
            [
                "tr.one queued",
                "tr.one Download",
                "tr.one Verify",
                "tr.one Install",
                "tr.one done"
            ]
        );
        assert!(env.is_installed("tr.one"));
        assert!(env.packs.jobs().is_empty());
        let downloads = install::downloads_dir(&env.dir.path().join("data"));
        assert_eq!(fs::read_dir(downloads).unwrap().count(), 0);
        let (_, catalog) = env.catalog().await;
        assert!(
            catalog
                .packs
                .iter()
                .find(|p| p.id == "tr.one")
                .unwrap()
                .installed
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn checks_terms_and_queues_bundles_in_order() {
        let mut env = Env::new(plenty);
        let (snapshot, catalog) = env.catalog().await;
        let members = env.packs.bundle_members(&catalog, "recommended").unwrap();
        assert_eq!(members, ["tr.one", "tr.two"]);
        let err = env
            .packs
            .install(&catalog, &snapshot, &members, false)
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::TermsNotAcknowledged);
        assert!(env.packs.jobs().is_empty(), "nothing queued");

        env.acks.push(("tr.two".to_owned(), "1.0.0".to_owned()));
        let (snapshot, catalog) = env.catalog().await;
        env.packs
            .install(&catalog, &snapshot, &members, false)
            .unwrap();
        env.finished(2).await;
        assert!(env.is_installed("tr.one") && env.is_installed("tr.two"));
        let events = env.events();
        let done: Vec<_> = events.iter().filter(|e| e.ends_with("done")).collect();
        assert_eq!(done, ["tr.one done", "tr.two done"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn refuses_bundled_unknown_and_busy_packs() {
        let env = Env::new(plenty);
        let (snapshot, mut catalog) = env.catalog().await;
        let ids = |ids: &[&str]| ids.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            env.packs
                .install(&catalog, &snapshot, &ids(&["nope.pack"]), false)
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
        catalog.packs[0].job = Some(JobView::Queued);
        assert_eq!(
            env.packs
                .install(&catalog, &snapshot, &ids(&["tr.one"]), false)
                .unwrap_err()
                .code,
            AppErrorCode::PackBusy
        );
        catalog.packs[0].job = None;
        catalog.packs[0].bundled = true;
        assert_eq!(
            env.packs
                .install(&catalog, &snapshot, &ids(&["tr.one"]), false)
                .unwrap_err()
                .code,
            AppErrorCode::PackBundled
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fails_early_without_disk_space() {
        let env = Env::new(|_| Ok(1));
        env.install(&["tr.one"]).await.unwrap();
        env.finished(1).await;
        assert_eq!(env.events().last().unwrap(), "tr.one DiskSpaceInsufficient");
        assert!(!env.is_installed("tr.one"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancels_queued_jobs_and_uninstalls() {
        let mut env = Env::new(plenty);
        env.acks.push(("tr.two".to_owned(), "1.0.0".to_owned()));
        env.install(&["tr.one", "tr.two"]).await.unwrap();
        // tr.two waits behind tr.one, unless tr.one already finished.
        if env.packs.cancel("tr.two").is_ok() {
            env.finished(2).await;
            assert!(env.events().contains(&"tr.two InstallCancelled".to_owned()));
            assert!(!env.is_installed("tr.two"));
        } else {
            env.finished(2).await;
        }
        assert!(env.is_installed("tr.one"));

        env.packs.uninstall("tr.one").await.unwrap();
        assert!(!env.is_installed("tr.one"));
        assert_eq!(
            env.packs.uninstall("en.core").await.unwrap_err().code,
            AppErrorCode::PackBundled
        );
        let sizes = env.packs.vacuum().await.unwrap();
        assert!(sizes.after > 0);
        assert_eq!(
            env.packs.cancel("tr.one").unwrap_err().code,
            AppErrorCode::NotFound
        );
    }
}
