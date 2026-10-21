# 0001. Build the desktop app with Tauri v2 and React

- Status: Accepted
- Date: 2026-10-21

## Context

Siyak must run on Windows, macOS and Linux, work fully offline, keep database, file system and
network access out of reach of the user interface, and stay approachable for contributors
(spec §1, §3). Electron bundles a whole browser per app; native toolkits would need three UIs.

## Decision

Use Tauri v2: a Rust core plus the operating system's webview. The interface is React with
TypeScript (strict) and Vite, Tailwind CSS and shadcn/ui. Commands between the two are typed end
to end with `specta`/`tauri-specta`.

## Consequences

- Small installers and a Rust core where SQLite, embeddings and network calls live.
- The webview differs per platform (WebView2, WKWebView, WebKitGTK); CI builds on all three.
- `specta`/`tauri-specta` for Tauri v2 are still release candidates; versions are pinned exactly.
- Contributors need Rust and Node toolchains; `CONTRIBUTING.md` and the devcontainer cover both.
