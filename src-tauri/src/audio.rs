//! Recitation audio (ADR 0014): the layout of audio packs and the `siyak-audio:`
//! scheme that plays their files in the webview.
//!
//! An audio pack is an archive pack (pack-format §4.10) whose files are one mp3 per
//! verse, `audio/SSSAAA.mp3`, kept in `<app_data>/packs/<pack id>/`. The webview has
//! no file system access (spec §14.3), so it plays a verse from
//! `siyak-audio://localhost/<pack id>/<verse id>.mp3` (`http://siyak-audio.localhost/…`
//! on Windows). The handler serves only the verse files of installed audio packs:
//! the path on disk is built here from a checked pack id and verse number, never
//! taken from the request.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use specta::Type;
use tauri::http::{Response, StatusCode, header};

use crate::db::content::{LocalizedTitle, localized_title};
use crate::error::AppResult;

/// The URI scheme registered in `lib.rs` and allowed as `media-src` by the CSP.
pub const SCHEME: &str = "siyak-audio";

/// Verses per surah (Hafs, Kufan count); the order of the global verse ids.
pub const AYAH_COUNTS: [u16; 114] = [
    7, 286, 200, 176, 120, 165, 206, 75, 129, 109, 123, 111, 43, 52, 99, 128, 111, 110, 98, 135,
    112, 78, 118, 64, 77, 227, 93, 88, 69, 60, 34, 30, 73, 54, 45, 83, 182, 88, 75, 85, 54, 53, 89,
    59, 37, 35, 38, 29, 18, 45, 60, 49, 62, 55, 78, 96, 29, 22, 24, 13, 14, 11, 11, 18, 12, 12, 30,
    52, 52, 44, 28, 28, 20, 56, 40, 31, 50, 40, 46, 42, 29, 19, 36, 25, 22, 17, 19, 26, 30, 20, 15,
    21, 11, 8, 8, 19, 5, 8, 8, 11, 11, 8, 3, 9, 5, 4, 7, 3, 6, 3, 5, 4, 5, 6,
];

/// Number of verses, the highest global verse id.
pub const VERSE_COUNT: u32 = 6236;

/// The database member of an audio pack (`pack_info` only).
pub const DB_MEMBER: &str = "audio.sqlite";

/// The license and attribution of the recording, shown on About → Sources.
pub const LICENSE_MEMBER: &str = "audio/LICENSE";

/// `(surah, ayah)` of a global verse id.
pub fn surah_ayah(verse_id: u32) -> Option<(u32, u32)> {
    let mut first = 1u32;
    for (surah, count) in (1u32..).zip(AYAH_COUNTS) {
        let count = u32::from(count);
        if verse_id < first + count {
            return (verse_id >= first).then(|| (surah, verse_id - first + 1));
        }
        first += count;
    }
    None
}

/// The archive member (and the path below the pack's directory) of one verse.
pub fn verse_member(surah: u32, ayah: u32) -> String {
    format!("audio/{surah:03}{ayah:03}.mp3")
}

static MEMBER_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut names = vec![DB_MEMBER.to_owned(), LICENSE_MEMBER.to_owned()];
    for (surah, count) in (1u32..).zip(AYAH_COUNTS) {
        names.extend((1..=u32::from(count)).map(|ayah| verse_member(surah, ayah)));
    }
    names
});

static MEMBERS: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| MEMBER_NAMES.iter().map(String::as_str).collect());

/// Every member an audio pack must hold: its database, LICENSE and one mp3 per verse.
pub fn archive_members() -> &'static [&'static str] {
    MEMBERS.as_slice()
}

/// The reciter of an audio pack, from the manifest's `meta` (pack-format §4.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reciter {
    /// Name in Turkish.
    pub name_tr: String,
    /// Name in English.
    pub name_en: String,
    /// `murattal`, `mujawwad` or `muallim`.
    pub style: String,
}

impl Reciter {
    /// Reads and checks `meta.reciter` and `meta.style`.
    pub fn from_meta(meta: &Map<String, Value>) -> Result<Self, String> {
        let reciter = meta
            .get("reciter")
            .and_then(Value::as_object)
            .ok_or("meta.reciter must be an object")?;
        let name = |lang: &str| {
            reciter
                .get(lang)
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .ok_or(format!("meta.reciter.{lang} must be a non-empty string"))
        };
        let style = meta
            .get("style")
            .and_then(Value::as_str)
            .filter(|s| matches!(*s, "murattal" | "mujawwad" | "muallim"))
            .ok_or("meta.style must be murattal, mujawwad or muallim")?;
        Ok(Self {
            name_tr: name("tr")?,
            name_en: name("en")?,
            style: style.to_owned(),
        })
    }
}

/// An installed audio pack, as the reader's player lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AudioPack {
    /// Pack id, e.g. `audio.alafasy`.
    pub id: String,
    /// Title as published (`packs.title`).
    pub title: String,
    /// Title in the interface languages.
    pub titles: LocalizedTitle,
    /// `murattal`, `mujawwad` or `muallim`, if the pack says.
    pub style: Option<String>,
    /// Attribution shown with the player (spec §6.5).
    pub attribution: Option<String>,
}

/// Lists the installed audio packs by title.
pub fn list_audio_packs(conn: &Connection) -> AppResult<Vec<AudioPack>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, meta_json, attribution FROM packs WHERE type = 'audio' ORDER BY title, id",
    )?;
    let rows = stmt.query_map([], |row| {
        let meta: Option<String> = row.get(2)?;
        let style = meta
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
            .and_then(|value| value["style"].as_str().map(str::to_owned));
        Ok(AudioPack {
            id: row.get(0)?,
            title: row.get(1)?,
            titles: localized_title(meta.as_deref()),
            style,
            attribution: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A verse of an audio pack, parsed from a `siyak-audio:` request path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioRequest {
    /// The audio pack's id.
    pub pack_id: String,
    /// The global verse id.
    pub verse_id: u32,
}

impl AudioRequest {
    /// Parses `/<pack id>/<verse id>.mp3`, percent-encoded or not (`convertFileSrc`
    /// encodes the slash). Pack ids are `audio.` plus lowercase letters, digits and `-`.
    pub fn parse(path: &str) -> Option<Self> {
        let decoded = percent_decode(path)?;
        let rest = decoded.strip_prefix('/').unwrap_or(&decoded);
        let (pack_id, file) = rest.split_once('/')?;
        let name = pack_id.strip_prefix("audio.")?;
        let valid_id = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        let verse = file.strip_suffix(".mp3")?;
        if !valid_id || verse.is_empty() || !verse.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let verse_id: u32 = verse.parse().ok()?;
        if !(1..=VERSE_COUNT).contains(&verse_id) {
            return None;
        }
        Some(Self {
            pack_id: pack_id.to_owned(),
            verse_id,
        })
    }

    /// The file of this verse below `packs_dir`.
    pub fn file(&self, packs_dir: &Path) -> Option<PathBuf> {
        let (surah, ayah) = surah_ayah(self.verse_id)?;
        Some(
            packs_dir
                .join(&self.pack_id)
                .join(verse_member(surah, ayah)),
        )
    }
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// A byte range of a `Range: bytes=…` header, resolved against the file length.
fn byte_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    // Only a single range; browsers ask for one when seeking media.
    if spec.contains(',') || len == 0 {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    let (start, end) = match (start.trim(), end.trim()) {
        ("", suffix) => {
            let n: u64 = suffix.parse().ok()?;
            (len.saturating_sub(n), len - 1)
        }
        (start, "") => (start.parse().ok()?, len - 1),
        (start, end) => (start.parse().ok()?, end.parse::<u64>().ok()?.min(len - 1)),
    };
    (start <= end && start < len).then_some((start, end))
}

fn status(code: StatusCode) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(code)
        .body(Cow::Borrowed(&[][..]))
        .unwrap_or_default()
}

/// Answers one `siyak-audio:` request.
///
/// `is_audio_pack` says whether a pack id is an installed pack of type `audio`.
pub fn respond(
    packs_dir: &Path,
    is_audio_pack: impl Fn(&str) -> bool,
    path: &str,
    range: Option<&str>,
) -> Response<Cow<'static, [u8]>> {
    let Some(request) = AudioRequest::parse(path) else {
        return status(StatusCode::BAD_REQUEST);
    };
    if !is_audio_pack(&request.pack_id) {
        return status(StatusCode::NOT_FOUND);
    }
    let Some(data) = request.file(packs_dir).and_then(|f| fs::read(f).ok()) else {
        return status(StatusCode::NOT_FOUND);
    };
    let len = data.len() as u64;
    let builder = Response::builder()
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .header(header::ACCEPT_RANGES, "bytes")
        // A pack's verse file never changes while the pack is installed.
        .header(header::CACHE_CONTROL, "no-cache");
    let response = match range {
        None => builder
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, len)
            .body(Cow::Owned(data)),
        Some(value) => match byte_range(value, len) {
            Some((start, end)) => {
                let (from, to) = (start as usize, end as usize + 1);
                builder
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{len}"))
                    .header(header::CONTENT_LENGTH, end - start + 1)
                    .body(Cow::Owned(data[from..to].to_vec()))
            }
            None => builder
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{len}"))
                .body(Cow::Borrowed(&[][..])),
        },
    };
    response.unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_every_verse() {
        let total: u32 = AYAH_COUNTS.iter().map(|&c| u32::from(c)).sum();
        assert_eq!(total, VERSE_COUNT);
        assert_eq!(archive_members().len(), VERSE_COUNT as usize + 2);
    }

    #[test]
    fn maps_verse_ids() {
        assert_eq!(surah_ayah(1), Some((1, 1)));
        assert_eq!(surah_ayah(8), Some((2, 1)));
        assert_eq!(surah_ayah(262), Some((2, 255)));
        assert_eq!(surah_ayah(6236), Some((114, 6)));
        assert_eq!(surah_ayah(0), None);
        assert_eq!(surah_ayah(6237), None);
        assert_eq!(verse_member(2, 255), "audio/002255.mp3");
    }

    #[test]
    fn parses_requests() {
        let want = AudioRequest {
            pack_id: "audio.alafasy".into(),
            verse_id: 262,
        };
        assert_eq!(
            AudioRequest::parse("/audio.alafasy/262.mp3"),
            Some(want.clone())
        );
        assert_eq!(AudioRequest::parse("/audio.alafasy%2F262.mp3"), Some(want));
        for bad in [
            "/audio.alafasy/0.mp3",
            "/audio.alafasy/6237.mp3",
            "/audio.alafasy/1.wav",
            "/audio.alafasy/../1.mp3",
            "/audio.../1.mp3",
            "/en.pickthall/1.mp3",
            "/audio.Alafasy/1.mp3",
            "/audio.alafasy/1/2.mp3",
            "/audio.alafasy%2F..%2F1.mp3",
            "/audio.alafasy/+1.mp3",
            "/audio.alafasy/%zz.mp3",
        ] {
            assert_eq!(AudioRequest::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn resolves_ranges() {
        assert_eq!(byte_range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(byte_range("bytes=2-4", 10), Some((2, 4)));
        assert_eq!(byte_range("bytes=2-40", 10), Some((2, 9)));
        assert_eq!(byte_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(byte_range("bytes=10-", 10), None);
        assert_eq!(byte_range("bytes=4-2", 10), None);
        assert_eq!(byte_range("bytes=0-1,3-4", 10), None);
        assert_eq!(byte_range("items=0-1", 10), None);
    }

    #[test]
    fn serves_installed_verse_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("audio.test").join("audio/002255.mp3");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"0123456789").unwrap();
        let installed = |id: &str| id == "audio.test";

        let ok = respond(dir.path(), installed, "/audio.test/262.mp3", None);
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[header::CONTENT_TYPE], "audio/mpeg");
        assert_eq!(ok.body().as_ref(), b"0123456789");

        let part = respond(
            dir.path(),
            installed,
            "/audio.test/262.mp3",
            Some("bytes=2-4"),
        );
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.headers()[header::CONTENT_RANGE], "bytes 2-4/10");
        assert_eq!(part.body().as_ref(), b"234");

        let past = respond(
            dir.path(),
            installed,
            "/audio.test/262.mp3",
            Some("bytes=20-"),
        );
        assert_eq!(past.status(), StatusCode::RANGE_NOT_SATISFIABLE);

        let other = respond(dir.path(), installed, "/audio.other/262.mp3", None);
        assert_eq!(other.status(), StatusCode::NOT_FOUND);
        let missing = respond(dir.path(), installed, "/audio.test/1.mp3", None);
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let bad = respond(dir.path(), installed, "/../secret", None);
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn lists_installed_audio_packs() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn, crate::db::content::MIGRATIONS).unwrap();
        conn.execute_batch(
            r#"INSERT INTO packs (id, type, title, version, license, attribution, meta_json, installed_at)
               VALUES ('audio.b', 'audio', 'B', '1.0.0', 'x', 'Source: Quran.com',
                       '{"title": {"tr": "B tr", "en": "B en"}, "style": "mujawwad"}', 'now'),
                      ('audio.a', 'audio', 'A', '1.0.0', 'x', NULL, NULL, 'now'),
                      ('model.m', 'model', 'M', '1.0.0', 'x', NULL, NULL, 'now');"#,
        )
        .unwrap();
        let packs = list_audio_packs(&conn).unwrap();
        let ids: Vec<&str> = packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["audio.a", "audio.b"]);
        assert_eq!(packs[1].titles.tr.as_deref(), Some("B tr"));
        assert_eq!(packs[1].style.as_deref(), Some("mujawwad"));
        assert_eq!(packs[0].style, None);
    }

    #[test]
    fn reads_the_reciter() {
        let meta = serde_json::json!({
            "reciter": { "tr": "Mişari Raşid el-Afasi", "en": "Mishari Rashid al-Afasy" },
            "style": "murattal",
        });
        let reciter = Reciter::from_meta(meta.as_object().unwrap()).unwrap();
        assert_eq!(reciter.name_en, "Mishari Rashid al-Afasy");
        let bad = serde_json::json!({ "reciter": { "tr": "x" }, "style": "murattal" });
        assert!(Reciter::from_meta(bad.as_object().unwrap()).is_err());
        let style = serde_json::json!({ "reciter": { "tr": "x", "en": "y" }, "style": "fast" });
        assert!(Reciter::from_meta(style.as_object().unwrap()).is_err());
    }
}
