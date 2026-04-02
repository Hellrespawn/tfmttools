# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace. The CLI entry point lives in
`crates/cli/` and builds the `tfmt` binary from `src/main.rs`. Core
rename logic is in `crates/core/`, filesystem helpers are in
`crates/fs/`, and history support is in `crates/history/`. Shared test
utilities for the integration harness live in `crates/test-harness/`.
Workspace-level integration fixtures and sample report assets live under
`tests/fixtures/cli/`. Example templates are under `examples/`, and
notes or design work live under `docs/`.

## Build, Test, and Development Commands

Use Cargo from the workspace root.

- `cargo build` builds the default workspace member, `tfmttools-cli`.
- `cargo build --workspace` builds every crate.
- `cargo run -- --help` runs the CLI locally.
- `cargo test --workspace` runs unit and integration tests across the workspace.
- `cargo test -p tfmttools-cli --test integration -- --nocapture` runs the fixture-backed CLI integration suite.
- `cargo +nightly fmt --all` applies the workspace formatting rules.
- `cargo +nightly clippy --workspace --all-targets` checks the shared lint configuration.
- `cargo test -p tfmttools-cli` is the quickest way to exercise the CLI crate, including its custom integration harness.

## Coding Style & Naming Conventions

Follow `rustfmt.toml`: 4-space indentation, 80-column width, grouped
imports, and reordered imports/items. Keep modules focused and use
snake_case for files, modules, and functions. Use PascalCase for types
and traits. Prefer small, explicit helper functions over dense inline
logic. When adding CLI behavior, keep argument parsing in
`crates/cli/src/args.rs` or `crates/cli/src/commands/`.

## Testing Guidelines

Tests are a mix of crate-local tests and a custom `libtest-mimic`
integration harness in the CLI crate. Keep fixture-backed integration
inputs under `tests/fixtures/cli/`; if you add new scenario structure,
mirror the conventions already used there so the harness can discover
them consistently. Name test cases after the behavior they cover and keep
sample assets close to the fixture that uses them. After changing code,
run `cargo test --workspace` and `cargo +nightly clippy --workspace
--all-targets` as part of your normal workflow. Treat `clippy` output as
actionable: fix reported issues, or document why a finding is being left
in place, before opening a PR.

## Commit & Pull Request Guidelines

Recent commits use short, imperative subjects such as `Add option to specify template on command line` and `Handle new lints`. Keep commit titles concise, capitalized, and behavior-focused. PRs should explain the user-visible change, call out affected crates, and mention any added fixtures or template changes. Include command output when relevant, especially for test or lint fixes.

## Contributor Notes

The workspace MSRV is Rust 1.85.0 (`edition = "2024"`). If you touch
templates or reports, check `examples/` and
`tests/fixtures/cli/test-template.html` so sample output and test
reporting stay aligned. The workspace metadata also watches that fixture
template for tooling integrations.
