# 0014. Ship verse-by-verse recitations as audio packs from Quran.com

- Status: Accepted
- Date: 2026-11-07

## Context

Readers asked to hear each verse while they read. Spec v1 listed audio as out of scope, and
principle 4 (spec §1) says content whose redistribution rights are not confirmed is not
distributed. The usual source of verse-by-verse recitations is Quran.com (Quran.Foundation),
which serves the recordings of EveryAyah and QuranicAudio from its CDN. A Kaggle copy of those
files exists (`abdo3id/quran-com-audio-files`, 12 recitations, about 21 GB), but it is a
re-upload under a license the uploader chose, and the files are the same as Quran.com's. Neither
Quran.com nor the reciters state a license for redistribution.

The app is offline first and the webview has no file system access (spec §14.3), so streaming
from a CDN would break the first and loading files with the asset protocol would weaken the
second.

## Decision

- **Source.** Recitations are taken directly from Quran.com: the 12 verse recitations of its API
  v4 (one mp3 per verse). Quran.com has two more reciters only as surah files with verse
  timings (Yasser ad-Dussary, Khalifah al-Tunaiji), but the timings do not fit the files (some
  surah files are shorter than their timings, some timings are out of order), so cutting them
  would put verse boundaries in the wrong place. Their packs take the same recitations as
  verse files from EveryAyah, whose recordings Quran.com also serves. The recording where
  children repeat after the reciter and a duplicate streaming copy of al-Afasy are left out.
  That is 14 reciters.
- **Packs.** Each reciter is one `audio.<reciter>` pack: an archive pack of `audio.sqlite`
  (`pack_info` only), `audio/LICENSE` and exactly 6,236 files `audio/SSSAAA.mp3`, re-encoded to
  mono MP3 at 64 kbit/s (pack-format §4.10). A reciter is 0.5–1.2 GB, below GitHub's 2 GiB
  limit for a release asset. The packs are built once by a maintainer (`siyak-data build-audio`),
  uploaded to an `audio-*` release and pinned in `source.toml`; data releases list the pinned
  assets without rebuilding them.
- **Rights.** The maintainer decided to publish the packs with attribution ("Recitation: … ·
  Source: Quran.com" or "… EveryAyah"), for personal and non-commercial listening, and to remove a pack from the
  manifest when a rights holder asks. This is a deliberate exception to principle 4, recorded
  here; `license_terms` say so before download, and `audio/LICENSE` is on About → Sources.
- **Playback.** The core registers the `siyak-audio:` URI scheme (`src-tauri/src/audio.rs`). It
  serves `/<pack id>/<verse id>.mp3` with byte ranges, only for installed packs of type `audio`,
  and builds the path on disk itself from the checked pack id and verse number. The CSP allows
  the scheme as `media-src` and nothing else changes: no `fs` permission, no asset protocol.
- **Schema.** `packs.type` gains `'audio'` (content schema 3, migration `0003_audio`). Packs that
  hold only files (model, audio) install under any content schema up to the current one, so a
  later schema change does not force rebuilding gigabytes of audio.

## Consequences

- Recitation works offline once a reciter is installed; the reader shows a play action on every
  verse, plays on to the end of the surah, plays the reciter's basmala (1:1) before the first
  verse of every surah except al-Fātiḥa and at-Tawba, and highlights and follows the verse.
- The packs carry recordings whose redistribution was not cleared. If a reciter, a producer or
  Quran.Foundation objects, the pack leaves the manifest in the next data release; installed
  copies stay on users' computers.
- Re-encoding loses some quality compared with the 64–192 kbit/s originals; speech at 64 kbit/s
  mono stays clear. The original files are pinned by sha256 (`files.tsv`), so a pack can be
  rebuilt at another bitrate.
- Linux needs GStreamer's MP3 decoder (`gst-plugins-good`) for WebKitGTK to play the files.
- Rebuilding the `packs` table needed a migration that runs with foreign keys off; the migration
  runner gained `foreign_keys_off` and checks the foreign keys before committing.
