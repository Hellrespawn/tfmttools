# Agent Map

Compact map for finding the right code quickly. Prefer crate-local
`AGENTS.md` files for scoped guidance when working inside a crate.

## Workspace

- `crates/tfmt/`: builds the `tfmt` binary, parses CLI flags, dispatches
  commands, owns terminal UI, and coordinates rename/undo/history flows.
- `crates/core/`: owns domain models, persistence, and rules: rename actions, validation,
  templates, audio metadata, item keys, history, and UTF-8 path wrappers.
- `crates/fs/`: owns filesystem effects and helpers: action execution,
  rename staging/planning, checksums, template loading, path iteration,
  and direct filesystem operations.
- `crates/core/src/history/`: owns validated SQLite storage, explicit record
  progress, borrowed undo/redo selection, patches, and history-only resolution.
- `crates/test-harness/`: owns the fixture-backed CLI integration test
  runner used by `crates/tfmt/tests/integration.rs`.
- `xtask/`: owns repository task shortcuts exposed through
  `cargo xtask`.

## Rename Flow

1. `tfmt` starts in `crates/tfmt/src/main.rs` and calls
   `tfmt::cli::run`.
2. `crates/tfmt/src/cli/mod.rs` initializes tracing, parses
   `TFMTArgs`, converts them to `TFMTOptions`, creates `FsHandler`, and
   dispatches the selected subcommand.
3. `crates/tfmt/src/cli/args.rs` defines global flags and the `rename`
   subcommand; `crates/tfmt/src/cli/options.rs` normalizes those args into
   `TFMTOptions` and `RenameOptions`.
4. `crates/tfmt/src/commands/rename/session.rs` creates a
   `RenameSession`, converts rename args, loads history, and coordinates
   setup, apply, and finish.
5. `crates/tfmt/src/commands/rename/setup.rs` resolves the template from
   the command line or previous history. It uses
   `crates/fs/src/template.rs` to load template files or scripts.
6. `setup.rs` gathers paths with `crates/fs/src/path_iterator.rs`,
   filters supported audio extensions, and reads tags through
   `crates/core/src/audiofile.rs`.
7. Audio tags are rendered through `crates/core/src/templates/` to build
   `RenameAction` values from
   `crates/core/src/action/rename_action.rs`.
8. `crates/tfmt/src/commands/rename/apply.rs` separates unchanged
   destinations, validates rename actions with
   `crates/core/src/action/validation.rs`, and prints the preview.
9. If confirmed, `apply.rs` sends actions to
   `crates/fs/src/action.rs`. The filesystem crate plans directory
   creation, detects cycles in `crates/fs/src/action/rename_cycles.rs`,
   and stages conflicting renames with
   `crates/fs/src/action/rename_staging.rs`.
10. `crates/tfmt/src/commands/rename/finish.rs` plans remaining-file and
    directory cleanup before effects.
11. `crates/tfmt/src/history/execution.rs` coordinates prepared actions and
    incremental SQLite history. Core `history/attempt.rs` owns transactions;
    `persistence.rs` opens and validates sessions and `patch.rs` stores patches.

## Undo/Redo Flow

1. `undo` and `redo` are parsed in `crates/tfmt/src/cli/args.rs` and
   dispatched from `TFMTSubcommand::run`.
2. `crates/tfmt/src/commands/undo_redo.rs` loads history from the
   configured history file.
3. Record selection happens in `crates/core/src/history/runtime.rs` with
   `records_for(mode, limit)`, using shared replay eligibility predicates.
4. The CLI previews selected records with
   `crates/tfmt/src/history/formatter.rs` and asks for confirmation unless
   confirmation is disabled.
5. `crates/tfmt/src/history/execution.rs` uses typed replay context and the
   same prepare/intent/execute/confirm/housekeeping loop as apply. Undo proceeds
   in reverse action order; redo proceeds forward from the applied cursor.
6. Each action confirmation updates progress and clears intent transactionally
   in `history/attempt.rs`. Affected cache records publish after commit. Errors
   require reading committed state; interrupted actions need manual resolution.

## Fixture Integration Flow

1. `crates/tfmt/tests/integration.rs` calls
   `tfmttools_test_cli::test_runner`.
2. `crates/test-cli/src/runner.rs` discovers
   `tests/fixtures/cli/cases/*.case.json` through
   `crates/fs/src/path_iterator.rs`.
3. `crates/test-harness/src/data.rs` reads the case schema:
   `description`, named `expectations`, ordered `tests`, commands, and
   previous-expectation checks.
4. `crates/test-cli/src/case.rs` creates a fresh temporary work
   directory per case and maps fixture sources under
   `tests/fixtures/cli/`.
5. Before each case, `runner.rs` copies `template/*` to `config/`,
   `audio/*` to `input/`, and `extra/*` to `input/extra/`.
6. Each command runs the `tfmt` binary in the temporary work directory
   with injected `--config-directory` and `--run-id` arguments, then
   appends the command from the case file.
7. Expectations verify file presence, optional checksums, and removal of
   `previous-expectations` unless an entry has
   `options: ["no-previous"]`.
8. Failing work directories are preserved. After the run,
   `runner.rs` writes runner-specific timestamped report files under
   `tests/reports/`.

## Change Recipes

### Add A CLI Flag

1. Add the flag to `crates/tfmt/src/cli/args.rs`.
2. Convert it in `crates/tfmt/src/cli/options.rs` if it affects shared
   options or rename options.
3. Use the normalized option in `crates/tfmt/src/commands/`.
4. Verify with `cargo xtask test-cli`; add a fixture case when the
   behavior is command-visible.

### Add A Validation Rule

1. Add the rule and tests in `crates/core/src/action/validation.rs`.
2. Make sure `validate_rename_actions` includes the new rule in the
   intended order.
3. If the user-facing error changes, check the CLI path through
   `crates/tfmt/src/commands/rename/apply.rs`.
4. Verify with `cargo xtask test-core`; run `cargo xtask test-cli` when
   CLI behavior is affected.

### Add Filesystem Planning Behavior

1. Start in `crates/fs/src/action.rs` for the executor/handler surface.
2. Put rename ordering logic in `crates/fs/src/action/rename_planner.rs`.
3. Put temporary-path behavior in
   `crates/fs/src/action/rename_staging.rs`.
4. Put cycle detection changes in
   `crates/fs/src/action/rename_cycles.rs`.
5. Verify with `cargo xtask test-fs`; run the CLI integration
   suite if fixture behavior changes.

### Add An Integration Fixture

1. Read `tests/fixtures/cli/README.md`.
2. Add a `*.case.json` file under `tests/fixtures/cli/cases/`.
3. Add templates under `tests/fixtures/cli/template/` only when needed.
4. Prefer existing assets under `audio/` and `extra/`; add checksums when
   file identity matters.
5. Verify with `cargo xtask test-integration`.

### Change History Formatting

1. Update CLI formatting in `crates/tfmt/src/history/formatter.rs`.
2. Check callers in `crates/tfmt/src/commands/show_history.rs` and
   `crates/tfmt/src/commands/undo_redo.rs`.
3. If record data or state changes, update
   `crates/core/src/history/model.rs` and `crates/core/src/history/runtime.rs`.
4. Verify with `cargo xtask test-cli`.

## History format changes

History uses the concrete types in `crates/core/src/history/`. Record actions
and metadata retain strict JSON payloads inside SQLite. Records have explicit
applied count, completeness, and redo eligibility; IDs define chronological
order. Legacy JSON databases are rejected without import. The initial SQLite
schema is undeployed: amend version 1 directly and reject earlier development
schema shapes without rewriting them.

Read `docs/superpowers/specs/2026-10-10-manual-interruption-handling-design.md`
and `docs/superpowers/specs/2026-10-10-history-simplification-design.md` for the
current contracts. Regenerate `docs/history/schema-v1.sql` with
`cargo xtask history-schema`. Run core history tests and CLI compatibility and
interruption tests. Published format changes require explicit compatibility
handling. Never infer filesystem outcomes or automatically resume operations.
