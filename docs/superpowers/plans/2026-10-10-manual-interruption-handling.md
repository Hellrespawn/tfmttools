# Manual interruption handling implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> for inline execution, or superpowers:subagent-driven-development if the user
> selects delegation. Steps use checkbox syntax for tracking.

**Goal:** Replace automatic recovery with incremental confirmed history and
explicit manual resolution, reducing complexity and increasing consistency.

**Architecture:** Core persists confirmed actions and one current attempt;
filesystem modules execute once and supply reporting details. The CLI uses one
attempt-and-confirm loop for apply, undo, and redo. Tag replacement privately
retains its original until durable confirmation; no session infers outcomes or
resumes work.

**Tech Stack:** Rust 1.91.0, edition 2024, rusqlite, rusqlite_migration, clap,
existing binary-patch and CLI fixture infrastructure.

**Spec:** `docs/superpowers/specs/2026-10-10-manual-interruption-handling-design.md`

## Global constraints

- Reduce complexity and increase consistency; replace and delete old recovery.
- No automatic recovery, rollback, filesystem reconciliation, or continuation.
- No generalized filesystem outcome categories in shared execution.
- No durable storage of unattempted actions for a later session.
- Retain exact-byte replay, stale-input checks, verified candidates, and locks.
- Keep `journal_mode=DELETE`, `synchronous=EXTRA`, and `foreign_keys=ON`.
- Never keep a SQLite transaction open across filesystem effects.
- Resolution changes history only; it never inspects or changes affected files.
- Partial runs and failed/interrupted replay are not redo eligible.
- Preserve original tag bytes until completion is durably confirmed.

## Review focus

- An ambiguous completion commit must be reported from committed database state,
  without deleting a potentially necessary backup (Tasks 1 and 3).
- Interrupted copy/delete can leave both paths; resolution must not silently
  perform deletion or claim a compound action completed (Tasks 2 and 4).
- Failure during undo must retain the original action order and applied prefix,
  including staging moves (Tasks 1 and 3).
- Backup cleanup failure must leave its action confirmed and produce an accurate
  leftover-path notice, without cleanup journal state (Tasks 2 and 3).
- Schema creation must be transactional; existing databases with unexpected
  versions or structure must be rejected without writes (Task 1).

## File responsibilities

- Replace `crates/core/src/history/journal.rs` with `attempt.rs`: transactional
  attempt/confirmation/resolution; retain patch validation without recovery data.
- Modify core `model.rs`, `runtime.rs`, `database.rs`, `persistence.rs`, and
  `mod.rs`: partial records, replay cursor, selection, validation, exports.
- Replace `crates/core/src/history/schema-v1.sql` directly: one initial schema
  for confirmed actions, open runs, attempts, and patches. No v2 migration; retain the migration framework.
- Replace fs `recorded_execution.rs` with `prepared_execution.rs`: one-shot
  non-tag execution, preparation ownership, and reporting details.
- Simplify fs `file_switch.rs`: one-shot tag replacement and local backup
  lifecycle. Keep existing patch/tag preparation helpers where useful.
- Rewrite CLI `history/execution.rs`: a single sequential execution loop;
  delete expected-state reconstruction and startup recovery.
- Add CLI `commands/resolve_history.rs` and update `commands/mod.rs`,
  `cli/args_definition.rs`, and `cli/args.rs`: explicit history resolution.
- Update history display/formatting, rename session, validate, undo/redo, and
  clear-history command flows to report/block unresolved attempts uniformly.
- Replace recovery-specific tests; retain byte replay and validation coverage.

### Task 1: Incremental history and initial schema

**Files:** Core history files above; `crates/core/tests/history_journal.rs`
(replace with `history_attempts.rs`), `history_operations.rs`, `history_schema.rs`,
`history_database_validation.rs`, and `tests/support/history.rs`.

**Interfaces:** Core exports `OperationKind::{Apply, Undo, Redo}`,
`RunId(i64)`, `AttemptId(i64)`, `AttemptOutcome::{Applied, NotApplied}`,
`AttemptDetails { paths: Vec<String>, instructions: String }`, and
`CurrentAttempt { id, run_id, action_position, action: StoredAction, details,
patches: Option<BinaryPatchPair> }`. Details describe intent, never executable
recovery steps. Existing IDs for records remain unchanged.

`History` produces:

```rust
fn begin_run(&mut self, kind: OperationKind, record_id: Option<usize>,
    metadata: Option<ActionRecordMetadata>) -> Result<RunId>;
fn begin_attempt(&mut self, run: RunId, action_position: usize,
    action: &StoredAction, details: &AttemptDetails,
    patches: Option<&BinaryPatchPair>) -> Result<AttemptId>;
fn confirm_attempt(&mut self, attempt: AttemptId) -> Result<()>;
fn discard_unexecuted_attempt(&mut self, attempt: AttemptId) -> Result<()>;
fn close_run(&mut self, run: RunId, complete: bool) -> Result<Record>;
fn current_attempt(&self) -> Result<Option<CurrentAttempt>>;
fn resolve_attempt(&mut self, attempt: AttemptId,
    outcome: AttemptOutcome) -> Result<Record>;
fn close_abandoned_run(&mut self) -> Result<Option<Record>>;
```

`Record` adds `applied_count() -> usize`, `is_complete() -> bool`, and
`redo_allowed() -> bool`. Reuse `History::patches` with its existing signature.

- [x] Add failing core tests: confirm two actions then fail a third; only the
  first two are recorded, undo selection includes the partial record, and redo
  eligibility is false. Applied resolution adds exactly the attempted action;
  not-applied resolution does not. Both close the run and clear its marker.
- [x] Add failing replay tests: undo decrements the applied cursor, redo
  increments it; unconfirmed attempts do neither. Resolution respects direction.
  Partial undo selects the same record before earlier records; a new confirmed
  action supersedes redo, but an empty abandoned run does not.
- [x] Add initial-schema/transaction tests: new databases use version 1 and
  reopen read-only; malformed schemas and future versions are rejected without
  writes. Failed confirmation leaves a marker or a committed completion, never
  half of each.
- [x] Run `cargo xtask test-core`; confirm new assertions fail before implementation.
- [x] Implement the interfaces and initial schema. Store one open run, at most
  one attempt, record completeness/redo eligibility, and applied cursor.
  Retain rusqlite_migration for transactional initialization and version tracking,
  with only the initial schema registered. Remove specific v2 migration and
  compatibility scaffolding for the undeployed schema.
- [x] Replace journal exports and validation dependencies. Delete durable plans,
  recovery descriptors, completed/cleaned progress, and old journal mutation
  methods. Preserve payload/patch integrity and stale attempt-ID rejection.
- [x] Run `cargo xtask test-core`; expect all core tests to pass. Later consumer
  compilation is restored by Tasks 2 and 3; no compatibility recovery adapter.
- [x] Commit as `Replace recovery journal with incremental history`.

### Task 2: One-shot filesystem effects and private tag backups

**Files:** Fs files above, `action.rs`, `lib.rs`, `error.rs`,
`tests/file_switch.rs`, and `tests/recorded_execution.rs` (replace with
`tests/prepared_execution.rs`).

**Interfaces:** Fs exports an opaque `PreparedAction` with
`action(&self) -> &StoredAction`, `details(&self) -> &AttemptDetails`,
`patches(&self) -> Option<&BinaryPatchPair>`,
`execute(&mut self) -> FsResult<()>`,
`confirm(&mut self) -> FsResult<()>`, and
`retain_artifacts(&mut self)`. It consumes core reporting types, not history
connections. Keep preparation entry points `prepare_action(&Action,
OperationKind)`, `prepare_tag_edit`, and `prepare_tag_replay`, returning this
fs-owned type with their existing argument shapes.

Preparation artifacts are owned locally until `retain_artifacts` is called
after durable intent. Execute once; do not interpret an already changed target
as proof of successful prior execution. `confirm` only performs local
post-confirmation housekeeping; it cannot restore or replay anything.

- [x] Add failing tests for tag backup retained after execution, deleted only by
  confirmation, and preserved after an injected install error. Instructions and
  artifact paths identify everything needed for manual repair.
- [x] Add failing tests for copy errors preserving the source, partial destination
  reporting, destination verification/synchronization before source deletion,
  collision checks, and staging rename execution without retry logic.
- [x] Add a cleanup-error test: confirmation reports the leftover backup path,
  leaves installed bytes intact, and performs no rollback. Retain exact-byte
  replay, symlink, permissions, hard-link, and stale-input tests.
- [x] Run focused fs tests and confirm new assertions fail before implementation.
- [x] Implement private one-shot tag switching and non-tag execution. Keep
  necessary input validation and durability operations within the owning action.
  Remove retained-original restoration, recovery state matching, generalized
  artifact cleanup, and `recover_prepared` exports. Remove the obsolete recovery
  error name where a normal execution error is sufficient.
- [x] Run `cargo xtask test-fs`; expect all fs tests to pass.
- [x] Commit as `Execute filesystem actions without recovery inference`.

### Task 3: One execution loop and incremental replay

**Files:** `crates/tfmt/src/history/execution.rs`, rename `session.rs`, validate,
undo/redo, clear-history, show-history, and history formatter.

**Interfaces:** Preserve CLI-local `execute_recorded` and `replay_record` call
shapes. They consume Task 1's history methods and Task 2's prepared actions.
Replace `recover_pending` with `check_interrupted(history: &mut History,
fs: &FsHandler) -> Result<()>`: report stored intent, close marker-free abandoned
runs only on writable invocations, and reject file mutation if an attempt exists.
Read-only and dry-run paths report without writing or preparing effects.

- [x] Add failing CLI execution tests: the first error stops later actions;
  history contains confirmed actions; an execution error retains its marker;
  preparation failure does not create one. Successful history confirmation
  precedes backup deletion and starting the next action.
- [x] Add failing tests for completion-commit ambiguity and cleanup failure:
  reopen/read committed history before claiming completion; retain artifacts
  if completion is unconfirmed. Cleanup failure leaves the action confirmed
  and closes the run as partial without an attempted-action marker.
- [x] Add failing mixed-run/replay tests with staging moves: cursor updates
  survive errors, undo affects only the applied prefix, and redo excludes
  partial or failed/interrupted records.
- [x] Run `cargo xtask test-cli`; confirm new behavioral assertions fail.
- [x] Rewrite the loop: prepare, commit attempt, retain artifacts, execute,
  confirm history, perform housekeeping. Close ordinary stopped runs as partial
  where database writes remain possible; leave the durable open run otherwise.
  Replay iterates the remaining applied prefix for undo and the unapplied suffix
  for eligible redo. No filesystem outcome categories or next-session retries.
- [x] Delete `expected_states`, `path_key`, recovery mutation-path logic, and
  all startup recovery calls. Use one interruption formatter across commands.
- [x] Run `cargo xtask test-cli`; expect execution and existing CLI tests to pass
  except replaced recovery expectations, which must be removed in this task.
- [x] Commit as `Unify incremental apply undo and redo execution`.

### Task 4: Explicit history-only manual resolution

**Files:** CLI command/argument files above; replace
`crates/tfmt/tests/history_recovery.rs` with `history_interruption.rs`.

**Interfaces:** Add `tfmt resolve-history --attempt <ID> --outcome
<applied|not-applied>`. Both arguments are required. Dispatch to
`resolve_history(app_options: &TFMTOptions, attempt: i64,
outcome: AttemptOutcome) -> Result<()>`. Reject dry-run resolution with a clear
message; normal dry-run commands remain read-only. No implicit resolution prompt.

- [x] Add failing subprocess tests: explicit applied/not-applied resolution
  changes only history, including when target paths are absent or inaccessible;
  resolving a tag attempt never removes its backup/candidate. Wrong attempt IDs
  fail without writes, and an unresolved compound copy/delete reports both paths.
- [x] Add failing interruption tests at intent, file effect, completion commit,
  and cleanup: reopening never executes, restores, deletes, or hashes audio
  files. Reports show command direction, confirmed action count, attempted action,
  and saved artifact paths. Interruption between actions closes only the run.
- [x] Add CLI tests for blocked mutation/undo/redo/clear-history, read-only
  show-history, dry runs, and successful resolution followed by explicit undo.
- [x] Run `cargo xtask test-cli`; confirm new tests fail before command changes.
- [x] Implement the command and reports using stored data only. Include guidance
  to repair a compound partial result into applied or not-applied state before
  resolution. Never provide a filesystem recovery option.
- [x] Run `cargo xtask test-cli` and `cargo xtask test-integration`; expect PASS.
- [x] Commit as `Add explicit manual history resolution`.

### Task 5: Documentation, deletion review, and final verification

**Files:** README, CHANGELOG, `crates/core/AGENTS.md`, history docs/schema
snapshots, `xtask/src/main.rs`, and relevant CLI fixtures/report expectations.

**Interfaces:** Schema snapshot generation exposes the single initial schema.
Public help documents explicit resolution and the
manual filesystem repair contract.

- [x] Update user documentation with partial-run undo, redo restrictions,
  interruption reports, tag backups, and history-only resolution examples.
  Update crate guidance using writing-for-agents instructions at execution time.
- [x] Run `cargo xtask history-schema`; update versioned snapshots and schema
  tests to compare the current source. Check fixture/report expectations affected by display.
- [x] Search for `recover_pending`, `recover_prepared`, `RecoveryDescriptor`,
  `set_operation_plan`, `complete_cleanup`, and expected-state reconstruction.
  Remove obsolete production code and tests; historical design documents remain.
- [x] Review the resulting diff for one attempt-and-confirm flow, local tag
  backups, no generic recovery replacement, and no stored remaining plan.
- [x] Run `cargo test --workspace`,
  `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
  Expect all commands to pass; fix actionable failures before completion.
- [x] Commit as `Document manual interruption handling and remove recovery remnants`.

## Self-review

The tasks cover each spec section: Task 1 owns storage, partial history, and
compatibility; Task 2 owns action-specific safety; Task 3 owns ordinary errors
and replay; Task 4 owns crash reporting and manual resolution; Task 5 verifies
deletion and documentation. All five review-focus cases have owning tests.
No task adds automatic filesystem repair or requires separate state per possible
filesystem outcome. Shared types and interfaces are defined before consumers.

Tasks 1–3 replace coupled interfaces in order; a workspace build is required
after Task 3. Do not retain the old recovery engine just to keep intermediate
commits workspace-buildable. The proposed CLI spelling and conservative replay
eligibility rule are explicit review points before implementation.

## Execution rulings

- Core, fs, and CLI replacements were committed together after workspace
  verification because their removed interfaces were coupled. Focused crate
  tests ran during the transition; xtask's own dependencies require the CLI to
  compile even for a core-only command.
- Tag preparation artifacts are retained before committing intent, protecting
  ambiguous commits. Intent failures report their paths for manual removal.
- Copies write to a newly created destination directly. Partial copies therefore
  remain at a reported path; no random copy candidate or cleanup protocol is
  needed. Destination verification and synchronization still precede deletion.
- User clarification: the SQLite schema has not been deployed. Collapse it into
  one initial version-1 schema and remove the specific v2 migration and historical
  compatibility tests. Keep rusqlite_migration, its validation test, and version
  tracking for future changes, as clarified by the user. Align the snapshot and
  retain identity/version/structure rejection.
- Independent final review found the unreported-copy-candidate issue. The direct
  destination implementation replaces it; a subprocess file-size-limit test
  observed the failure before the fix and verifies interruption artifacts after.
