# History Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address all seven SQLite history review findings while preserving manual interruption handling and exact replay.

**Architecture:** Retain the existing record cache and shared CLI execution loop. Make progress and operation inputs explicit, share validation, consolidate patch persistence, and publish affected-record updates only after successful transactions.

**Tech Stack:** Rust workspace, rusqlite, serde, existing Cargo xtasks.

**Spec:** [Approved design](../specs/2026-10-10-history-simplification-design.md). Read it before executing this plan.

## Global Constraints

- Keep Rust 1.91.0 and edition 2024. Add no dependencies.
- Preserve CLI flags, record ordering, displayed history, action payloads, patch format, and exact byte replay.
- The SQLite schema remains undeployed. Amend the initial version-1 schema and its snapshot directly.
- Do not add a migration or import path for those databases.
- Manual resolution remains history-only.
- At most one run and one current attempt exist. No subsequent action starts before confirmation.
- Keep session locks, validation, artifact retention, and the prepare/intent/execute/confirm/housekeeping sequence.

## Review Focus

- A failed or uncertain commit must not expose stale cached records; Task 6 tests commit and reload failures.
- Manual applied resolution of the first action must supersede redo exactly as normal confirmation does; Tasks 1 and 6 test this.
- Empty and partial records must preserve selection and formatting behavior; Tasks 1 and 5 test zero/all limits and progress display.
- An older development database with `records.position` must be rejected without rewriting; Task 2 tests both open modes.
- Moving validation must not accept patches on non-tag actions or lose strict action/metadata JSON behavior; Tasks 1 and 4 test these contracts.

## File responsibilities

- `crates/core/src/history/model.rs`: concrete record progress and pure replay rules.
- `crates/core/src/history/attempt.rs`: run/attempt transactions and shared attempt validation.
- `crates/core/src/history/database.rs`: schema checks and fully validated record decoding.
- `crates/core/src/history/patch.rs`: public patch types and internal patch persistence/validation.
- `crates/core/src/history/runtime.rs`: session cache and borrowed selection API.
- `crates/core/src/history/persistence.rs`: load/reload and cache validity boundary.
- `crates/core/src/history/schema-v1.sql`, `docs/history/schema-v1.sql`: initial schema and snapshot.
- `crates/tfmt/src/history/execution.rs`: typed execution context and committed-state error reporting.
- CLI formatter/commands and core integration tests: migrate consumers and verify behavior.
- Create `crates/core/tests/history_cache.rs`: cache/reopen behavior tests. Internal fault-injection and decode-count tests live beside the implementation under `#[cfg(test)]`.

### Task 1: Concrete progress and shared invariants

**Files:** Modify `model.rs`, `database.rs`, `attempt.rs`; update `crates/core/tests/history_contract.rs`, `history_attempts.rs`, `history_operations.rs`, and `fixtures/history/v1-{all-variants,null-optionals}.json`.

**Interfaces:**
- `Record::can_undo(&self) -> bool`, `Record::can_redo(&self) -> bool`.
- `Record::next_action_position(&self, kind: OperationKind) -> Option<usize>`: apply returns length; undo uses checked subtraction; redo returns the cursor only when eligible.
- Concrete private progress fields and direct accessors; `Record::from_storage(id, actions, state, timestamp, metadata, applied_count, complete, redo_allowed) -> Result<Record>` validates the cursor/state contract. Remove `set_progress`.
- Internal `AttemptDetails::validate(&self) -> Result<()>` and `validate_attempt(connection: &Connection, attempt: &CurrentAttempt) -> Result<()>` serve both creation and loading. Tag/patch checks move to Task 4's helpers later.

- [ ] Add tests that preview records initialize `(len, true, true)`, records require all progress fields when deserializing, and persistence retains explicit partial progress. Preserve null tag locator/encoding assertions and strict action/metadata checks.
- [ ] Add an eligibility/cursor matrix covering applied, undone, redone, superseded, empty, partial, and redo-disabled records. Exercise both run admission and selection through existing APIs; test wrong position, mismatched replay action/patch, and empty reporting details through mutation and reload.
- [ ] Run `cargo test -p tfmttools-core --test history_contract --test history_attempts --test history_operations`; verify new expectations fail before implementation.
- [ ] Implement concrete construction, shared helpers, and one attempt validator. Construct the prospective `CurrentAttempt` before insertion for validation; assign its durable ID only after insertion. Do not validate by writing then rereading the attempt.
- [ ] Update existing record fixtures with explicit progress matching their states. Keep action/metadata representations unchanged; remove fallback-specific expectations.
- [ ] Run `cargo xtask test-core`, fix failures, and commit this independently testable change.

### Task 2: Simplify stored records and initial schema

**Files:** Modify `database.rs`, `attempt.rs`, `persistence.rs`, both SQL schema files, `history_schema.rs`, `history_database_validation.rs`, and SQL-bearing history tests.

**Interfaces:** `database::read_record(&Connection, usize) -> Result<Record>` and `read_records(&Connection) -> Result<Vec<Record>>`; remove `DatabaseRecord`. Record queries order by ID and no longer select position.

- [ ] Add a database test inserting valid records with IDs `2, 7, 11` out of insertion order; assert ascending cache order, descending undo order, and unchanged timestamp offsets. Retain rejection of noncontiguous action positions.
- [ ] Build an old-shape database by adding `position INTEGER NOT NULL DEFAULT 0` to the new records table. Assert writable and read-only loading reject it and database bytes remain unchanged. Existing foreign/version/schema rejection cases remain.
- [ ] Run `cargo test -p tfmttools-core --test history_schema --test history_database_validation`; verify the new ordering/schema expectations fail where applicable.
- [ ] Remove record position, its allocation query, and wrapper mappings; adapt all SQL fixtures and constructors. Amend version 1 directly, retaining identity pragmas, strict tables, foreign keys, and durability settings.
- [ ] Run `cargo xtask history-schema` and verify its result matches the amended SQL snapshot. Run `cargo xtask test-core`, then commit.

### Task 3: Typed operation requests and CLI execution

**Files:** Modify `history/{attempt.rs,mod.rs}`, `crates/tfmt/src/history/execution.rs`, `crates/core/tests/support/history.rs`, and all `begin_run` consumers found with `rg`.

**Interfaces:**
- Public `RunRequest::Apply { metadata: ActionRecordMetadata }` and `RunRequest::Replay { record_id: usize, direction: HistoryMode }`.
- `History::begin_run(&mut self, request: RunRequest) -> Result<RunId>`.
- `impl From<HistoryMode> for OperationKind` supplies the sole direction conversion.
- CLI-private copyable `ExecutionContext::{Apply, Replay { record_id: usize, direction: HistoryMode }}` with `kind(self) -> OperationKind`.
- `prepare(history, context, position, action)` and `execute_actions(history, run, context, actions)` replace kind/optional-ID parameters; their remaining parameter and return types stay unchanged.

- [ ] Preserve/add behavior cases for apply metadata retention, selecting a missing or ineligible replay record, and tag replay in both directions. Invalid request combinations are eliminated by enum construction rather than runtime tests.
- [ ] Run `cargo xtask test-core` and `cargo xtask test-cli` as the baseline before changing signatures.
- [ ] Implement the request enum, conversion, and context; migrate core test helpers and all call sites. Branch on context for tag preparation, remove replay ID unwrap, and keep the shared execution loop unchanged otherwise.
- [ ] Run `cargo xtask test-core`, `cargo xtask test-fs`, and `cargo xtask test-cli`; confirm original-retention, partial replay, and resolution cases pass, then commit.

### Task 4: Consolidate patch persistence and error categories

**Files:** Modify `history/{patch.rs,attempt.rs,database.rs}`, `history_database_validation.rs`, and `history_attempts.rs`. Keep filesystem binary patch implementation unchanged.

**Interfaces:** Internal patch module functions:
- `validate_pair(&BinaryPatchPair) -> Result<()>` for mutation input.
- `validate_action_pair(&StoredAction, Option<&BinaryPatchPair>) -> Result<()>` for mutation input.
- `read(&Connection, record_id: usize, position: usize) -> Result<Option<BinaryPatchPair>>`.
- `read_attempt(&Connection, AttemptId) -> Result<Option<BinaryPatchPair>>`.
- `insert(&Connection, usize, usize, &BinaryPatchPair) -> Result<()>`.
- `insert_attempt(&Connection, AttemptId, &BinaryPatchPair) -> Result<()>`.
- `validate_records(&Connection, &[Record]) -> Result<()>` for the load boundary.

Reading helpers map invalid persisted pair/association data to `LoadError`, with record/position or attempt context. Mutation validation returns `MutError`. Shared length/parameter encoding returns `SaveError` on conversion failure. SQLite failures remain `Database` errors. Share validation rules without losing boundary-specific categories.

- [ ] Add table-driven mutation/load tests for wrong format, short/missing BSDIFF headers, invalid lengths/hash widths, tag actions missing patches, and non-tag actions with patches. Use constraint-disabled fixtures only for malformed persisted values that normal SQL constraints reject.
- [ ] Run `cargo test -p tfmttools-core --test history_attempts --test history_database_validation`; verify missing category/context expectations fail.
- [ ] Move persistence functions into `patch.rs`; share row decoding and patch parameter encoding but keep explicit SQL per table. Replace positional INSERTs in touched confirmation/patch code with explicit columns. Use the shared pair association rule in attempt and record loading.
- [ ] Run `cargo xtask test-core` and `cargo xtask test-fs`; retain exact replay and stale-file rejection coverage, then commit.

### Task 5: Borrowed, infallible cache queries

**Files:** Modify `runtime.rs`, `crates/tfmt/src/history/{formatter.rs,mod.rs,execution.rs}`, `commands/{undo_redo.rs,show_history.rs,clear_history.rs}`, `commands/rename/template_resolution.rs`, and history test consumers.

**Interfaces:**
- `History::records_for(&self, HistoryMode, Option<usize>) -> Vec<&Record>`.
- `History::previous_record(&self) -> Option<&Record>`.
- `HistoryFormatter::format_history(&self, &History) -> String`.
- `HistoryFormatter::format_records(&self, &[&Record]) -> Option<String>`.
- CLI `get_records` returns owned `Vec<Record>` by cloning only selected replay records before mutation; confirmation/output remain fallible.

- [ ] Adapt selection tests to the new API and assert `Some(0)`, `Some(1)`, overlarge limits, and `None` preserve expected undo/redo ordering and eligibility. Add formatter coverage for empty history, partial cursor, and redo-disabled records with unchanged text.
- [ ] Run `cargo xtask test-core` and `cargo xtask test-cli`; new API references initially fail compilation.
- [ ] Replace wrappers with two ordinary iterator branches using shared eligibility predicates and `limit.unwrap_or(usize::MAX)`. Borrow for formatting and previous-template resolution; clone explicitly at replay's mutable boundary. Remove obsolete `?` and imports only where operations became infallible.
- [ ] Use `rg` to confirm all old accessor names are gone. Run `cargo xtask test-core` and `cargo xtask test-cli`, then commit.

### Task 6: Transactional affected-record cache publication

**Files:** Modify `history/{attempt.rs,runtime.rs,persistence.rs}`, `crates/tfmt/src/history/execution.rs`; create `crates/core/tests/history_cache.rs`; add internal tests beside transaction/cache helpers.

**Interfaces and failure boundary:**
- Internal `History::publish_records(&mut self, records: Vec<Record>)` upserts affected records in ID order without database access.
- Internal `History::require_usable(&self) -> Result<()>` rejects poisoned sessions at mutation boundaries.
- Internal `cache_usable: bool`, initially true; successful `load` restores it only after the complete snapshot validates. Infallible `records`, `records_for`, `previous_record`, and `is_empty` assert this precondition, with a clear programmer-error message. They must never return stale records from a poisoned session.
- Add public `History::ensure_usable(&self) -> Result<()>` for fallible CLI reporting boundaries. It delegates to the same session check; it performs no IO.
- Mutation code invalidates the cache immediately before `tx.commit()`. A successful commit publishes staged records and restores usability; a commit error leaves the session poisoned. Statement/decoding failures before commit leave the previously valid cache unchanged because the transaction rolls back.
- `finish_run` reloads committed history through `load()` on its error branch before reading attempts, reporting, or closing an abandoned run. If reload fails, return contextual original/reload errors and leave the session unusable. `interruption_report` checks `ensure_usable()` before reading cached records. Other callers already start with a successful load or propagate mutation errors without further cache access.

- [ ] Add cache-versus-read-only-reopen assertions after apply start, confirmation, undo/redo, close, and both resolution outcomes. Cover first-action supersession of an undone record and preservation of another record's applied prefix; subsequent confirmations must not change those records again.
- [ ] Add rollback tests using an aborting confirmation trigger: cached records remain unchanged and the attempt remains durable. Add internal commit fault injection immediately at the commit boundary: assert cache access cannot return stale data, mutation fails until load succeeds, and load restores the committed database state. Test failed reload leaves the guard in force. Keep test hooks compiled only for tests.
- [ ] Add a scoped thread-local decode observer in record decoding under `#[cfg(test)]`. With unrelated older records and a multi-action active record, assert later confirmations decode only the active record. First-confirmation supersession may decode affected older records, but not unaffected ones. No timing threshold or production tracing dependency.
- [ ] Run the new cache tests before optimization to demonstrate the unrelated-record decoding failure.
- [ ] Stage affected records inside each transaction before commit. Query supersession IDs with the same predicate as the update, only for the first apply confirmation (`action_position == 0`). Reuse this confirmation path for manual applied resolution. Preserve synchronous per-action commits and patch handling.
- [ ] Remove full refresh calls. Apply start publishes its new record, replay start publishes none, attempt creation publishes none, and close/resolution publish only affected records. Apply the commit guard to all history transactions that can produce an uncertain session, including attempt insertion.
- [ ] Verify `finish_run` reloads before errors use history, artifact retention survives commit/reload errors, and resolution failures leave markers intact. Run `cargo xtask test-core`, `cargo xtask test-fs`, and `cargo xtask test-cli`, then commit.

### Task 7: Documentation, final verification, and review

**Files:** Modify `README.md`, `CHANGELOG.md`, relevant history/fixture documentation and crate guidance discovered with `rg`. Review all modified production/test files against the spec.

- [ ] Document concrete progress in the record contract and the amended undeployed schema. Remove obsolete accessor/fallback references and retain existing manual-resolution instructions. Do not promise constant-time active-record append or automatic recovery.
- [ ] Check all seven spec sections against the diff: old wrappers, `DatabaseRecord`, record position, optional progress, full mutation refresh, duplicated attempt rules, and replay-ID unwrap must be absent.
- [ ] Run `cargo +nightly fmt --all`, `cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`. Fix actionable failures and rerun affected checks. Record exact results and any environment blockers.
- [ ] Review transaction failure paths, borrowed cache lifetimes, schema rejection without writes, and filesystem artifact retention. Apply the requesting-code-review skill for the final review workflow; do not merge or publish without authorization.
- [ ] Commit documentation and verification fixes. Report changes, evidence, and remaining limitations with links to the spec and plan.
