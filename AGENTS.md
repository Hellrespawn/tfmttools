# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace. The CLI entry point lives in `tfmttools-cli/` and builds the `tfmt` binary from `src/main.rs`. Core rename logic is in `tfmttools-core/`, filesystem helpers are in `tfmttools-fs/`, and history support is split between `tfmttools-history-core/` and `tfmttools-history-serde/`. Shared test utilities and fixture-driven integration data live in `tfmttools-test/`. Example templates are under `examples/`; test fixtures are under `tfmttools-test/testdata/`.

## Build, Test, and Development Commands
Use Cargo from the workspace root.

- `cargo build` builds the default workspace member, `tfmttools-cli`.
- `cargo build --workspace` builds every crate.
- `cargo run -- --help` runs the CLI locally.
- `cargo test --workspace` runs unit and integration tests across the workspace.
- `cargo test -p tfmttools-cli --test integration -- --nocapture` runs the fixture-backed CLI integration suite.
- `cargo +nightly fmt --all` applies the workspace formatting rules.
- `cargo +nightly clippy --workspace --all-targets` checks the shared lint configuration.

## Coding Style & Naming Conventions
Follow `rustfmt.toml`: 4-space indentation, 80-column width, grouped imports, and reordered imports/items. Keep modules focused and use snake_case for files, modules, and functions. Use PascalCase for types and traits. Prefer small, explicit helper functions over dense inline logic. When adding CLI behavior, keep argument parsing in `tfmttools-cli/src/args.rs` or `src/commands/`.

## Testing Guidelines
Tests are a mix of crate-local tests and a custom `libtest-mimic` integration harness. Add new CLI scenarios as `.case.json` files in `tfmttools-test/testdata/cases/`; keep fixture inputs in sibling `audio/`, `extra/`, and `template/` directories. Name test cases after the input or behavior they cover, for example `typical_input_previous_data.case.json`. After changing code, run `cargo test --workspace` and `cargo +nightly clippy --workspace --all-targets`, then fix or justify any lint findings before opening a PR.

## Commit & Pull Request Guidelines
Recent commits use short, imperative subjects such as `Add option to specify template on command line` and `Handle new lints`. Keep commit titles concise, capitalized, and behavior-focused. PRs should explain the user-visible change, call out affected crates, and mention any added fixtures or template changes. Include command output when relevant, especially for test or lint fixes.

## Contributor Notes
The workspace MSRV is Rust 1.85.0 (`edition = "2024"`). If you touch templates or reports, check `examples/` and `tfmttools-test/testdata/test-template.html` so sample output and test reporting stay aligned.
