# Test Harness Crate Guidance

This crate owns the fixture-backed CLI integration harness used by
`crates/cli/tests/integration.rs`.

## Task Map

- Case schema readers and expectation data: `src/data.rs`.
- Fixture directory and temporary work directory setup: `src/context.rs`.
- Case discovery, command execution, expectation checks, and report
  generation: `src/runner.rs`.
- Serialized command, expectation, and case outcomes: `src/outcome.rs`.
- Report template: `../../tests/fixtures/cli/test-template.html`.
- Fixture authoring guide: `../../tests/fixtures/cli/README.md`.

## Verification

- Harness or fixture behavior changes:
  `cargo test -p tfmttools-cli --test integration -- --nocapture`.
- Crate-local compile check: `cargo test -p tfmttools-test`.
