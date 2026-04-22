# Agent Support And Token Efficiency Plan

## Goal

Make the repository easier for coding agents to navigate, modify, and verify
without repeatedly reading broad files or inferring project conventions from
source code.

This plan focuses on documentation and repository guidance first. Code
reorganization is listed separately because it has higher review risk.

## Principles

- Keep root guidance short and high-signal.
- Put task-specific guidance close to the files it applies to.
- Prefer maps and examples over prose explanations.
- Document the fastest correct verification command for each change type.
- Avoid asking agents to inspect generated or low-value files unless relevant.

## Phase 1: Root Agent Guidance

Update `AGENTS.md`.

1. Fix stale paths.
   - Replace `crates/cli/src/args.rs` with
     `crates/cli/src/cli/args.rs`.
   - Check all other listed paths against the current tree.

2. Add a compact task map.
   - CLI flags and option conversion:
     `crates/cli/src/cli/args.rs`,
     `crates/cli/src/cli/options.rs`.
   - Rename command flow:
     `crates/cli/src/commands/rename/`.
   - Rename action model:
     `crates/core/src/action/rename_action.rs`.
   - Rename validation:
     `crates/core/src/action/validation.rs`.
   - Filesystem application and staging:
     `crates/fs/src/action.rs`, `crates/fs/src/action/`.
   - History model and persistence:
     `crates/history/src/`.
   - CLI integration fixtures:
     `crates/test-harness/src/`,
     `tests/fixtures/cli/cases/`.

3. Add fast verification guidance.
   - Core-only changes: `cargo test -p tfmttools-core`.
   - Filesystem action/planning changes: `cargo test -p tfmttools-fs`.
   - CLI changes: `cargo test -p tfmttools-cli`.
   - CLI fixture harness changes:
     `cargo test -p tfmttools-cli --test integration -- --nocapture`.
   - Final broad check: `cargo test --workspace`.
   - Lint and format gate:
     `cargo +nightly fmt --all --check` and
     `cargo +nightly clippy --workspace --all-targets`.

4. Add low-value file guidance.
   - Avoid reading `tests/fixtures/cli/report/` unless report output is the
     subject of the task.
   - Avoid reading `tests/fixtures/cli/mvp.css` unless report styling is the
     subject of the task.
   - Treat binary audio fixtures and images as test assets unless the task
     explicitly concerns fixture contents.

## Phase 2: Fixture Documentation

Add `tests/fixtures/cli/README.md`.

Document:

1. Fixture directory roles.
   - `cases/`: JSON test cases discovered by the integration harness.
   - `template/`: templates copied into the test config directory.
   - `audio/`: audio files copied into the input directory.
   - `extra/`: non-audio files copied into the input directory.
   - `test-template.html`: report template.
   - `report/`: generated report output.

2. Case file schema.
   - `description`: human-readable case summary.
   - `expectations`: named path/checksum sets.
   - `tests`: ordered test steps.
   - `command`: CLI command appended after harness-provided global options.
   - `expectations`: expectation set that should exist after the step.
   - `previous-expectations`: expectation set whose paths should no longer
     exist unless marked otherwise.
   - `options: ["no-previous"]`: allow a previous path to remain.

3. Harness behavior.
   - Every case starts from a fresh temporary work directory.
   - Templates, audio fixtures, and extra files are copied before each case.
   - The harness injects `--config-directory` and `--run-id`.
   - Failing work directories are preserved for inspection.
   - `tests/fixtures/cli/report/test-report.html` is generated after a run.

4. Adding a new case.
   - Name cases after behavior, such as
     `case_only_rename.case.json`.
   - Keep matching templates in `template/` when needed.
   - Prefer the smallest fixture that proves the behavior.
   - Include checksums when file identity matters.

## Phase 3: Scoped Agent Files

Add small crate-local `AGENTS.md` files where they reduce root context.

1. `crates/cli/AGENTS.md`.
   - Document the CLI command path:
     `main.rs -> cli::run -> cli/args.rs -> commands::*`.
   - Point option parsing to `cli/args.rs` and option normalization to
     `cli/options.rs`.
   - Point rename behavior to `commands/rename/`.
   - Point integration coverage to `crates/cli/tests/integration.rs` and
     `tests/fixtures/cli/`.

2. `crates/core/AGENTS.md`.
   - Document that this crate owns rename actions, validation, template
     rendering, item keys, and shared UTF-8 path wrappers.
   - Point validation changes to `src/action/validation.rs`.
   - Point rename action model changes to `src/action/rename_action.rs`.
   - Prefer `cargo test -p tfmttools-core` for local verification.

3. `crates/fs/AGENTS.md`.
   - Document that this crate owns filesystem effects, template loading,
     path iteration, checksums, and rename staging.
   - Point staging/planning changes to `src/action/`.
   - Point direct filesystem behavior to `src/fs.rs`.
   - Prefer `cargo test -p tfmttools-fs` for local verification.

4. `crates/test-harness/AGENTS.md`.
   - Point schema readers to `src/data.rs`.
   - Point fixture execution to `src/runner.rs`.
   - Point report output to `src/outcome.rs` and
     `tests/fixtures/cli/test-template.html`.
   - Refer to `tests/fixtures/cli/README.md` for fixture authoring.

## Phase 4: Architecture Map

Add `docs/architecture.md` or `docs/agent-map.md`.

Keep it compact and optimized for lookup.

Include:

1. Workspace map.
   - One sentence per crate.

2. Rename flow.
   - `tfmt` entry point.
   - CLI parsing.
   - session creation.
   - template resolution.
   - file discovery and tag reading.
   - rename action creation.
   - validation.
   - preview or apply.
   - filesystem action execution.
   - history storage.

3. Undo/redo flow.
   - CLI command.
   - history record selection.
   - action execution.
   - record state update.

4. Fixture integration flow.
   - case discovery.
   - temporary directory setup.
   - command execution.
   - expectation verification.
   - report generation.

5. Common change recipes.
   - Add a CLI flag.
   - Add a validation rule.
   - Add filesystem planning behavior.
   - Add an integration fixture.
   - Change history formatting.

## Phase 5: Tooling Convenience

Consider adding a `justfile` or equivalent script wrapper.

Suggested commands:

```just
check:
    cargo check --workspace

test:
    cargo test --workspace

test-core:
    cargo test -p tfmttools-core

test-fs:
    cargo test -p tfmttools-fs

test-cli:
    cargo test -p tfmttools-cli

test-integration:
    cargo test -p tfmttools-cli --test integration -- --nocapture

lint:
    cargo +nightly fmt --all --check
    cargo +nightly clippy --workspace --all-targets
```

Only add this if the project wants another developer dependency. Otherwise,
keep the command list in `AGENTS.md`.

## Phase 6: Optional Code Structure Improvements

These changes are not required for better docs, but would reduce future agent
context needs.

1. Split `crates/core/src/action/validation.rs`.
   - Candidate modules:
     `errors.rs`, `forbidden.rs`, `collisions.rs`, `path_rules.rs`.
   - Keep public validation API stable while moving implementation details.

2. Split `crates/fs/src/action.rs`.
   - Candidate modules:
     `handler.rs`, `executor.rs`, `rename_planner.rs`,
     `rename_cycles.rs`, `rename_staging.rs`.
   - Keep behavior unchanged and move tests with the module they cover.

3. Split large rename command files by workflow phase.
   - Candidate modules:
     `template_resolution.rs`, `discovery.rs`, `planning.rs`,
     `apply.rs`, `finish.rs`, `session.rs`.
   - Keep the external command surface unchanged.

## Validation

For documentation-only phases:

- Run a path check manually with `rg --files` for every documented path.
- Run `cargo test -p tfmttools-cli` only if fixture docs or harness guidance
  changes are paired with harness behavior changes.

For optional code structure phases:

- Run the narrow crate test first.
- Run `cargo test --workspace` before merging.
- Run `cargo +nightly fmt --all --check`.
- Run `cargo +nightly clippy --workspace --all-targets`.

## Suggested Order

1. Update root `AGENTS.md`.
2. Add `tests/fixtures/cli/README.md`.
3. Add crate-local `AGENTS.md` files.
4. Add `docs/agent-map.md` or `docs/architecture.md`.
5. Decide whether to add a `justfile`.
6. Treat code splitting as separate PRs after documentation is in place.
