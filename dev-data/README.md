# Development data

`dev-content.db` is a small seed database that debug builds copy to `content.db` on first launch,
so contributors can run Siyak without the data pipeline (spec §16.3). Release builds bundle the
real data instead, `src-tauri/resources/core.db` (see [Real data](#real-data-coredb) below).

## Contents

| Table                                       | Rows                                                                                     |
| ------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `surahs`                                    | all 114 surahs; `bismillah` (Tanzil's basmala) only for surahs whose text is in the seed |
| `verses`                                    | surahs 1, 2, 9, 112, 113 and 114 only (437 verses), Uthmani and Simple Clean text        |
| `packs`                                     | `en.pickthall` (translation, bundled), `morphology.qac` (morphology, not bundled)        |
| `translations`, `translations_fts`          | Pickthall for the 437 verses                                                             |
| `notices`                                   | the Tanzil copyright blocks of both texts, as in `core.db`, and the QAC notice           |
| `words`, `word_segments`, `roots`, `lemmas` | the `morphology.qac` rows for the 437 verses (8,701 words)                               |
| `content_meta`                              | `core_version` (`CORE_VERSION` in `xtask/src/dev_db.rs`) and `tanzil_version`            |
| `verses_fts`                                | the normalized Simple Clean text of the 437 verses, written by the app's Rust normalizer |

The schema comes from the app's own migrations (`src-tauri/migrations/content/`), so it always
matches what the app expects. There is deliberately **no Turkish translation** yet: the Turkish
source is still being verified (spec §18, question 6).

## Fixtures and sources

| File                              | Source                                                                                                                                                                                                                                               | License                                                                                         |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `fixtures/quran-uthmani.xml`      | [Tanzil](https://tanzil.net/download/) Uthmani text 1.1, XML: `quranType=uthmani&outType=xml&marks=true&sajdah=true&alef=true&tatweel=true` (pause marks, sajdah signs, tatweel below superscript alefs; rub' al-hizb signs off)                     | CC BY 3.0, verbatim copies only                                                                 |
| `fixtures/quran-simple-clean.xml` | Tanzil Simple Clean text 1.1, XML, no options: `quranType=simple-clean&outType=xml`                                                                                                                                                                  | CC BY 3.0, verbatim copies only                                                                 |
| `fixtures/en.pickthall.txt`       | [Tanzil translations](https://tanzil.net/trans/), `en.pickthall`, "Text (with aya numbers)"                                                                                                                                                          | Translation in the public domain (Pickthall, 1930); see the Tanzil terms below                  |
| `fixtures/surahs.tsv`             | `name_ar`, `revelation_type`, `ayah_count`: [Tanzil metadata](https://tanzil.net/res/text/metadata/quran-data.xml). `name_tr`: `siyak-data/data/surah-names-tr.csv`. `name_en`: Tanzil's transliterated name (`tname`). Both exactly as in `core.db` | Metadata: CC BY 3.0 (Tanzil)                                                                    |
| `fixtures/juz.tsv`                | Juz start verses from the Tanzil metadata                                                                                                                                                                                                            | CC BY 3.0 (Tanzil)                                                                              |
| `fixtures/packs.json`             | Pack metadata for the bundled translation, written by the maintainers                                                                                                                                                                                | MIT OR Apache-2.0                                                                               |
| `fixtures/morphology.tsv`         | One row per segment, exported from the `morphology.qac` tables built by `siyak-data` (`word_segments` joined with `words`, `lemmas`, `roots`) for the surahs in the seed. Format changed only; no tag added, removed or corrected (ADR 0007)         | GNU GPL with the QAC terms of use (`qac-NOTICE.txt`): non-commercial use, source and link shown |
| `fixtures/qac-NOTICE.txt`         | Exact copy of `siyak-data/sources/morphology/qac/NOTICE`                                                                                                                                                                                             | Must ship with the data                                                                         |
| `fixtures/normalize_vectors.json` | Exact copy of `siyak-data/tests/normalize_vectors.json`, the contract of the Arabic search normalizer (a test checks the two match when both repositories are present)                                                                               | MIT OR Apache-2.0                                                                               |

The Tanzil files are excerpts of the official downloads: the XML declaration, the copyright
comment and the `<sura>` elements for surahs 1, 2, 9, 112, 113 and 114 are kept byte for byte.
Nothing inside them has been changed. They use the same edition and options as `siyak-data`
(`sources/tanzil/source.toml` there, spec §5.1): the elements were copied from its pinned version
1.1 downloads, and the header and the elements already present matched those files byte for byte.
Surah 2 (al-Baqarah, 286 verses) tests the reader on a long surah; surah 9 (at-Tawba) has no
basmala.

The surah names in `surahs.tsv` are those of `core.db`, so the development data and the full data
show the same names. TODO: Tanzil's English transliterations (e.g. "Aal-i-Imraan") are not ideal
for the interface; a curated English list belongs in `siyak-data` (`data/surah-names-en.csv`, like
the Turkish one), after which this file is updated to match.

**Why XML for the Arabic text:** Tanzil's plain-text format adds the basmala to the start of the
first verse of each surah. The XML format keeps it in a separate `bismillah` attribute, so
`verses.text_uthmani` contains only the verse itself.

`morphology.qac` is not bundled in the seed's `packs` row, so a contributor can remove it on the
Packs screen to work on what the app shows without morphology.

## Required attribution

The Tanzil copyright block is kept in every fixture file and must stay there. Wherever this data
is shown, the source must be stated with a link:

> Quran text: Tanzil Project (https://tanzil.net), licensed under Creative Commons Attribution 3.0.
> Changing the text is not allowed.

The Tanzil translation terms say: _"The translations provided at this page are for non-commercial
purposes only. If used otherwise, you need to obtain necessary permission from the translator or
the publisher."_ Siyak is non-commercial, and Pickthall's translation is in the public domain.

Morphology data comes from the Quranic Arabic Corpus, whose terms require the source and a link
wherever it is used:

> Source: Quranic Arabic Corpus (https://corpus.quran.com).

## Real data (`core.db`)

`src-tauri/resources/core.db` is the complete database built by
[`siyak-data`](https://github.com/siyak-app/siyak-data) (all 6,236 verses, the bundled translations,
the Tanzil notices). Release builds bundle it and copy it to `content.db` on first launch.

| Field             | Value                                                              |
| ----------------- | ------------------------------------------------------------------ |
| siyak-data commit | `72bc7953fc37b93dc17e6ad7ab421147ef64358e`                         |
| sha256            | `76ed118ad4be5faf020e65f1507c7a380321499e7b190daf3d008f9a1635a50d` |
| `core_version`    | 1                                                                  |
| `tanzil_version`  | 1.1                                                                |

To update it, build `core.db` in a `siyak-data` checkout next to this repository, then run
`cargo xtask sync-core` (or `cargo xtask sync-core --from <path>`). The task checks the row
counts, `core_version` (it must never go down) and that the schema is exactly what the app's
content migrations create, copies the file and prints the values for the table above. Commit the
database together with the updated table.

## Regenerating

```sh
cargo xtask build-dev-db
```

This rebuilds `dev-data/dev-content.db` from `fixtures/` (the code is in `xtask/src/dev_db.rs`).
The output is deterministic, so a rebuild without fixture changes leaves the file unchanged. Commit
the regenerated database together with any fixture change. Keep the file under 5 MB.

To add a surah, download the full XML files with the options above and copy the surah's `<sura>`
element unchanged into both fixture files. Then add the matching lines from the full
`en.pickthall` file and run the command again.
