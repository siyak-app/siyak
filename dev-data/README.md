# Development data

`dev-content.db` is a small seed database that debug builds copy to `content.db` on first launch,
so contributors can run Siyak without the data pipeline (spec §16.3). Release builds bundle the
same file as `resources/core.db` until the real `core.db` arrives in milestone M1.

## Contents

| Table                              | Rows                                                                                     |
| ---------------------------------- | ---------------------------------------------------------------------------------------- |
| `surahs`                           | all 114 surahs; `bismillah` (Tanzil's basmala) only for surahs whose text is in the seed |
| `verses`                           | surahs 1, 112, 113 and 114 only (22 verses), Uthmani and Simple Clean text               |
| `packs`                            | `en.pickthall` (translation, bundled)                                                    |
| `translations`, `translations_fts` | Pickthall for the 22 verses                                                              |
| `notices`                          | the Tanzil copyright blocks of both texts, as in `core.db`                               |
| `content_meta`                     | `core_version` (`CORE_VERSION` in `xtask/src/dev_db.rs`) and `tanzil_version`            |
| `verses_fts`                       | empty until search arrives in M4 (the app's Rust normalizer fills it); full data has it  |

The schema comes from the app's own migrations (`src-tauri/migrations/content/`), so it always
matches what the app expects. There is deliberately **no Turkish translation** yet: the Turkish
source is still being verified (spec §18, question 6).

## Fixtures and sources

| File                              | Source                                                                                                                                                                                                                         | License                                                                        |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------ |
| `fixtures/quran-uthmani.xml`      | [Tanzil](https://tanzil.net/download/) Uthmani text, XML, default options: `quranType=uthmani&outType=xml&marks=true&sajdah=true&alef=true&tatweel=true`                                                                       | CC BY 3.0, verbatim copies only                                                |
| `fixtures/quran-simple-clean.xml` | Tanzil Simple Clean text, XML, no options: `quranType=simple-clean&outType=xml`                                                                                                                                                | CC BY 3.0, verbatim copies only                                                |
| `fixtures/en.pickthall.txt`       | [Tanzil translations](https://tanzil.net/trans/), `en.pickthall`, "Text (with aya numbers)"                                                                                                                                    | Translation in the public domain (Pickthall, 1930); see the Tanzil terms below |
| `fixtures/surahs.tsv`             | `name_ar`, `revelation_type`, `ayah_count`: [Tanzil metadata](https://tanzil.net/res/text/metadata/quran-data.xml). `name_tr`, `name_en`: common Turkish names and common English transliterations, written by the maintainers | Metadata: CC BY 3.0 (Tanzil)                                                   |
| `fixtures/juz.tsv`                | Juz start verses from the Tanzil metadata                                                                                                                                                                                      | CC BY 3.0 (Tanzil)                                                             |
| `fixtures/packs.json`             | Pack metadata for the bundled translation, written by the maintainers                                                                                                                                                          | MIT OR Apache-2.0                                                              |

The Tanzil files are excerpts of the official downloads: the XML declaration, the copyright
comment and the `<sura>` elements for surahs 1, 112, 113 and 114 are kept byte for byte. Nothing
inside them has been changed.

**Why XML for the Arabic text:** Tanzil's plain-text format adds the basmala to the start of the
first verse of each surah. The XML format keeps it in a separate `bismillah` attribute, so
`verses.text_uthmani` contains only the verse itself.

## Required attribution

The Tanzil copyright block is kept in every fixture file and must stay there. Wherever this data
is shown, the source must be stated with a link:

> Quran text: Tanzil Project (https://tanzil.net), licensed under Creative Commons Attribution 3.0.
> Changing the text is not allowed.

The Tanzil translation terms say: _"The translations provided at this page are for non-commercial
purposes only. If used otherwise, you need to obtain necessary permission from the translator or
the publisher."_ Siyak is non-commercial, and Pickthall's translation is in the public domain.

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
