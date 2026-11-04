# Architecture

This is a map of Siyak for contributors: what the parts are, how data flows between them and which
rules must not be broken. The full design, including features that are not built yet, is in the
[specification](docs/spec/spec.md); decisions are recorded in [docs/adr/](docs/adr/README.md).

## Layers

```
┌──────────────── Webview (React, TypeScript) ────────────────┐
│ screens and components · TanStack Router · TanStack Query   │
│ src/lib/queries.ts ──▶ src/lib/bindings.ts (generated)      │
└──────────────────────────────┬──────────────────────────────┘
                               │ Tauri IPC (typed commands)
┌──────────────────────────────┴───── Rust core (src-tauri) ──┐
│ commands/ ──▶ db/ ──▶ content.db + user.db (SQLite)         │
│ commands/packs ──▶ packs/ ──▶ net (allow-listed HTTP)       │
│ commands/search ──▶ search (FTS5)                           │
│ commands/morphology ──▶ morphology (words, concordance)     │
│ later: related, embed, llm, export, secrets                 │
└─────────────────────────────────────────────────────────────┘
```

- **The webview is only the user interface.** It renders, keeps UI preferences and calls commands.
  It has no file system, network or shell access (ADR 0001, spec §3).
- **The Rust core owns everything else:** both databases, pack files, models, network calls and
  API keys.

## The two databases

Both live in the operating system's app data directory (`app_data_dir()`, e.g.
`~/Library/Application Support/com.kemalozyon.siyak` on macOS). See ADR 0002.

| File         | Holds                                                                 | Written by                       |
| ------------ | --------------------------------------------------------------------- | -------------------------------- |
| `content.db` | Quran text, surah metadata, installed packs (translations, tafsir, …) | first-launch seed, pack installs |
| `user.db`    | bookmarks, notes, collections, settings, AI caches and history        | the user's actions               |

- **First launch:** if the content file is missing, a seed is copied into place. Release builds
  seed `content.db` from the bundled `resources/core.db`, the real Quran data built by `siyak-data`
  and copied in with `cargo xtask sync-core`. Debug builds pick the seed with `SIYAK_DEV_DATA`: unset
  or `dev` seeds `content.db` from the small `dev-data/dev-content.db`, `full` seeds a separate
  `content.full.db` from `resources/core.db`. Switching never mixes the two; `user.db` is shared,
  because user data refers to verses by their stable global id. The seed is recorded as
  `content_meta.seed`, and debug builds show it in a badge in the header.
- **Migrations:** ordered SQL files in `src-tauri/migrations/{content,user}/NNNN_name.sql`, embedded
  at compile time and applied on startup inside one transaction, tracked by `PRAGMA user_version`.
  `user.db` is backed up to `user.db.bak` before it is migrated. An app never writes to a database
  created by a newer version (`db_version_unsupported`).
- **Core refresh:** on every start, after migrating, the app compares `content_meta.core_version`
  of its content file with the bundled seed's. When the bundled one is newer it backs up the content
  file to `content.db.bak` and, in one transaction, replaces the core tables (surahs, verses, their
  search index, notices and the bundled translations) while leaving installed packs untouched. On
  failure the transaction rolls back, the backup stays, the app keeps running on the old core and
  the interface shows `core_refresh_failed` (spec §6.2, ADR 0011).
- **Failure handling:** if the databases cannot be opened, the window still appears and every
  command returns the error code, which the interface shows as a translated message.

## The reader

- **Data.** `get_surah` and `get_verses` load a whole surah with the selected translations in one
  call (al-Baqarah with one translation is about 200 KB of JSON and a few milliseconds in a release
  build), so there is no paging. TanStack Query caches it.
- **Virtualization.** `ReaderView` renders the verses with TanStack Virtual inside a plain scroll
  container: only the visible verses plus a few above and below are in the DOM, heights are
  measured after rendering (`measureElement`) and estimated from the text length before.
  `VerseItem` is memoized, so scrolling renders only the verses entering the window. The surah
  header (names, translation titles, basmala) sits above the list and is passed to the virtualizer
  as `scrollMargin`.
- **Routes and positions.** `/read/:surah` and `/read/:surah/:ayah`; a verse link scrolls to the
  verse and highlights it briefly. The verse at the top is remembered per surah for the session
  and stored as `reader.last_position`, which the index route opens on the next launch.
- **Keyboard.** The verse list is one tab stop with roving focus: ↑/↓, Home and End move between
  verses (scrolling the virtual list as needed) and Enter opens the verse in the context panel,
  as does a click (except one that ends a text selection, so text can still be copied).
  Ctrl/⌘+K opens the command palette, whose "Go to" understands `2:255`, `Bakara 255` and
  `Baqarah 255` (`src/lib/goTo.ts`).
- **Translations and actions.** `reader.translations` is an ordered list of pack ids; each block
  gets its pack's `lang` and `dir`. The copy actions use the web clipboard API and announce the
  result through the app's one live region (`LiveAnnouncer`).

## Settings and the experience profile

- **One typed model.** Settings are stored in `user.db` as `key → JSON text`. `src/lib/settings.ts`
  declares every key with its type, validator and default, and is the only place that parses and
  serializes them; an invalid stored value falls back to its default. The Zustand store
  (`src/stores/ui.ts`) holds the parsed values; `usePreferencesSync` loads them once and writes
  each change back, debounced.
- **Experience profile.** The first-launch question ("Read and understand" / "Do research") sets
  `experience_profile` and writes every key it governs at once with `set_settings`
  (`src/features/profile/`). Settings → Display & behaviour shows each of those keys, disables the
  ones whose features arrive later, and can reset to either profile. Nothing is locked to a
  profile; it only chooses defaults (spec §13.2).
- **Technical terms.** Wording that names a linguistic concept goes through `useTerm` or `<Term>`
  (`src/i18n/terms.ts`), which show the plain wording with the technical term in a tooltip or in
  brackets, following `ui.technical_terms`.
- **Appearance.** Themes are CSS variables in `src/styles.css` (light, dark, sepia); a test checks
  their contrast. Typography settings become CSS variables on `<html>` used by `.arabic-text` and
  `.translation-text`.

## Packs (summary)

Optional content — translations, tafsirs, morphology, vectors and embedding models — is published
as versioned packs on GitHub Releases of the `siyak-data` repository and listed in a manifest
(spec §6). The app downloads a pack, verifies its sha256 and copies only the expected tables and
columns into `content.db` in one transaction.

- `packs/manifest.rs` fetches and caches the manifest (one hour; the last good copy offline) and
  builds the catalog the Packs screen shows. `packs/download.rs` downloads with resume and checks
  sizes and sha256; `packs/install.rs` installs, removes and cleans up after interrupted jobs.
- `packs/service.rs` is the queue: one job at a time, each on its own `content.db` connection, so
  reading never waits for an install. Progress reaches the webview as the `pack://progress`,
  `pack://done` and `pack://error` events; `usePackEvents` patches them into the cached catalog.
- `net.rs` is the only HTTP client: a fixed host allow-list, redirects checked hop by hop and a
  connection log without content in `user.db` (Settings → Privacy).
- Debug builds can read packs from a local directory with `SIYAK_PACK_SOURCE` (see CONTRIBUTING).

The context panel's Tafsir tab reads a passage through `get_tafsir` and follows the selected verse.
Passage text is untrusted: `TafsirText` parses it in an inert document and rebuilds only the allowed
elements of spec §5.2, without attributes, so no markup from a pack reaches the page.

The compare screen (`/compare/:surah/:range`) is a table with one row per verse. `get_compare` returns
the verses and each tafsir's overlapping passages; `tafsirCells` lays a passage covering several verses
over their rows as one cell, which scrolls so that long commentary does not stretch the rows.

## Search (summary)

`search.rs` holds the single search box's logic (spec §7): `classify_query` decides whether a query
is a verse to jump to, Arabic letters, a Latin-script root or other text, and `search_text` queries
`verses_fts` or `translations_fts`. User input never reaches FTS5 as syntax: `fts_query` rebuilds it
from quoted strings. `normalize` is the port of the pipeline's Arabic normalizer; it is tested against
the shared vectors (`dev-data/fixtures/normalize_vectors.json`) and must reproduce `core.db`'s index.
The Arabic index holds normalized text that is never displayed, so its matches are marked word by word
on `text_clean`. Results carry their text as plain parts, never markup.

## IPC and generated bindings

- Commands are `async` functions in `src-tauri/src/commands/`, annotated with `#[tauri::command]`
  and `#[specta::specta]`. They return `Result<T, AppError>`.
- `AppError` is `{ code, message }`. `code` is a stable `snake_case` identifier; `message` is a
  developer diagnostic. Rust never produces user-facing text.
- `tauri-specta` generates `src/lib/bindings.ts` from the commands and their types. Debug builds
  rewrite it on startup; `cargo xtask bindings` does the same, and CI fails if the committed file
  is out of date. Never edit it by hand.
- Components never call the bindings directly: `src/lib/queries.ts` wraps them in TanStack Query
  hooks and turns errors into `AppErrorException` with the code.
- Every command must be listed in `src-tauri/src/commands/names.rs` and granted in
  `src-tauri/capabilities/main.json`; otherwise Tauri refuses the call. A test checks the list
  against the registered commands.

## Interface principles

- **i18n everywhere.** Every visible string is a key in `src/i18n/en.json` and `src/i18n/tr.json`.
  ESLint rejects literal strings in JSX, TypeScript rejects unknown keys and a test checks that
  both files have the same keys. Error codes map to `errors.<code>`.
- **Right-to-left text.** Arabic blocks use `dir="rtl" lang="ar"` and the bundled Amiri Quran font;
  translations use the pack's language. Only logical Tailwind utilities (`ms-`, `pe-`, `start-`, …)
  are allowed (enforced by ESLint).
- **Progressive disclosure.** One interface for readers and researchers: screens open simple and
  reveal depth on request (ADR 0005). Each new feature must define how it looks in the reader
  (simple) defaults.
- **Three panes.** Navigation (left), content (center) and context (right). Pane sizes and
  visibility are UI preferences, kept in a Zustand store and saved to `user.db` settings.

## Security invariants

These hold in every change; a pull request that weakens one needs an ADR.

1. **The webview never talks to the network.** The CSP is `default-src 'self'` (plus Tauri's IPC
   endpoint); fonts, scripts and styles are bundled. All HTTP goes through one allow-listed client
   in Rust (`net.rs`).
2. **The webview gets no plugin permissions.** No `fs`, `http` or `shell`; the main capability
   grants only the app's own commands and listening to its events. Links (a pack's source) are
   opened by the Rust side from addresses it already holds.
3. **API keys never reach the webview.** They live in the OS keychain and the interface only learns
   whether a key is saved (`secrets.rs`, M8).
4. **Packs are untrusted input.** Only expected tables and columns are copied; no SQL, triggers or
   views from a pack are executed; every download is checked against its sha256.
5. **Machine output is labelled.** AI answers and translations are always shown next to their
   source with provider, model and date.

## Module map: to change X, look at Y

| To change…                         | Look at                                                                                                                      |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| a database table                   | a new file in `src-tauri/migrations/{content,user}/`, then `src-tauri/src/db/`                                               |
| what a command returns             | `src-tauri/src/db/*.rs` (queries and types), `src-tauri/src/commands/`                                                       |
| add a command                      | `commands/` + `commands/names.rs` + `capabilities/main.json`, then `cargo xtask bindings` and a hook in `src/lib/queries.ts` |
| error codes                        | `src-tauri/src/error.rs` and `errors.*` in both i18n files                                                                   |
| app startup, seed selection, state | `src-tauri/src/lib.rs`                                                                                                       |
| window, CSP, bundle                | `src-tauri/tauri.conf.json`, `src-tauri/capabilities/`                                                                       |
| the layout and panes               | `src/app/layout/AppShell.tsx`, `src/stores/ui.ts`                                                                            |
| routes                             | `src/app/router.tsx`                                                                                                         |
| the reader                         | `src/features/reader/`                                                                                                       |
| the command palette, "Go to"       | `src/features/command-palette/`, `src/lib/goTo.ts`                                                                           |
| the right-hand panel               | `src/features/context-panel/`                                                                                                |
| a setting or its default           | `src/lib/settings.ts`, then `src/features/settings/`                                                                         |
| profile defaults, first launch     | `src/features/profile/profileDefaults.ts`, `src/features/onboarding/`                                                        |
| settings storage and sync          | `src/stores/ui.ts`, `src/app/preferences.ts`                                                                                 |
| wording in the interface           | `src/i18n/en.json` and `src/i18n/tr.json`                                                                                    |
| colours, fonts, theme              | `src/styles.css`                                                                                                             |
| base UI components                 | `src/components/ui/` (shadcn/ui, owned by us)                                                                                |
| development data                   | `dev-data/fixtures/`, then `cargo xtask build-dev-db` (`xtask/src/dev_db.rs`)                                                |
| the bundled Quran data             | `siyak-data`, then `cargo xtask sync-core` (`xtask/src/sync_core.rs`)                                                        |
| core refresh                       | `src-tauri/src/db/content.rs` (`refresh_core`)                                                                               |
| CI                                 | `.github/workflows/ci.yml`                                                                                                   |
| the website                        | `website/src/content/docs/{en,tr}/`                                                                                          |

Modules for later milestones (`related`, `embed`, `llm`, `export`,
`secrets`) already exist as documented, empty files; their `//!` comments say what they will
hold and in which milestone.
