# CLI Crate Guidance

This crate builds the `tfmt` binary. The command path starts at
`src/main.rs`, enters `cli::run` in `src/cli/mod.rs`, parses arguments in
`src/cli/args.rs`, normalizes options in `src/cli/options.rs`, then
dispatches through `src/commands/`.

## Task Map

- CLI argument parsing: `src/cli/args.rs`.
- Option conversion and defaults: `src/cli/options.rs`.
- Rename command behavior: `src/commands/rename/`.
- History display formatting: `src/history/formatter.rs`.
- Terminal UI helpers: `src/ui/`.
- Integration entry point: `tests/integration.rs`.
- Fixture-backed cases and assets: `../../tests/fixtures/cli/`.

## Verification

- CLI-only changes: `cargo test -p tfmttools-cli`.
- Fixture harness changes:
  `cargo test -p tfmttools-cli --test integration -- --nocapture`.
- Broader workspace confidence: `cargo test --workspace`.
