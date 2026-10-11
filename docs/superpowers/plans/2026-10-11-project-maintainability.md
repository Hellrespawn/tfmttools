# Project Maintainability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This plan assumes inline execution; delegation requires the user's choice.

**Goal:** Resolve all eight approved review findings while retaining the existing history and filesystem safety contracts.

**Architecture:** Replace direct filesystem execution with a read-only planner feeding the existing prepared-action and recorded-execution path. Share audio discovery and encoding vocabulary within existing crates, and simplify CLI status handling and Cargo test orchestration.

**Tech Stack:** Rust workspace, Cargo/xtask, color-eyre, Lofty, SQLite, existing custom CLI fixture harness.

**Spec:** [Approved maintainability design](../specs/2026-10-10-project-maintainability-design.md).

## Global Constraints

- Preserve Rust 1.91.0, edition 2024, the existing crate responsibilities, and the dependency set.
- Preserve CLI flags, template semantics, action ordering, cleanup selection, confirmation policy, history serialization, SQLite schema, and exact-byte tag replay.
- Preserve prepare → retain artifacts → commit intent → execute → confirm history → confirm housekeeping.
- Do not introduce a new executor framework, recovery engine, generic repository, plugin system, dependency, persistent field, or schema migration.
- Keep filesystem outcome inference and automatic recovery out of core.
- Preserve copy verification, synchronization, collisions, stale-input checks, unresolved-attempt blocking, and commit/cache invalidation contracts.
- Update README.md, CHANGELOG.md, relevant fixture expectations, and affected task maps alongside each observable change.
- Do not rewrite template parsing, confirmation policy, or snapshot infrastructure.

## Review Focus

- A forced-copy cycle must preserve both originals until the prepared path has verified the relevant copies; test in Task 2.
- A planned directory removal may be a no-op because an unselected file remains; output must not assert removal; test in Task 3.
- A traversal failure after one valid audio entry must stop rename and fix without file effects; test in Task 4.
- Validation findings must exit 1 without an operational-error banner while owned guards drop; test in Task 5.
- Noncanonical stored encoding spellings must still fail loading despite the shared type; test in Task 6.

## Delivery and file boundaries

Use six sequential changes, each independently buildable and reviewable. These
are separate review units within one approved scope; separate specs are not
needed because the existing design already defines their shared contracts.

1. `xtask/src/main.rs`: automatic Cargo test discovery and direct command construction.
2. `crates/fs/src/action/planning.rs`: planner entry point, replacing `executor.rs`; delete `handler.rs`. Update exports, consumers, and behavior tests.
3. `crates/tfmt/src/commands/rename/confirmation.rs` and `cleanup.rs`: rename existing planning modules; `session.rs` owns successful-run reporting.
4. `crates/tfmt/src/commands/audio_discovery.rs`: common audio discovery; existing filesystem and fixture consumers propagate traversal errors.
5. `crates/tfmt/src/commands/mod.rs`, `cli/args.rs`, `cli/mod.rs`, `main.rs`, and `commands/validate.rs`: command outcomes, normal return, and required history references.
6. `crates/core/src/tag_encoding.rs`: encoding vocabulary; existing consumers, history serde declarations, and repository guidance use the simplified definitions.

Each task includes its own documentation and checks. Do not commit unless the
user requests commits; checkpoints below describe reviewable changes, not an
instruction to alter branch history.

### Task 1: Discover all test targets through Cargo

**Files:** `xtask/src/main.rs`, `README.md`, `CHANGELOG.md`.

**Interfaces:** Keep existing task names and focused integration argument forwarding. `test` runs `cargo test --workspace -- --nocapture`; `test-cli` runs `cargo test -p tfmt -- --nocapture`.

- [ ] Run `cargo test -p tfmt -- --list` and inspect the custom harness's argument support before changing orchestration. Confirm the ordinary `confirmation` target exists.
- [ ] Replace broad-task `run_steps` branches with the two commands above. Delete `TEST_WORKSPACE_ARGS`, `TEST_HISTORY_COMPAT_ARGS`, and `TEST_CLI_BIN_ARGS`. Keep `TEST_INTEGRATION_ARGS`, focused tasks, and lint sequencing.
- [ ] Fold `run_cargo_with_env` into `run_cargo(args: &[&str]) -> ExitCode`: construct `Command::new("cargo")`, add arguments, and call existing `run_command`. Remove both environment loops and the wrapper.
- [ ] Update `HELP` and development documentation to describe automatic target discovery.
- [ ] Run `cargo xtask test-cli` and `cargo xtask test`. Save evidence that both run `tests/confirmation.rs` and the custom `integration` target, plus applicable unit/doc tests. Verify focused argument forwarding with `cargo xtask test-integration --list`.

Do not add tests mirroring argument constants; Cargo's actual target execution
is the acceptance check. Subsequent conventional integration targets in this
plan must run without further xtask edits.

### Task 2: Replace the competing filesystem executor with a planner

**Files:** Move `crates/fs/src/action/executor.rs` to `action/planning.rs`; delete `action/handler.rs`; modify `action.rs`, `lib.rs`, `fs_handler.rs`, `crates/fs/tests/prepared_execution.rs`, rename `apply.rs`/`finish.rs` consumers, and `crates/fs/AGENTS.md`. Update `CHANGELOG.md` for public API removal.

**Interfaces:** Export `pub fn plan_actions(rename_actions: Vec<RenameAction>, move_mode: MoveMode) -> FsResult<Vec<Action>>` from filesystem crate root. Retain existing prepared-action interfaces and recorded CLI execution interfaces unchanged.

- [ ] Extend existing planner behavior coverage before deletion. For `stages_two_file_swap`, `stages_three_file_cycle`, `stages_chain_into_freed_target`, `skips_make_dir_for_directory_that_already_exists`, and `stages_case_only_rename`, assert planning leaves directory entries and file bytes unchanged. Preserve platform-specific handling for case sensitivity.
- [ ] Add `forced_copy_cycle_preserves_contents` using the existing temp-file helpers: plan a swap with `AlwaysCopy`, assert copy/remove ordering for each staged move, execute through prepare/execute/confirm, then assert each destination contains the other original. Extend existing copy failure/collision coverage rather than duplicating it.
- [ ] Run `cargo test -p tfmttools-fs` to establish the extended tests' baseline. Tests for the new free function will fail to compile until extraction; behavioral assertions against the existing planner must pass first.
- [ ] Extract the instance planner into `action/planning.rs` with the exact public signature above. Keep `RenamePlanner`, staging, directory actions, temporary-path selection, device detection, and copy/remove order unchanged. No `FsHandler` argument or mutation in planning.
- [ ] Change CLI planning callers and filesystem tests to the free function. Replace direct handler execution in tests with prepared actions and explicit housekeeping confirmation. Move history undo/redo assertions into recorded CLI tests where they require persisted history; do not label simulated filesystem reversal as a history test.
- [ ] Delete `ActionExecutor`, `ActionHandler`, direct application/removal methods, and exports. Search all former handler-only `FsHandler` methods/result types; remove only those with no remaining caller. Retain public helpers with workspace consumers.
- [ ] Run `cargo test -p tfmttools-fs` and `cargo test -p tfmt`. Existing `history_interruption`, copy verification, stale-input, collision, and exact-patch tests must pass. Exercise cross-device behavior when a suitable second device is available; otherwise record that platform limitation.
- [ ] Update filesystem task maps and changelog. Check `rg 'ActionExecutor|ActionHandler|apply_rename_actions' crates` has no obsolete references.

### Task 3: Make rename planning names and output truthful

**Files:** Rename `commands/rename/apply.rs` → `confirmation.rs`, `finish.rs` → `cleanup.rs`; update `mod.rs`, `session.rs`, and call sites. Extend `crates/tfmt/tests/confirmation.rs` and `history_interruption.rs`; update relevant `tests/fixtures/cli/cases/` expectations, README, and changelog.

**Interfaces:** `confirm_and_plan(session: &RenameSession, plan: RenamePlan) -> Result<RenameDecision>`. Define `RenameDecision::{Confirmed { actions, unchanged_files, metadata }, NothingToRename, Aborted}` using the existing field types. Cleanup helper parameter/return types remain unchanged; names become `append_cleanup_actions`, `plan_cleanup_actions`, and `plan_file_moves`.

- [ ] Add subprocess assertions for accepted cleanup, declined cleanup, and dry-run cleanup. Accepted writable cleanup must report completion only after execution; declined cleanup still performs the primary rename; dry run preserves files/history and has no execution/saved-run success message.
- [ ] Add `nonempty_directory_cleanup_does_not_claim_removal`: retain an unselected file in a candidate directory, assert the directory remains and output does not contain `Removed empty folders`.
- [ ] Extend the existing deterministic execution-failure scenario with cleanup planned: assert no `Run completed.` or saved-run message, and retain the existing confirmed-prefix/interruption assertions. Run `cargo test -p tfmt --test confirmation` and the affected interruption test to observe the expected output failures.
- [ ] Rename modules, functions, enum, variants, and uses exactly as above. Remove the ignored no-work payload. Keep existing confirmation and action ordering.
- [ ] Replace premature cleanup success wording with prospective wording (`Files to delete` for writable previews, `Would delete` for dry runs). After `execute_recorded(...)` returns successfully, writable runs print `Run completed.` followed by the existing saved-run message. Propagate execution/housekeeping errors before either success message. Do not count directory actions as actual removals.
- [ ] Preserve explicit cleanup-skip diagnostics when protected/remaining paths cannot be resolved, and preserve primary execution after declined cleanup. Add assertions to the existing cleanup-policy test or the module's focused tests.
- [ ] Run `cargo test -p tfmt`, including changed fixture expectations. Update docs with actual final wording; do not weaken byte/file/history assertions while updating output.

### Task 4: Share fallible discovery and stop incomplete scans

**Files:** Create `commands/audio_discovery.rs`; modify `commands/mod.rs`, `rename/discovery.rs`, `validate.rs`, `crates/fs/src/path_iterator.rs`, `fs_handler.rs`, `template.rs`, `crates/test-cli/src/runner.rs`, and affected consumers found by search. Add CLI coverage in `crates/tfmt/tests/discovery.rs`; update fixture documentation, README, and changelog.

**Interfaces:** `pub(crate) fn gather_audio_paths(app_options: &TFMTOptions, options: &PathIteratorOptions<'_>) -> Result<Vec<Utf8PathBuf>>`. Add `pub fn input_directory(&self) -> &Utf8Path` to `PathIteratorOptions` for error context. Internal collection accepts `impl Iterator<Item = FsResult<Utf8PathBuf>>` to allow deterministic test injection, without adding a production policy framework.

- [ ] In the new discovery module, test `scan_error_discards_partial_results` with an iterator containing `Ok(audio_path)` then `Err(...)`: assert the result is an error containing the input directory. Test successful ordering, extension matching, and counter behavior using the existing predicates/UI API.
- [ ] Add subprocess `incomplete_scan_does_not_rename_or_fix` coverage. On Unix, create a valid first audio entry followed by a filename containing invalid UTF-8 bytes; assert nonzero status and unchanged file bytes/names and no recorded action effects for both rename and validation fix. This deterministically exercises the real `PathIterator` error without permission assumptions. Keep injected collector coverage portable; explicitly record Unix-only coverage.
- [ ] Run the new tests and confirm old discovery silently truncates the subprocess scan. Implement common spinner/counters/filter/debug-delay logic with fallible iteration. Finish on success and clear/finish UI on failure without reporting successful gathering. Update both callers to `?` before reading/planning/applying files.
- [ ] Make fixture `load_data` propagate traversal errors with case-directory context. Add runner unit tests for a synthetic discovery error and stable sorted names; retain the distinct empty-case-directory error. Use a local iterator helper where necessary, without exposing a generic public discovery API.
- [ ] Audit every `PathIterator` consumer with `rg`. Make directory discovery used for mutation return `FsResult<Vec<_>>` and propagate through callers; template enumeration already handling results must remain fallible. For rename cleanup, preserve the explicit diagnostic-and-skip policy when this propagated error reaches cleanup. Document any retained best-effort reporting consumer in code.
- [ ] Run `cargo test -p tfmttools-fs`, `cargo test -p tfmttools-test-cli`, and `cargo test -p tfmt`. Check recursion depth, ignored files, stable order, and CLI fixture selection remain unchanged. Update fixture authoring guidance to explain discovery errors abort selection.

### Task 5: Return command outcomes and require validation history

**Files:** `commands/mod.rs`, `commands/validate.rs`, `cli/args.rs`, `cli/mod.rs`, `main.rs`; create `crates/tfmt/tests/exit_status.rs`; update README and changelog.

**Interfaces:** `pub(crate) enum CommandOutcome { Success, ValidationFailed }` in `commands/mod.rs`. Dispatch `run(...) -> Result<CommandOutcome>`; validation returns the same result type; other handlers keep `Result<()>` and dispatch maps success. `cli::run() -> Result<std::process::ExitCode>`; `main() -> color_eyre::Result<std::process::ExitCode>`. Fix helpers and `apply_and_store_fix` take `&mut History` in the existing history argument position.

- [ ] Add subprocess assertions for successful command status 0, validation findings status 1 with the existing report and no extra operational error, and operational failure status 1 with the existing command-specific formatter. Reuse small audio fixtures; avoid generated report assets.
- [ ] Add a unit check around the result-to-exit mapping with an owned drop sentinel: scope exit must set the sentinel on success, findings, and operational error. Ensure the mapping tested is called by `cli::run`, not a test-only imitation. Keep tracing guard owned by `run` until return; inspect that no normal branch bypasses it.
- [ ] Run baseline exit tests, then change dispatch and validation `check` to return outcomes. Change `cli::run` to map normal operational errors after rendering; debug builds propagate rich errors with `?`. Map `ValidationFailed` to `ExitCode::FAILURE` in both configurations. Update `main` to delegate unchanged.
- [ ] Restructure validation into check versus fix branches. Only the fix branch loads/checks history, including the existing dry-run load mode; pass `&mut History` to both fix helpers and the shared recorder. Remove optional-history plumbing and its `expect`. Preserve rejection of bare `validate --fix`.
- [ ] Verify dry-run fixes leave history and files unchanged; no-change fixes retain existing reporting. Check-only validation must succeed without acquiring a writable history session. Extend an existing locked-history/check scenario or add it to `exit_status.rs`.
- [ ] Run `cargo test -p tfmt` and `cargo test -p tfmt --features debug`. Inspect `rg 'process::exit' crates/tfmt/src`: remaining exits must be existing signal handling, not command completion. Preserve Clap and panic/signal behavior.

### Task 6: Centralize encoding names and remove mechanical leftovers

**Files:** Create `crates/core/src/tag_encoding.rs`; modify core `lib.rs`, `history/conversion.rs`, `history/stored.rs`, `history/model.rs`; filesystem `action/tag_edit.rs`; CLI `commands/validate.rs`; fixture runner `crates/test-cli/src/runner.rs`; root and crate AGENTS task maps, README, changelog.

**Interfaces:** `pub enum TagTextEncoding { Latin1, UTF16, UTF16BE, UTF8 }` with `pub fn parse(name: &str) -> Option<Self>` and `pub fn name(self) -> &'static str`; implement conversions `From<TagTextEncoding> for lofty::TextEncoding` and `TryFrom<lofty::TextEncoding> for TagTextEncoding`. Use `type Error = lofty::TextEncoding` to preserve an unsupported value rather than default it. Export `pub mod tag_encoding` from core. Stored optional string fields remain strings.

- [ ] Add table-driven core tests with exact pairs `Latin1`, `UTF16`, `UTF16BE`, `UTF8`; assert name, parse, and Lofty round trips. Assert `latin1`, `utf8`, `UTF-8`, empty string, and unknown names return `None`. Run the targeted module tests and observe missing-module/interface failure.
- [ ] Implement the type and conversions. Inspect the pinned Lofty enum: handle its current four values exhaustively; if additional values exist, return `Err(original)` and preserve each caller's existing rejection/reporting behavior. Do not introduce unknown-to-UTF8 defaults.
- [ ] Replace string/Lofty mappings in stored-history validation, tag editing, CLI validation, and fixture reporting. Keep canonical names unchanged, including the independent raw ID3 frame reader and its byte-level tests.
- [ ] Extend `crates/core/tests/history_contract.rs` or existing stored-payload validation coverage with a noncanonical encoding string and assert load rejection. Run existing history contract, database validation, native tag verification, and exact patch replay coverage.
- [ ] Remove redundant same-name serde field renames while retaining `default`, strictness, tags, and real exceptions. Replace explicit enum variant renames with `rename_all = "snake_case"` only where all names are identical. Compare serialized contract fixtures and SQL snapshot without regeneration; any difference blocks this cleanup.
- [ ] Fix root guidance to `docs/history/schema-v1.sql`; filesystem guidance to `src/fs_handler.rs`; describe planner/prepared-action entry points and renamed CLI modules in task maps. Do not rewrite unrelated guidance.
- [ ] Run `cargo test -p tfmttools-core`, `cargo test -p tfmttools-fs`, `cargo test -p tfmttools-test-cli`, and `cargo test -p tfmt`. Inspect that repeated encoding maps and optional-history panic are gone, and executable/stored action models remain separate.

### Final verification and review

- [ ] Format with `cargo +nightly fmt --all`, then run `cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`. Fix actionable failures; report unavailable platform/toolchain checks accurately.
- [ ] Run `cargo xtask test-cli` and `cargo xtask test` again after new integration targets exist. Verify automatic inclusion of `confirmation`, `discovery`, `exit_status`, and the custom harness. Run focused changed fixtures through `cargo xtask test-integration`.
- [ ] Review `git diff --check` and the complete diff against every approved spec section. Identify the deleted direct executor, duplicated discovery pipeline, manual test list, optional-history panic, and repeated encoding mappings. Confirm no dependency/schema/persistent-state changes, recovery behavior, or weakened durability/assertions.
- [ ] Report final changes, commands and results, removed APIs, intentional output/error behavior, and any platform limitations. Leave implementation ready for review; do not merge, publish, or commit without the corresponding user instruction.
