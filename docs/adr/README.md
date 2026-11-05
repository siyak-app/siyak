# Architecture Decision Records

Each record explains one decision: its context, the decision and its consequences. To propose an
architectural change, open a pull request that adds `NNNN-short-title.md` based on
[the template](0000-template.md) with status "Proposed".

| #                                                 | Decision                                                                      |
| ------------------------------------------------- | ----------------------------------------------------------------------------- |
| [0001](0001-tauri-v2-and-react.md)                | Build the desktop app with Tauri v2 and React                                 |
| [0002](0002-two-databases.md)                     | Keep content and user data in two databases                                   |
| [0003](0003-tafsir-is-never-embedded.md)          | Look tafsir up by verse; never embed it                                       |
| [0004](0004-on-demand-tafsir-translation.md)      | Translate Arabic tafsir on demand with the user's own model                   |
| [0005](0005-progressive-disclosure.md)            | Use progressive disclosure instead of separate modes                          |
| [0006](0006-license.md)                           | License the code as MIT OR Apache-2.0; data keeps its own licenses            |
| [0007](0007-qac-morphology-unmodified.md)         | Use the Quranic Arabic Corpus v0.4 morphology, unmodified                     |
| [0008](0008-classical-tafsirs-from-quranpedia.md) | Take classical tafsirs from the Quranpedia dumps; no contemporary works in v1 |
| [0009](0009-english-code-bilingual-docs.md)       | Write code in English; keep documentation bilingual                           |
| [0010](0010-rust-xtask.md)                        | Automate repository tasks with a Rust xtask                                   |
| [0011](0011-core-refresh-strategy.md)             | Refresh the core data in place, in one transaction                            |
| [0012](0012-local-and-api-embedders.md)           | Embed queries locally by default, or with an embedding API                    |
