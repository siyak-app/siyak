# Contributing to Siyak

Thank you for helping. Siyak welcomes code, data and license research, translations, reviews of
terminology and documentation. This guide is in English; a Turkish version is in
[CONTRIBUTING.tr.md](CONTRIBUTING.tr.md).

**The 15-minute goal:** on a machine with the prerequisites installed, you should be able to clone
the repository and see the app running in under 15 minutes. If something slows you down, that is a
bug: please open an issue.

## Prerequisites

All platforms:

- [Git](https://git-scm.com/)
- [Rust](https://rustup.rs/) via rustup. The exact toolchain is pinned in `rust-toolchain.toml`
  and installed automatically on the first `cargo` command.
- [Node.js](https://nodejs.org/) 24 LTS or newer
- [pnpm](https://pnpm.io/installation) 12: `npm install -g pnpm@12`

Then, for your operating system:

### Windows

- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the
  "Desktop development with C++" workload (MSVC).
- WebView2 runtime. It is preinstalled on Windows 10 (1803 and later) and Windows 11; otherwise
  install the [Evergreen Bootstrapper](https://developer.microsoft.com/microsoft-edge/webview2/).

### macOS

- Xcode Command Line Tools: `xcode-select --install`

### Linux

Debian / Ubuntu:

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Fedora:

```sh
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```

### Dev container

Alternatively, open the repository in the [dev container](.devcontainer/) (VS Code "Reopen in
Container" or GitHub Codespaces). It has Rust, Node, pnpm and the Linux libraries ready.

## Setup

```sh
git clone https://github.com/siyak-app/siyak.git
cd siyak
pnpm install
pnpm tauri dev
```

The first `pnpm tauri dev` compiles the Rust core and takes a few minutes; later runs are fast. The
app opens with the complete Quran data (`src-tauri/resources/core.db`), so you never need the data
pipeline.

App data (the two databases) is stored in the OS app data directory for `com.kemalozyon.siyak`,
e.g. `~/Library/Application Support/com.kemalozyon.siyak` on macOS,
`%APPDATA%\com.kemalozyon.siyak` on Windows and `~/.local/share/com.kemalozyon.siyak` on Linux.
Deleting that directory resets the app.

### Quran data

Debug and release builds use `src-tauri/resources/core.db`, the complete data built by
[`siyak-data`](https://github.com/siyak-app/siyak-data). Its origin is recorded in
[`dev-data/README.md`](dev-data/README.md#real-data-coredb): currently siyak-data commit
`72bc795`, sha256 `76ed118ad4be…`, `core_version` 1. Maintainers update it with
`cargo xtask sync-core`.

To run a debug build on the small development dataset (`dev-data/`), which already includes
morphology, start it with `SIYAK_DEV_DATA=dev pnpm tauri dev`. Each dataset gets its own content
database (`content.db` for the development data, `content.full.db` for the full data), so you can
switch back and forth without resetting anything; `user.db` is shared. Debug builds show which
dataset is loaded in the header. Published packs cover the whole Quran, so they install only on the
full data; on the development data the app refuses them and says so.

To try packs without publishing them, build them in `siyak-data` and point a debug build at its
output: `SIYAK_PACK_SOURCE=../siyak-data/dist pnpm tauri dev`. The app then reads `manifest.json`
from that directory and copies each pack from it by file name, with the same checks as a download.

## Scripts

| Command                    | What it does                                                           |
| -------------------------- | ---------------------------------------------------------------------- |
| `pnpm tauri dev`           | Run the app with hot reload                                            |
| `pnpm tauri build --debug` | Build an unsigned debug app and installer                              |
| `pnpm lint`                | ESLint, including the i18n and right-to-left rules                     |
| `pnpm format`              | Format with Prettier (`pnpm format:check` only checks)                 |
| `pnpm typecheck`           | TypeScript type check                                                  |
| `pnpm test`                | Frontend tests (Vitest)                                                |
| `pnpm docs:ts`             | Check TSDoc of the public query hooks (TypeDoc)                        |
| `pnpm changelog`           | Regenerate `CHANGELOG.md` from Conventional Commits (git-cliff)        |
| `pnpm --dir website dev`   | Preview the website                                                    |
| `cargo test`               | Rust tests                                                             |
| `cargo xtask bindings`     | Regenerate `src/lib/bindings.ts` after changing a command or its types |
| `cargo xtask build-dev-db` | Rebuild `dev-data/dev-content.db` after changing `dev-data/fixtures/`  |
| `cargo xtask sync-core`    | Check and copy `siyak-data`'s `core.db` to `src-tauri/resources/`      |

## Running the checks locally

CI runs these on every pull request; run them before you push:

```sh
pnpm lint && pnpm format:check && pnpm typecheck && pnpm test && pnpm docs:ts
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo xtask bindings && git diff --exit-code -- src/lib/bindings.ts
pnpm --dir website build
```

## Conventions

- **English in code.** Identifiers, comments, commit messages, branch names and PR text are in
  English. Islamic terms follow the [glossary](docs/glossary.md): `surah`, `verse`, `translation`,
  `tafsir`, never `sure`, `ayet` or `meal`.
- **No hard-coded user-facing text.** Add every string to both `src/i18n/en.json` and
  `src/i18n/tr.json`. Technical terms get a plain and a technical form
  (`term.<name>.plain` / `term.<name>.technical`).
- **Right-to-left aware.** Use logical Tailwind utilities (`ms-`, `me-`, `ps-`, `pe-`, `start-`,
  `end-`); Arabic text gets `dir="rtl" lang="ar"`.
- **Errors are codes.** Rust returns `AppError` with a stable code; the interface translates it. No
  `unwrap()`/`expect()` outside tests and unrecoverable startup code.
- **Documented code.** Every Rust module starts with `//!`, every public item has `///`, public
  hooks have TSDoc. Comments explain why, and link the spec section or ADR when logic is not
  obvious.
- **Small and focused.** No dead or commented-out code; later milestones get a documented seam,
  not a half feature.

### Branches and commits

- Branch from `main`: `feat/reader-verse-actions`, `fix/settings-save`, `docs/glossary-terms`.
- Commit messages and PR titles follow [Conventional Commits](https://www.conventionalcommits.org/):
  `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `build:`, `ci:`, `chore:`. PRs are squash-merged,
  so the PR title becomes the commit message (CI checks it).

### Pull requests

1. For anything bigger than a small fix, open or comment on an issue first.
2. Architectural changes start as an ADR proposal: a PR adding `docs/adr/NNNN-title.md`
   ([template](docs/adr/0000-template.md)).
3. Fill in the [pull request template](.github/PULL_REQUEST_TEMPLATE.md), including how the change
   looks in the reader (simple) defaults and whether both languages were updated.
4. CI must be green and one maintainer approval is needed.

Definition of done: code, tests, English documentation where relevant, and a bilingual guide update
if the change is user-facing.

## Ways to contribute

| Path                                                                     | Where                                                 | Guide                                                                 |
| ------------------------------------------------------------------------ | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Code                                                                     | this repository                                       | this file, issues labelled `good first issue`                         |
| New translation or tafsir sources, license research                      | [siyak-data](https://github.com/siyak-app/siyak-data) | `docs/adding-a-source.md` there, the "New source proposal" issue form |
| Interface translation (a new language)                                   | `src/i18n/`                                           | `docs/translating.md` (coming)                                        |
| Reviewing Turkish labels and term translations; reporting QAC tag errors | issues and Discussions                                | the "Translation or term correction" and "QAC tag error" issue forms  |
| Documentation                                                            | `website/`                                            | the "Edit this page" link on every page                               |

Issues and Discussions are welcome in Turkish or English.

## Community files

The Code of Conduct, the security policy (how to report vulnerabilities privately) and support
information live in the organization's [`.github` repository](https://github.com/siyak-app/.github) and
apply to every Siyak repository.

## License of contributions

Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in
Siyak is dual-licensed under `MIT OR Apache-2.0`, as described in [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE), without any additional terms or conditions.
