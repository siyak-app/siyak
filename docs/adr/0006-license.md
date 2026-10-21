# 0006. License the code as MIT OR Apache-2.0; data keeps its own licenses

- Status: Accepted
- Date: 2026-10-21

## Context

The project should be easy to reuse and contribute to, and follow the Rust ecosystem's
convention. The data it uses comes from many sources with different terms (spec §2, §6.5).

## Decision

The code is dual-licensed `MIT OR Apache-2.0`, copyright "Kemal Özyön and Siyak contributors".
Contributions are accepted under the same terms. Data packs are distributed separately and keep
their own licenses, shown to the user before download and on the Sources screen.

## Consequences

- Restrictive data terms (e.g. the QAC morphology terms) never affect the code license.
- Every bundled or downloadable dataset must carry its license and required attribution.
