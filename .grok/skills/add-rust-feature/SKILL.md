---
name: add-rust-feature
description: Rules for adding a feature to a Rust project in this repo. Use when implementing a new feature, opening a feature branch, or wiring app logic before merge.
---

# Add a Rust feature

## Branch

- Create a dedicated branch, e.g. `feature/<short-name>`.
- Do not land unfinished or placeholder code on `main`.
- Open a PR into `main` when the feature is ready.

## Code quality

Quality gates live in git hooks and CI. Do not invent a third checklist.

Local hooks (once per clone: `./scripts/install-git-hooks.sh`):

- **pre-commit** — `cargo fmt --all` (restages formatted, already-staged `.rs` files)
- **pre-push** — `cargo clippy -- -D warnings` then `cargo test`

Do not use `--no-verify` on feature branches unless the hook cannot run (no Rust toolchain).

CI on the PR must stay green: rustfmt check, clippy `-D warnings`, `cargo test`, cargo-machete.

When writing code (hooks may not run in this environment):

- Match rustfmt; prefer let-chains / collapsible `if` (Clippy `collapsible_if`).
- No debug-only dead code that Clippy or machete would reject.
- Keep tests green; add tests next to the behavior you change.

## API and modules

- Match existing module boundaries; extend rather than rewrite when possible.
- If a function signature changes (extra args, new columns), update **all** call sites in the same change.

## Before PR / merge

- [ ] Feature branch, not `main`
- [ ] Hooks path installed locally when committing/pushing from a machine with Cargo
- [ ] CI green on the PR
- [ ] No unused deps or debug-only dead code

## PR hygiene

- Short title describing the feature.
- Body: what changed, how to test manually, any new config or files on disk.
