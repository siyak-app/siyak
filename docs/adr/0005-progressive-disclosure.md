# 0005. Use progressive disclosure instead of separate modes

- Status: Accepted
- Date: 2026-10-21

## Context

Siyak serves casual readers and researchers. Separate "basic" and "expert" modes would double
the screens to build and test, and would hide from readers that deeper features exist
(spec §1, §13.2).

## Decision

One interface. Screens open simple and reveal depth on request. A single first-launch question
("read and understand" or "do research") stores an experience profile that only sets defaults;
every default can be changed individually.

## Consequences

- One set of components; each feature must decide how it looks in the reader (simple) defaults.
  The pull request template asks this for every change.
- Technical terms have a plain and a technical wording in i18n files
  (`term.<name>.plain` / `term.<name>.technical`).
