//! Manifest fetching, caching (1 hour, last good copy offline), `min_app_version`
//! checks and the pack catalog shown on the Packs screen (spec §6.3–§6.4,
//! `siyak-data/docs/pack-format.md` §6).
//!
//! The manifest is untrusted input: the document size is capped, its
//! `schema_version` must be one this app knows, and every pack entry is
//! validated (ids are used in file names, sizes as limits). Unknown fields are
//! ignored so that newer manifests stay readable; packs of unknown types are
//! listed as unsupported instead of failing the whole manifest.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::db::content::LocalizedTitle;
use crate::embed::space::{ModelPackSpec, VectorPackSpec};
use crate::error::{AppError, AppErrorCode, AppResult};
use crate::net::{Net, Purpose};

/// The manifest format this app reads (`schema_version`).
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Primary manifest address, independent of the GitHub organization (spec §6.3).
pub const PRIMARY_MANIFEST_URL: &str = "https://quran.kemalozyon.com/data/manifest.json";

/// Fallback manifest address: the latest data release.
pub const FALLBACK_MANIFEST_URL: &str =
    "https://github.com/siyak-app/siyak-data/releases/latest/download/manifest.json";

/// Largest manifest document accepted.
pub const MAX_MANIFEST_BYTES: usize = 2 << 20;

/// Largest pack file (compressed or installed) a manifest may announce.
pub const MAX_PACK_BYTES: u64 = 4 << 30;

/// How long a fetched manifest is reused before it is fetched again.
pub const MANIFEST_MAX_AGE: Duration = Duration::from_secs(60 * 60);

/// File name of the cached manifest inside `<app_data>/cache/`.
const CACHE_FILE: &str = "manifest.json";

/// Pack types this app can install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackKind {
    /// `translations` rows plus FTS.
    Translation,
    /// `tafsir` passages and paragraphs.
    Tafsir,
    /// The morphology tables, shipped in an archive with extra files.
    Morphology,
    /// Verse embeddings of one vector space (spec §6.1, ADR 0012).
    Vectors,
    /// An ONNX embedding model for local vector packs, shipped in an archive.
    Model,
}

impl PackKind {
    /// The kind for a manifest `type`, if this app supports it.
    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "translation" => Some(Self::Translation),
            "tafsir" => Some(Self::Tafsir),
            "morphology" => Some(Self::Morphology),
            "vectors" => Some(Self::Vectors),
            "model" => Some(Self::Model),
            _ => None,
        }
    }

    /// The manifest `type` value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Translation => "translation",
            Self::Tafsir => "tafsir",
            Self::Morphology => "morphology",
            Self::Vectors => "vectors",
            Self::Model => "model",
        }
    }

    /// Whether packs of this kind are `.tar.zst` archives (pack-format §4.4).
    pub fn is_archive(self) -> bool {
        matches!(self, Self::Morphology | Self::Model)
    }

    /// The file name a pack of this kind is published under.
    pub fn file_name(self, id: &str) -> String {
        if self.is_archive() {
            format!("{id}.tar.zst")
        } else {
            format!("{id}.sqlite.zst")
        }
    }
}

/// A `{ "tr", "en" }` text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Type)]
pub struct LocalizedText {
    /// Turkish.
    pub tr: String,
    /// English.
    pub en: String,
}

/// A `{ "tr": [...], "en": [...] }` list, e.g. `license_terms`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, Type)]
pub struct LocalizedList {
    /// Turkish items.
    pub tr: Vec<String>,
    /// English items.
    pub en: Vec<String>,
}

/// One pack of the manifest, validated.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PackEntry {
    /// Pack id, e.g. `ar.jalalayn`.
    pub id: String,
    /// Pack type as written in the manifest.
    #[serde(rename = "type")]
    pub kind: String,
    /// BCP 47 language of the content.
    #[serde(default)]
    pub lang: Option<String>,
    /// Title in the interface languages.
    pub title: LocalizedText,
    /// One-sentence description in the interface languages.
    #[serde(default)]
    pub description: Option<LocalizedText>,
    /// Author (or authors) of the work.
    #[serde(default)]
    pub author: Option<String>,
    /// SemVer version of the pack.
    pub version: String,
    /// Oldest app version that can use the pack.
    #[serde(default = "default_min_app_version")]
    pub min_app_version: String,
    /// Size of the downloaded file.
    pub size: u64,
    /// Decompressed size: the decompression limit and the disk-space estimate.
    pub installed_size: u64,
    /// sha256 of the downloaded file, lowercase hex.
    pub sha256: String,
    /// Release asset address of the file.
    pub url: String,
    /// License name.
    pub license: String,
    /// Short list of terms shown before download.
    #[serde(default)]
    pub license_terms: Option<LocalizedList>,
    /// Attribution that must be shown with the content.
    #[serde(default)]
    pub attribution: Option<String>,
    /// Citation metadata (spec §11.2).
    #[serde(default)]
    pub citation: Option<serde_json::Value>,
    /// Type-specific metadata, merged into `packs.meta_json`.
    #[serde(default)]
    pub meta: Option<serde_json::Map<String, serde_json::Value>>,
    /// Packs this one needs (vectors → model, M7).
    #[serde(default)]
    pub requires: Vec<String>,
    /// Whether the user must confirm the terms before the first download.
    #[serde(default)]
    pub terms_ack: bool,
}

fn default_min_app_version() -> String {
    "0.1.0".to_owned()
}

impl PackEntry {
    /// The pack's kind, if this app supports its type.
    ///
    /// Vector and model packs also need valid embedding metadata; with invalid
    /// metadata they are listed as unsupported instead of failing the manifest.
    pub fn pack_kind(&self) -> Option<PackKind> {
        let kind = PackKind::parse(&self.kind)?;
        let empty = serde_json::Map::new();
        let meta = self.meta.as_ref().unwrap_or(&empty);
        let valid = match kind {
            PackKind::Vectors => VectorPackSpec::from_meta(meta)
                .and_then(|spec| spec.check_requires(&self.requires))
                .is_ok(),
            PackKind::Model => ModelPackSpec::from_meta(meta).is_ok(),
            PackKind::Translation | PackKind::Tafsir | PackKind::Morphology => true,
        };
        valid.then_some(kind)
    }

    /// The last path segment of `url`: the pack's file name.
    pub fn file_name(&self) -> &str {
        self.url.rsplit('/').next().unwrap_or_default()
    }
}

/// A set of packs installed with one action (spec §6.3).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Bundle {
    /// Bundle id, e.g. `recommended`.
    pub id: String,
    /// Title in the interface languages.
    pub title: LocalizedText,
    /// Description in the interface languages.
    #[serde(default)]
    pub description: Option<LocalizedText>,
    /// Member pack ids, in install order.
    pub packs: Vec<String>,
}

/// A validated manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// Every pack, in manifest order.
    pub packs: Vec<PackEntry>,
    /// Bundles with at least one member present in `packs`.
    pub bundles: Vec<Bundle>,
}

impl Manifest {
    /// The pack with `id`.
    pub fn pack(&self, id: &str) -> Option<&PackEntry> {
        self.packs.iter().find(|p| p.id == id)
    }

    /// The bundle with `id`.
    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        self.bundles.iter().find(|b| b.id == id)
    }
}

#[derive(Deserialize)]
struct RawManifest {
    schema_version: serde_json::Value,
    #[serde(default)]
    packs: Vec<PackEntry>,
    #[serde(default)]
    bundles: Vec<Bundle>,
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::new(AppErrorCode::ManifestInvalid, message)
}

/// Parses and validates a manifest document.
pub fn parse(bytes: &[u8]) -> AppResult<Manifest> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(invalid(format!(
            "manifest is {} bytes, limit {MAX_MANIFEST_BYTES}",
            bytes.len()
        )));
    }
    // The version is read first so that a future format fails with a clear
    // code even if the rest no longer parses.
    let version: serde_json::Value = serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|err| invalid(format!("manifest: {err}")))?
        .get("schema_version")
        .cloned()
        .unwrap_or_default();
    match version.as_u64() {
        Some(v) if v == u64::from(MANIFEST_SCHEMA_VERSION) => {}
        Some(v) => {
            return Err(AppError::new(
                AppErrorCode::ManifestUnsupported,
                format!("manifest schema_version {v}, this app reads {MANIFEST_SCHEMA_VERSION}"),
            ));
        }
        None => return Err(invalid("manifest has no integer schema_version")),
    }
    let raw: RawManifest =
        serde_json::from_slice(bytes).map_err(|err| invalid(format!("manifest: {err}")))?;
    debug_assert!(raw.schema_version.is_u64());

    let mut seen = HashSet::new();
    for pack in &raw.packs {
        validate_pack(pack)?;
        if !seen.insert(pack.id.as_str()) {
            return Err(invalid(format!("pack {} is listed twice", pack.id)));
        }
    }
    let mut bundles = Vec::new();
    let mut bundle_ids = HashSet::new();
    for mut bundle in raw.bundles {
        if !is_valid_id(&bundle.id) || !bundle_ids.insert(bundle.id.clone()) {
            return Err(invalid(format!(
                "invalid or duplicate bundle id {:?}",
                bundle.id
            )));
        }
        let mut members = HashSet::new();
        bundle
            .packs
            .retain(|id| seen.contains(id.as_str()) && members.insert(id.clone()));
        if !bundle.packs.is_empty() {
            bundles.push(bundle);
        }
    }
    Ok(Manifest {
        packs: raw.packs,
        bundles,
    })
}

/// `^[a-z0-9]+(\.[a-z0-9-]+)+$` (bundle ids may be a single segment), at
/// most 64 bytes. Pack ids become file and directory names.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.split('.').enumerate().all(|(i, part)| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b == b'-'))
        })
}

fn is_valid_pack_id(id: &str) -> bool {
    is_valid_id(id) && id.contains('.')
}

fn validate_pack(pack: &PackEntry) -> AppResult<()> {
    let id = &pack.id;
    if !is_valid_pack_id(id) {
        return Err(invalid(format!("invalid pack id {id:?}")));
    }
    if pack.sha256.len() != 64
        || !pack
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(format!(
            "{id}: sha256 must be 64 lowercase hex digits"
        )));
    }
    for (name, size) in [("size", pack.size), ("installed_size", pack.installed_size)] {
        if size == 0 || size > MAX_PACK_BYTES {
            return Err(invalid(format!("{id}: {name} {size} is out of range")));
        }
    }
    if !pack.url.starts_with("https://") {
        return Err(invalid(format!("{id}: url must use https")));
    }
    if let Some(kind) = pack.pack_kind()
        && pack.file_name() != kind.file_name(id)
    {
        return Err(invalid(format!(
            "{id}: url must end in {}",
            kind.file_name(id)
        )));
    }
    if semver::Version::parse(&pack.version).is_err()
        || semver::Version::parse(&pack.min_app_version).is_err()
    {
        return Err(invalid(format!(
            "{id}: version and min_app_version must be SemVer"
        )));
    }
    if pack.requires.iter().any(|r| !is_valid_pack_id(r)) {
        return Err(invalid(format!("{id}: invalid id in requires")));
    }
    Ok(())
}

/// The version of this app, for `min_app_version` checks.
pub fn app_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

/// Whether `pack` needs a newer app than `app`.
pub fn is_incompatible(pack: &PackEntry, app: &semver::Version) -> bool {
    semver::Version::parse(&pack.min_app_version).is_ok_and(|min| &min > app)
}

/// Where the manifest and pack files come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSource {
    /// The published manifest; packs from their release `url`s.
    Remote,
    /// A local directory such as `../siyak-data/dist` (development): its
    /// `manifest.json`, and each pack by its file name in the directory.
    Local(PathBuf),
}

impl PackSource {
    /// `remote` or `local`, as shown in the debug badge.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Remote => "remote",
            Self::Local(_) => "local",
        }
    }

    /// Where a local source finds the file of `pack`: its URL's file name
    /// inside the directory (the URL's host is ignored).
    pub fn local_file(&self, pack: &PackEntry) -> Option<PathBuf> {
        match self {
            Self::Local(dir) => Some(dir.join(pack.file_name())),
            Self::Remote => None,
        }
    }
}

/// The manifest as last loaded.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The manifest, if one was ever loaded.
    pub manifest: Option<std::sync::Arc<Manifest>>,
    /// When it was fetched.
    pub fetched_at: Option<SystemTime>,
    /// True if the latest fetch failed and an older copy is used.
    pub offline: bool,
    /// Why the latest fetch failed, if it did.
    pub error: Option<AppError>,
}

/// Loads the manifest from its source, caching the last good copy.
pub struct ManifestStore {
    source: PackSource,
    cache_path: PathBuf,
    urls: Vec<String>,
    state: Mutex<Option<Snapshot>>,
}

impl ManifestStore {
    /// A store for `source` that caches under `<data_dir>/cache/`.
    pub fn new(source: PackSource, data_dir: &Path) -> Self {
        Self::with_urls(
            source,
            data_dir,
            vec![
                PRIMARY_MANIFEST_URL.to_owned(),
                FALLBACK_MANIFEST_URL.to_owned(),
            ],
        )
    }

    /// A store with custom manifest addresses (tests use a local server).
    pub fn with_urls(source: PackSource, data_dir: &Path, urls: Vec<String>) -> Self {
        Self {
            source,
            cache_path: data_dir.join("cache").join(CACHE_FILE),
            urls,
            state: Mutex::new(None),
        }
    }

    /// The pack source.
    pub fn source(&self) -> &PackSource {
        &self.source
    }

    /// The manifest, fetched again if it is older than [`MANIFEST_MAX_AGE`]
    /// or `force` is set. A failed fetch falls back to the last good copy.
    pub async fn load(&self, net: &Net, force: bool) -> Snapshot {
        if let PackSource::Local(dir) = &self.source {
            let result = std::fs::read(dir.join(CACHE_FILE))
                .map_err(|err| {
                    AppError::new(
                        AppErrorCode::ManifestUnavailable,
                        format!("{}: {err}", dir.display()),
                    )
                })
                .and_then(|bytes| parse(&bytes));
            return match result {
                Ok(manifest) => Snapshot {
                    manifest: Some(manifest.into()),
                    fetched_at: Some(SystemTime::now()),
                    offline: false,
                    error: None,
                },
                Err(err) => Snapshot {
                    manifest: None,
                    fetched_at: None,
                    offline: false,
                    error: Some(err),
                },
            };
        }

        let current = self.current();
        if !force
            && let Some(snapshot) = &current
            && snapshot.error.is_none()
            && snapshot
                .fetched_at
                .and_then(|at| at.elapsed().ok())
                .is_some_and(|age| age < MANIFEST_MAX_AGE)
        {
            return snapshot.clone();
        }

        let snapshot = match self.fetch(net).await {
            Ok((manifest, bytes)) => {
                if let Err(err) = self.save_cache(&bytes) {
                    eprintln!("caching the manifest failed: {err}");
                }
                Snapshot {
                    manifest: Some(manifest.into()),
                    fetched_at: Some(SystemTime::now()),
                    offline: false,
                    error: None,
                }
            }
            Err(err) => {
                let cached = current
                    .filter(|s| s.manifest.is_some())
                    .or_else(|| self.read_cache());
                match cached {
                    Some(cached) => Snapshot {
                        offline: true,
                        error: Some(err),
                        ..cached
                    },
                    None => Snapshot {
                        manifest: None,
                        fetched_at: None,
                        offline: true,
                        error: Some(err),
                    },
                }
            }
        };
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = Some(snapshot.clone());
        snapshot
    }

    /// The last loaded snapshot without fetching, falling back to the cache file.
    pub fn current(&self) -> Option<Snapshot> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        state.or_else(|| {
            matches!(self.source, PackSource::Remote)
                .then(|| self.read_cache())
                .flatten()
        })
    }

    async fn fetch(&self, net: &Net) -> AppResult<(Manifest, Vec<u8>)> {
        let mut last_error = AppError::new(AppErrorCode::ManifestUnavailable, "no manifest URL");
        for url in &self.urls {
            match fetch_one(net, url).await {
                Ok(result) => return Ok(result),
                // A manifest from the future is not retried elsewhere.
                Err(err) if err.code == AppErrorCode::ManifestUnsupported => return Err(err),
                Err(err) => last_error = err,
            }
        }
        Err(last_error)
    }

    fn read_cache(&self) -> Option<Snapshot> {
        let bytes = std::fs::read(&self.cache_path).ok()?;
        let manifest = parse(&bytes).ok()?;
        let fetched_at = std::fs::metadata(&self.cache_path)
            .and_then(|m| m.modified())
            .ok();
        Some(Snapshot {
            manifest: Some(manifest.into()),
            fetched_at,
            offline: false,
            error: None,
        })
    }

    fn save_cache(&self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(dir) = self.cache_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.cache_path.with_extension("json.tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &self.cache_path)
    }
}

async fn fetch_one(net: &Net, url: &str) -> AppResult<(Manifest, Vec<u8>)> {
    let mut response = net.get(url, Purpose::Manifest, None).await?;
    if !response.status().is_success() {
        return Err(AppError::new(
            AppErrorCode::DownloadFailed,
            format!("{}: HTTP {}", response.host(), response.status()),
        ));
    }
    let bytes = response.bytes_limited(MAX_MANIFEST_BYTES).await?;
    Ok((parse(&bytes)?, bytes))
}

/// A pack as installed in `content.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPack {
    /// Pack id.
    pub id: String,
    /// Pack type.
    pub kind: String,
    /// Installed version.
    pub version: String,
    /// Whether it ships with the app.
    pub bundled: bool,
    /// English title (`packs.title`).
    pub title: String,
    /// Titles from `meta_json`.
    pub titles: LocalizedTitle,
    /// Language.
    pub lang: Option<String>,
    /// Author.
    pub author: Option<String>,
    /// License name.
    pub license: String,
    /// Attribution.
    pub attribution: Option<String>,
    /// `citation_json`.
    pub citation_json: Option<String>,
    /// `meta_json`.
    pub meta_json: Option<String>,
}

/// Reads every installed pack.
pub fn installed_packs(conn: &rusqlite::Connection) -> AppResult<Vec<InstalledPack>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, version, bundled, title, lang, author, license, attribution,
                citation_json, meta_json FROM packs ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        let meta_json: Option<String> = row.get(10)?;
        let meta = parse_object(meta_json.as_deref());
        let title = |lang: &str| meta["title"][lang].as_str().map(str::to_owned);
        Ok(InstalledPack {
            id: row.get(0)?,
            kind: row.get(1)?,
            version: row.get(2)?,
            bundled: row.get(3)?,
            title: row.get(4)?,
            titles: LocalizedTitle {
                tr: title("tr"),
                en: title("en"),
            },
            lang: row.get(5)?,
            author: row.get(6)?,
            license: row.get(7)?,
            attribution: row.get(8)?,
            citation_json: row.get(9)?,
            meta_json,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn parse_object(json: Option<&str>) -> serde_json::Value {
    json.and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// The string fields of a citation object, for display.
pub fn citation_fields(citation: &serde_json::Value) -> BTreeMap<String, String> {
    citation
        .as_object()
        .map(|fields| {
            fields
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

/// What the app is doing with a pack right now.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum JobView {
    /// Waiting for an earlier job.
    Queued,
    /// Downloading, verifying or installing.
    Running {
        /// Current phase.
        phase: Phase,
        /// Bytes done in this phase.
        done: f64,
        /// Bytes in this phase.
        total: f64,
    },
}

/// Phases of a pack job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Downloading (or copying from a local source).
    Download,
    /// Checking the sha256.
    Verify,
    /// Decompressing, checking and copying rows.
    Install,
}

/// One row of the Packs screen.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct PackView {
    /// Pack id.
    pub id: String,
    /// Pack type (`translation`, `tafsir`, `morphology`, …).
    pub kind: String,
    /// Language of the content.
    pub lang: Option<String>,
    /// Title in the interface languages.
    pub title: LocalizedTitle,
    /// English title, the fallback.
    pub title_fallback: String,
    /// Description in the interface languages.
    pub description: Option<LocalizedText>,
    /// Author.
    pub author: Option<String>,
    /// Version in the manifest, or the installed one if the pack is not in it.
    pub version: String,
    /// Download size in bytes.
    pub size: f64,
    /// Installed size in bytes.
    pub installed_size: f64,
    /// License name.
    pub license: String,
    /// Terms shown before download.
    pub license_terms: LocalizedList,
    /// Attribution.
    pub attribution: Option<String>,
    /// Citation fields.
    pub citation: BTreeMap<String, String>,
    /// Link to the source (`citation.url`).
    pub source_url: Option<String>,
    /// Oldest app version that can use the pack.
    pub min_app_version: String,
    /// Whether the manifest lists the pack.
    pub in_manifest: bool,
    /// Whether this app can install packs of this type.
    pub supported: bool,
    /// Whether the pack is installed.
    pub installed: bool,
    /// The installed version.
    pub installed_version: Option<String>,
    /// Whether the manifest has a newer version than the installed one.
    pub update_available: bool,
    /// Whether the pack needs a newer app.
    pub incompatible: bool,
    /// Whether the pack ships with the app (cannot be installed or removed).
    pub bundled: bool,
    /// Whether the terms must be confirmed before download.
    pub terms_ack_required: bool,
    /// Whether the user confirmed the terms of this version.
    pub terms_acknowledged: bool,
    /// The installed morphology pack this one would replace.
    pub replaces: Option<String>,
    /// The running or queued job for this pack.
    pub job: Option<JobView>,
}

/// One bundle of the Packs screen and of the first-launch card.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct BundleView {
    /// Bundle id.
    pub id: String,
    /// Title.
    pub title: LocalizedText,
    /// Description.
    pub description: Option<LocalizedText>,
    /// Member pack ids.
    pub packs: Vec<String>,
    /// Members that are not installed yet (and can be).
    pub missing: Vec<String>,
    /// Download size of the missing members.
    pub size: f64,
    /// Installed size of the missing members.
    pub installed_size: f64,
}

/// The Packs screen.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Catalog {
    /// `remote` or `local`.
    pub source: String,
    /// When the manifest was fetched (milliseconds since the epoch).
    pub fetched_at: Option<f64>,
    /// True if the latest fetch failed and a cached manifest is shown.
    pub offline: bool,
    /// Why the latest fetch failed.
    pub error: Option<AppError>,
    /// Packs: manifest order, then installed packs missing from the manifest.
    pub packs: Vec<PackView>,
    /// Bundles.
    pub bundles: Vec<BundleView>,
}

/// Everything the catalog is computed from.
pub struct CatalogInput<'a> {
    /// The pack source.
    pub source: &'a PackSource,
    /// The manifest snapshot.
    pub snapshot: &'a Snapshot,
    /// Installed packs.
    pub installed: &'a [InstalledPack],
    /// Acknowledged `(pack id, version)` pairs.
    pub acks: &'a [(String, String)],
    /// Jobs by pack id.
    pub jobs: &'a HashMap<String, JobView>,
    /// This app's version.
    pub app_version: &'a semver::Version,
}

/// Combines the manifest with what is installed.
pub fn catalog(input: &CatalogInput<'_>) -> Catalog {
    let installed: HashMap<&str, &InstalledPack> =
        input.installed.iter().map(|p| (p.id.as_str(), p)).collect();
    let acks: HashSet<(&str, &str)> = input
        .acks
        .iter()
        .map(|(id, v)| (id.as_str(), v.as_str()))
        .collect();
    let morphology = input
        .installed
        .iter()
        .find(|p| p.kind == PackKind::Morphology.as_str())
        .map(|p| p.id.clone());

    let mut packs = Vec::new();
    let empty = Manifest {
        packs: Vec::new(),
        bundles: Vec::new(),
    };
    let manifest = input.snapshot.manifest.as_deref().unwrap_or(&empty);
    for entry in &manifest.packs {
        let local = installed.get(entry.id.as_str());
        let newer = local.is_some_and(|l| is_newer(&entry.version, &l.version));
        let supported = entry.pack_kind().is_some();
        packs.push(PackView {
            id: entry.id.clone(),
            kind: entry.kind.clone(),
            lang: entry.lang.clone(),
            title: LocalizedTitle {
                tr: Some(entry.title.tr.clone()),
                en: Some(entry.title.en.clone()),
            },
            title_fallback: entry.title.en.clone(),
            description: entry.description.clone(),
            author: entry.author.clone(),
            version: entry.version.clone(),
            size: entry.size as f64,
            installed_size: entry.installed_size as f64,
            license: entry.license.clone(),
            license_terms: entry.license_terms.clone().unwrap_or_default(),
            attribution: entry.attribution.clone(),
            citation: entry
                .citation
                .as_ref()
                .map(citation_fields)
                .unwrap_or_default(),
            source_url: entry
                .citation
                .as_ref()
                .and_then(|c| c["url"].as_str())
                .map(str::to_owned),
            min_app_version: entry.min_app_version.clone(),
            in_manifest: true,
            supported,
            installed: local.is_some(),
            installed_version: local.map(|l| l.version.clone()),
            update_available: newer && !local.is_some_and(|l| l.bundled),
            incompatible: is_incompatible(entry, input.app_version),
            bundled: local.is_some_and(|l| l.bundled),
            terms_ack_required: entry.terms_ack,
            terms_acknowledged: acks.contains(&(entry.id.as_str(), entry.version.as_str())),
            replaces: (entry.pack_kind() == Some(PackKind::Morphology))
                .then(|| morphology.clone())
                .flatten()
                .filter(|m| *m != entry.id),
            job: input.jobs.get(&entry.id).cloned(),
        });
    }
    for local in input.installed {
        if manifest.pack(&local.id).is_some() {
            continue;
        }
        let meta = parse_object(local.meta_json.as_deref());
        let citation = parse_object(local.citation_json.as_deref());
        let terms: LocalizedList =
            serde_json::from_value(meta["license_terms"].clone()).unwrap_or_default();
        let description: Option<LocalizedText> =
            serde_json::from_value(meta["description"].clone()).ok();
        packs.push(PackView {
            id: local.id.clone(),
            kind: local.kind.clone(),
            lang: local.lang.clone(),
            title: local.titles.clone(),
            title_fallback: local.title.clone(),
            description,
            author: local.author.clone(),
            version: local.version.clone(),
            size: 0.0,
            installed_size: 0.0,
            license: local.license.clone(),
            license_terms: terms,
            attribution: local.attribution.clone(),
            citation: citation_fields(&citation),
            source_url: citation["url"].as_str().map(str::to_owned),
            min_app_version: default_min_app_version(),
            in_manifest: false,
            supported: PackKind::parse(&local.kind).is_some(),
            installed: true,
            installed_version: Some(local.version.clone()),
            update_available: false,
            incompatible: false,
            bundled: local.bundled,
            terms_ack_required: false,
            terms_acknowledged: false,
            replaces: None,
            job: input.jobs.get(&local.id).cloned(),
        });
    }

    let bundles = manifest
        .bundles
        .iter()
        .map(|bundle| {
            let missing: Vec<&PackEntry> = bundle
                .packs
                .iter()
                .filter_map(|id| manifest.pack(id))
                .filter(|p| {
                    p.pack_kind().is_some()
                        && !is_incompatible(p, input.app_version)
                        && installed
                            .get(p.id.as_str())
                            .is_none_or(|l| !l.bundled && is_newer(&p.version, &l.version))
                })
                .collect();
            BundleView {
                id: bundle.id.clone(),
                title: bundle.title.clone(),
                description: bundle.description.clone(),
                packs: bundle.packs.clone(),
                size: missing.iter().map(|p| p.size as f64).sum(),
                installed_size: missing.iter().map(|p| p.installed_size as f64).sum(),
                missing: missing.iter().map(|p| p.id.clone()).collect(),
            }
        })
        .collect();

    Catalog {
        source: input.source.label().to_owned(),
        fetched_at: input.snapshot.fetched_at.and_then(|at| {
            at.duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_millis() as f64)
        }),
        offline: input.snapshot.offline,
        error: input.snapshot.error.clone(),
        packs,
        bundles,
    }
}

/// Whether `candidate` is a higher SemVer version than `installed`.
pub fn is_newer(candidate: &str, installed: &str) -> bool {
    match (
        semver::Version::parse(candidate),
        semver::Version::parse(installed),
    ) {
        (Ok(c), Ok(i)) => c > i,
        _ => candidate != installed,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::net::test_support::{local_net, serve};

    /// A manifest entry for tests.
    pub(crate) fn entry(id: &str, kind: &str, version: &str) -> serde_json::Value {
        let ext = if kind == "morphology" || kind == "model" {
            "tar"
        } else {
            "sqlite"
        };
        serde_json::json!({
            "id": id, "type": kind, "lang": "ar",
            "title": { "tr": format!("{id} tr"), "en": format!("{id} en") },
            "author": "Author", "version": version, "min_app_version": "0.1.0",
            "size": 100, "installed_size": 1000, "sha256": "a".repeat(64),
            "url": format!("https://github.com/siyak-app/siyak-data/releases/download/t/{id}.{ext}.zst"),
            "license": "Public domain",
            "license_terms": { "tr": ["koşul"], "en": ["term"] },
            "attribution": "Source", "citation": { "type": "tafsir", "url": "https://example.org", "year": 1 }
        })
    }

    pub(crate) fn manifest_json(packs: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1, "generated_at": "2026-10-21T00:00:00Z",
            "packs": packs,
            "bundles": [{ "id": "recommended", "title": { "tr": "Önerilen", "en": "Recommended" },
                          "packs": ["morphology.qac", "ar.jalalayn", "missing.pack"] }]
        })
    }

    fn sample() -> serde_json::Value {
        manifest_json(vec![
            entry("ar.jalalayn", "tafsir", "1.0.0"),
            entry("morphology.qac", "morphology", "1.0.0"),
            entry("en.pickthall", "translation", "1.0.0"),
        ])
    }

    fn bytes(value: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    #[test]
    fn parses_a_valid_manifest_and_drops_unknown_bundle_members() {
        let manifest = parse(&bytes(&sample())).unwrap();
        assert_eq!(manifest.packs.len(), 3);
        assert_eq!(manifest.bundles[0].packs, ["morphology.qac", "ar.jalalayn"]);
        let qac = manifest.pack("morphology.qac").unwrap();
        assert_eq!(qac.pack_kind(), Some(PackKind::Morphology));
        assert_eq!(qac.file_name(), "morphology.qac.tar.zst");
        assert!(!qac.terms_ack);
    }

    #[test]
    fn parses_the_real_manifest_when_present() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../siyak-data/dist/manifest.json");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let manifest = parse(&bytes).unwrap();
        assert_eq!(manifest.packs.len(), 18);
        assert!(manifest.bundle("recommended").is_some());
    }

    #[test]
    fn ignores_unknown_fields_and_keeps_unknown_types() {
        let mut value = sample();
        value["future"] = serde_json::json!({ "x": 1 });
        value["packs"][0]["future_field"] = serde_json::json!([1, 2]);
        let mut future = entry("audio.x", "audio", "1.0.0");
        future["url"] = "https://github.com/x/audio.x.mp3".into();
        future["terms_ack"] = true.into();
        value["packs"].as_array_mut().unwrap().push(future);
        let manifest = parse(&bytes(&value)).unwrap();
        let future = manifest.pack("audio.x").unwrap();
        assert_eq!(future.pack_kind(), None);
        assert!(future.terms_ack);
    }

    #[test]
    fn vector_and_model_packs_need_valid_embedding_metadata() {
        let mut vectors = entry("vectors.e5", "vectors", "1.0.0");
        vectors["requires"] = serde_json::json!(["model.e5"]);
        vectors["meta"] = serde_json::json!({
            "embedding_model": "local/e5-small", "dim": 384,
            "backend": "local", "model_pack": "model.e5"
        });
        let mut model = entry("model.e5", "model", "1.0.0");
        model["meta"] = serde_json::json!({ "embedding_model": "local/e5-small", "dim": 384 });
        let mut api = entry("vectors.voyage", "vectors", "1.0.0");
        api["meta"] = serde_json::json!({
            "embedding_model": "voyage/voyage-4", "dim": 1024,
            "backend": "api", "provider": "voyage", "api_model": "voyage-4"
        });
        // Each broken one stays in the manifest, listed as unsupported.
        let mut no_requires = vectors.clone();
        no_requires["id"] = "vectors.a".into();
        no_requires["url"] = "https://github.com/x/vectors.a.sqlite.zst".into();
        no_requires["requires"] = serde_json::json!([]);
        let mut no_meta = entry("vectors.b", "vectors", "1.0.0");
        no_meta["requires"] = serde_json::json!(["model.e5"]);
        let mut bad_model = entry("model.c", "model", "1.0.0");
        bad_model["meta"] = serde_json::json!({ "embedding_model": "openai/x", "dim": 3 });

        let manifest = parse(&bytes(&manifest_json(vec![
            vectors,
            model,
            api,
            no_requires,
            no_meta,
            bad_model,
        ])))
        .unwrap();
        let kind = |id: &str| manifest.pack(id).unwrap().pack_kind();
        assert_eq!(kind("vectors.e5"), Some(PackKind::Vectors));
        assert_eq!(kind("model.e5"), Some(PackKind::Model));
        assert_eq!(kind("vectors.voyage"), Some(PackKind::Vectors));
        for id in ["vectors.a", "vectors.b", "model.c"] {
            assert_eq!(kind(id), None, "{id}");
        }
        assert_eq!(PackKind::Model.file_name("model.e5"), "model.e5.tar.zst");
        assert_eq!(
            PackKind::Vectors.file_name("vectors.e5"),
            "vectors.e5.sqlite.zst"
        );
    }

    #[test]
    fn rejects_an_unknown_schema_version() {
        let mut value = sample();
        value["schema_version"] = 2.into();
        assert_eq!(
            parse(&bytes(&value)).unwrap_err().code,
            AppErrorCode::ManifestUnsupported
        );
        value["schema_version"] = "1".into();
        assert_eq!(
            parse(&bytes(&value)).unwrap_err().code,
            AppErrorCode::ManifestInvalid
        );
    }

    #[test]
    fn rejects_oversized_documents() {
        let mut big = bytes(&sample());
        big.resize(MAX_MANIFEST_BYTES + 1, b' ');
        assert_eq!(parse(&big).unwrap_err().code, AppErrorCode::ManifestInvalid);
    }

    #[test]
    fn rejects_invalid_entries() {
        let cases: Vec<(&str, serde_json::Value)> = vec![
            ("id", "../evil".into()),
            ("id", "Ar.Jalalayn".into()),
            ("id", "jalalayn".into()),
            ("sha256", "abc".into()),
            ("sha256", "A".repeat(64).into()),
            ("size", 0.into()),
            ("installed_size", (MAX_PACK_BYTES + 1).into()),
            ("url", "http://github.com/ar.jalalayn.sqlite.zst".into()),
            ("url", "https://github.com/other.sqlite.zst".into()),
            ("version", "one".into()),
            ("title", "plain string".into()),
        ];
        for (field, value) in cases {
            let mut manifest = sample();
            manifest["packs"][0][field] = value.clone();
            assert_eq!(
                parse(&bytes(&manifest)).unwrap_err().code,
                AppErrorCode::ManifestInvalid,
                "{field} = {value}"
            );
        }
        let mut duplicate = sample();
        let first = duplicate["packs"][0].clone();
        duplicate["packs"].as_array_mut().unwrap().push(first);
        assert!(parse(&bytes(&duplicate)).is_err());
        assert!(parse(b"not json").is_err());
    }

    fn installed(id: &str, kind: &str, version: &str, bundled: bool) -> InstalledPack {
        InstalledPack {
            id: id.into(),
            kind: kind.into(),
            version: version.into(),
            bundled,
            title: id.into(),
            titles: LocalizedTitle::default(),
            lang: None,
            author: None,
            license: "PD".into(),
            attribution: None,
            citation_json: None,
            meta_json: Some(r#"{"license_terms": {"en": ["x"], "tr": ["y"]}}"#.into()),
        }
    }

    #[test]
    fn catalog_computes_states() {
        let mut value = sample();
        value["packs"][0]["version"] = "1.1.0".into();
        value["packs"][1]["terms_ack"] = true.into();
        value["packs"][2]["min_app_version"] = "99.0.0".into();
        let manifest = parse(&bytes(&value)).unwrap();
        let snapshot = Snapshot {
            manifest: Some(manifest.into()),
            fetched_at: Some(UNIX_EPOCH + Duration::from_secs(1)),
            offline: false,
            error: None,
        };
        let installed = vec![
            installed("ar.jalalayn", "tafsir", "1.0.0", false),
            installed("morphology.other", "morphology", "1.0.0", false),
            installed("en.pickthall", "translation", "1.0.0", true),
        ];
        let acks = vec![("morphology.qac".to_owned(), "1.0.0".to_owned())];
        let mut jobs = HashMap::new();
        jobs.insert("ar.jalalayn".to_owned(), JobView::Queued);
        let catalog = catalog(&CatalogInput {
            source: &PackSource::Local("/x".into()),
            snapshot: &snapshot,
            installed: &installed,
            acks: &acks,
            jobs: &jobs,
            app_version: &semver::Version::new(0, 1, 0),
        });
        assert_eq!(catalog.source, "local");
        assert_eq!(catalog.fetched_at, Some(1000.0));
        let pack = |id: &str| catalog.packs.iter().find(|p| p.id == id).unwrap();

        let jalalayn = pack("ar.jalalayn");
        assert!(jalalayn.installed && jalalayn.update_available);
        assert_eq!(jalalayn.installed_version.as_deref(), Some("1.0.0"));
        assert_eq!(jalalayn.job, Some(JobView::Queued));
        assert_eq!(jalalayn.source_url.as_deref(), Some("https://example.org"));
        assert_eq!(jalalayn.citation.get("year"), None, "only string fields");

        let qac = pack("morphology.qac");
        assert!(qac.terms_ack_required && qac.terms_acknowledged && !qac.installed);
        assert_eq!(qac.replaces.as_deref(), Some("morphology.other"));

        let pickthall = pack("en.pickthall");
        assert!(pickthall.bundled && pickthall.incompatible && !pickthall.update_available);

        let other = pack("morphology.other");
        assert!(!other.in_manifest && other.installed);
        assert_eq!(other.license_terms.en, ["x"]);

        let bundle = &catalog.bundles[0];
        assert_eq!(bundle.missing, ["morphology.qac", "ar.jalalayn"]);
        assert_eq!(bundle.size, 200.0);
    }

    #[test]
    fn local_source_resolves_files_by_name() {
        let manifest = parse(&bytes(&sample())).unwrap();
        let source = PackSource::Local("/data/dist".into());
        assert_eq!(
            source.local_file(manifest.pack("morphology.qac").unwrap()),
            Some(PathBuf::from("/data/dist/morphology.qac.tar.zst"))
        );
        assert_eq!(PackSource::Remote.local_file(&manifest.packs[0]), None);
    }

    #[tokio::test]
    async fn local_store_reads_manifest_from_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.json"), bytes(&sample())).unwrap();
        let data = tempfile::tempdir().unwrap();
        let store = ManifestStore::new(PackSource::Local(dir.path().into()), data.path());
        let (net, log) = local_net();
        let snapshot = store.load(&net, false).await;
        assert_eq!(snapshot.manifest.unwrap().packs.len(), 3);
        assert!(
            log.0.lock().unwrap().is_empty(),
            "no network for a local source"
        );
    }

    #[tokio::test]
    async fn remote_store_falls_back_caches_and_works_offline() {
        use axum::Router;
        use axum::http::StatusCode;
        use axum::routing::get;
        let body = bytes(&sample());
        let router = Router::new()
            .route(
                "/primary",
                get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
            )
            .route("/fallback", get(move || async move { body.clone() }));
        let base = serve(router).await;
        let data = tempfile::tempdir().unwrap();
        let urls = vec![format!("{base}/primary"), format!("{base}/fallback")];
        let store = ManifestStore::with_urls(PackSource::Remote, data.path(), urls);
        let (net, log) = local_net();

        let first = store.load(&net, false).await;
        assert!(first.error.is_none() && !first.offline);
        assert_eq!(log.0.lock().unwrap().len(), 2);
        // Within the hour the manifest is not fetched again…
        store.load(&net, false).await;
        assert_eq!(log.0.lock().unwrap().len(), 2);
        // …unless forced.
        store.load(&net, true).await;
        assert_eq!(log.0.lock().unwrap().len(), 4);

        // Next launch: a fresh cache file counts as fetched within the hour.
        let offline = ManifestStore::with_urls(
            PackSource::Remote,
            data.path(),
            vec!["http://127.0.0.1:9/manifest.json".into()],
        );
        assert!(!offline.load(&net, false).await.offline);
        // Offline refresh: the cached copy is used, and the snapshot says so.
        let snapshot = offline.load(&net, true).await;
        assert!(snapshot.offline);
        assert_eq!(snapshot.error.unwrap().code, AppErrorCode::NetworkFailed);
        assert_eq!(snapshot.manifest.unwrap().packs.len(), 3);
    }

    #[tokio::test]
    async fn remote_store_without_cache_reports_the_error() {
        let data = tempfile::tempdir().unwrap();
        let store = ManifestStore::with_urls(
            PackSource::Remote,
            data.path(),
            vec!["http://127.0.0.1:9/manifest.json".into()],
        );
        let (net, _log) = local_net();
        let snapshot = store.load(&net, false).await;
        assert!(snapshot.manifest.is_none() && snapshot.offline);
    }
}
