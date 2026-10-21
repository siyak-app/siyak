# 0010. Automate repository tasks with a Rust xtask

- Status: Accepted
- Date: 2026-10-21

## Context

Two tasks need to run outside the app: building the development seed database
(`dev-data/dev-content.db`) and exporting the TypeScript bindings for CI. A Node script would
need a native SQLite module and would duplicate the schema.

## Decision

Repository automation lives in the `xtask` crate, run as `cargo xtask <task>`. It depends on the
app library, so the development database is built with the app's own migrations and the bindings
with the app's own command list.

## Consequences

- The seed schema cannot drift from the migrations; there is no second SQLite dependency.
- Running a task compiles the app library once (shared with normal builds).
