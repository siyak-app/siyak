<!-- Title: a Conventional Commit, e.g. "feat: add verse bookmarks". It becomes the squash commit message. -->

## What and why

<!-- What does this change, and why is it needed? -->

## Linked issue

<!-- e.g. Closes #123 -->

## How it was tested

<!-- Automated tests added or changed; manual steps you followed. -->

## Screenshots

<!-- For UI changes: before/after, light and dark, Turkish and English if wording changed. -->

## How does this look in the reader (simple) defaults?

<!-- Spec §13.2: what does a casual reader see? Is anything hidden or shown only on request? Write "not user-facing" if it does not apply. -->

## Checklist

- [ ] New user-facing strings are added to both `src/i18n/tr.json` and `src/i18n/en.json`
- [ ] Documentation is updated (README, CONTRIBUTING, ARCHITECTURE, ADR, website) or not needed
- [ ] `cargo xtask bindings` was run if a command or its types changed
- [ ] Local checks pass (see CONTRIBUTING, "Running the checks locally")
