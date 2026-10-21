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
│ later: search, morphology, related, embed, packs, llm,      │
│        export, net, secrets                                 │
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

- **First launch:** if `content.db` is missing, the bundled seed is copied into place. Debug builds
  use `dev-data/dev-content.db`; release builds use `resources/core.db` (for now the same file,
  bundled through `tauri.conf.json`; the real `core.db` comes from the data pipeline in M1).
- **Migrations:** ordered SQL files in `src-tauri/migrations/{content,user}/NNNN_name.sql`, embedded
  at compile time and applied on startup inside one transaction, tracked by `PRAGMA user_version`.
  `user.db` is backed up to `user.db.bak` before it is migrated. An app never writes to a database
  created by a newer version (`db_version_unsupported`).
- **Failure handling:** if the databases cannot be opened, the window still appears and every
  command returns the error code, which the interface shows as a translated message.

## Packs (summary)

Optional content — translations, tafsirs, morphology, vectors and embedding models — is published
as versioned packs on GitHub Releases of the `siyak-data` repository and listed in a manifest
(spec §6). The app downloads a pack, verifies its sha256 and copies only the expected tables and
columns into `content.db` in one transaction. Packs are planned for M3; `src-tauri/src/packs/` is
the seam.

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
   in Rust (`net.rs`, M3/M8).
2. **The webview gets no plugin permissions.** No `fs`, `http` or `shell`; the main capability
   grants only the app's own commands.
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
| the right-hand panel               | `src/features/context-panel/`                                                                                                |
| settings                           | `src/features/settings/`, `src/stores/ui.ts`, `src/app/preferences.ts`                                                       |
| wording in the interface           | `src/i18n/en.json` and `src/i18n/tr.json`                                                                                    |
| colours, fonts, theme              | `src/styles.css`                                                                                                             |
| base UI components                 | `src/components/ui/` (shadcn/ui, owned by us)                                                                                |
| development data                   | `dev-data/fixtures/`, then `cargo xtask build-dev-db` (`xtask/src/dev_db.rs`)                                                |
| CI                                 | `.github/workflows/ci.yml`                                                                                                   |
| the website                        | `website/src/content/docs/{en,tr}/`                                                                                          |

Modules for later milestones (`search`, `morphology`, `related`, `embed`, `packs`, `llm`, `export`,
`net`, `secrets`) already exist as documented, empty files; their `//!` comments say what they will
hold and in which milestone.
