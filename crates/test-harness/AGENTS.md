# Test Harness Crate Guidance

This crate owns shared fixture parsing and report utilities for test harness
crates.

## Task Map

- Case schema readers and expectation data: `src/data.rs`.
- Fixture directory helpers: `src/context.rs`.
- Shared serialized command, expectation, and report outcomes:
  `src/outcome.rs`.
- Static report viewer and writer: `src/report.rs`, `assets/report/`.
- Fixture authoring guide: `../../tests/fixtures/cli/README.md`.

## Verification

- Shared harness or fixture behavior changes:
  `cargo test -p tfmttools-cli --test integration -- --nocapture`.
- Crate-local compile check: `cargo test -p tfmttools-test-harness`.
