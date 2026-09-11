---
name: add-rust-feature
description: Rules for adding a feature to a Rust project in this repo. Use when implementing a new feature, opening a feature branch, or wiring app logic before merge.
---

# Add a Rust feature

## Branch

- Create a dedicated branch, e.g. `feature/<short-name>`.
- Do not land unfinished or placeholder code on `main`.
- Open a PR into `main` when the feature is ready.

## Code quality (always)

- Run `cargo fmt` before committing (CI uses `cargo fmt -- --check`).
- Run `cargo clippy -- -D warnings` and fix all findings.
- Run `cargo test` and keep tests green.
- Prefer let-chains / collapsible `if` where Clippy expects them (avoid nested `if` that Clippy flags as `collapsible_if`).

## API and modules

- Match existing module boundaries; extend rather than rewrite when possible.
- If a function signature changes (extra args, new columns), update **all** call sites in the same change.

## Before PR / merge

- [ ] `cargo fmt -- --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`
- [ ] CI green on the PR
- [ ] No debug-only dead code that Clippy or machete would reject

## PR hygiene

- Short title describing the feature.
- Body: what changed, how to test manually, any new config or files on disk.
