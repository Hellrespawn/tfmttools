# SQLite History and Exact Replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Persist validated SQLite history and perform recoverable audio edits
whose undo/redo restores exact recorded bytes.

**Architecture:** Core owns the database and durable operation journal. Fs owns
binary patching, candidate verification, and recoverable file effects. The CLI
coordinates committed journal progress with ordered filesystem execution.

**Tech Stack:** Rust 1.91.0, rusqlite, rusqlite_migration, qbsdiff, Lofty,
SHA-256 through sha2, tempfile, SQLite 3.37.0 or newer.

**Spec:** `docs/superpowers/specs/2026-10-08-sqlite-exact-replay-design.md`

## Global Constraints

- Reject old JSON histories without importing, rewriting, or deleting them.
- Preserve IDs, record/action ordering, metadata, timestamps, and replay states.
- Use `STRICT` tables and SQLite 3.37.0 or newer.
- Use `journal_mode=DELETE`, `synchronous=EXTRA`, and `foreign_keys=ON`.
- Verify effective settings before filesystem effects; validate existing FKs.
- Commit both patches and pending intent before switching audio files.
- Retain the original through durable completion without another backup copy.
- Replay patches rather than rerunning Lofty; reject unexpected file bytes.
- Preserve symlinks by resolving their targets; reject hard-linked audio edits.
- Dry runs do not write audio, patches, or perform recovery switches.
- Preserve Rust MSRV 1.91.0 and the existing history pathname.

## Review Focus

- Multiple edits of the same path in a run require sequential expected hashes;
  test in Task 6 rather than preflighting every edit against initial bytes.
- Candidate or retained-original path collisions must preserve unrelated files;
  test exclusive reservation and recovery collision handling in Task 4.
- A deleted or redirected audio symlink during execution must stop safely;
  test recorded resolved-path identity in Task 4.
- Database commits failing after a successful file switch must retain recovery
  files and resume without repeating edits; test in Tasks 3 and 6.
- Copy/remove moves and directory cleanup interrupted between effects must
  neither lose progress nor delete externally changed files; test in Task 5.

## File responsibilities and shared types

- `core/src/history/database.rs`: connection setup, identity, migrations, SQL.
- `core/src/history/journal.rs`: pending operations and transactional progress.
- `core/src/history/patch.rs`: storage-only patch and byte-identity types.
- `core/src/history/{model,stored,conversion,runtime,persistence,error}.rs`:
  adapt current models and entry points; remove legacy JSON migration code.
- `fs/src/action/tag_edit.rs`: current Lofty mutation helpers on candidates.
- `fs/src/action/binary_patch.rs`: qbsdiff generation/application and hashes.
- `fs/src/action/file_switch.rs`: preparation, staged rename, reconciliation.
- `fs/src/action/recorded_execution.rs`: non-tag action preparation/recovery.
- `tfmt/src/history/execution.rs`: operation orchestration and recovery.
- Existing CLI command files: route writes/replay through orchestration.
- `docs/history/schema-v1.sql`: deterministic new SQLite contract snapshot.

Paths above are relative to `crates/` except documentation. Keep preview tag
changes separate from patch storage. Do not add patch buffers to template
planning's executable `Action`; prepared journal entries associate them with
ordered stored actions.

Core shared types:

- `ByteIdentity { length: u64, sha256: [u8; 32] }`.
- `BinaryPatchPair { format: String, before: ByteIdentity,
  after: ByteIdentity, forward: Vec<u8>, reverse: Vec<u8> }`.
- `OperationId(i64)` and `OperationKind::{Apply, Undo, Redo}`.
- `PreparedAction { action: StoredAction, recovery: RecoveryDescriptor,
  patches: Option<BinaryPatchPair> }`.
- `RecoveryDescriptor` stores resolved paths, expected identities/existence,
  candidate and retained-original paths where applicable, and the action's
  recovery policy. It is strict, tagged storage data, not executable code.
- `PendingOperation` contains ID, kind, record ID, ordered entries with completed
  flags, metadata for a new run, and cleanup progress. Applied record state and
  pending progress are distinct; no new replay state is needed.

## Task 1: Replace document persistence with validated SQLite

**Files:** Workspace/core manifests and lockfile; history database, persistence,
runtime, stored, error, mod; `core/tests/history_{persistence,migration,contract,
locking,schema,operations}.rs`; old migration/key-map code; xtask schema output.

**Interfaces:** Preserve `History::new/load/save/records` and record selection
while adding `History::open_read_only(path: Utf8PathBuf) -> Result<Self>`.
Expose `history_schema_sql() -> &'static str`. Use bundled SQLite with rusqlite
and a compatible rusqlite_migration release verified against MSRV before pinning.

- [x] Write database tests: saving then reopening preserves record values and
  order; header is `SQLite format 3\0`; effective journal mode is `delete`,
  synchronous is `3`, and foreign_keys is `1` on each writable connection.
  Old JSON bytes remain identical after rejection. Wrong application ID,
  future version, malformed timestamps/actions, duplicate IDs/positions,
  non-BLOB patch columns, and FK violations fail without replacing live records.
  Read-only opening leaves bytes unchanged and performs no migrations/optimize.
- [x] Run `cargo test -p tfmttools-core --test history_persistence`; confirm
  failures reflect JSON persistence or missing database APIs.
- [x] Implement database schema using migration-managed user_version. Reserve
  application_id `0x54464d54` (TFMT). Create STRICT records/actions/patches/
  operations/progress tables, explicit ordering columns, constraints, indexes,
  and transactional validation. Configure/verify PRAGMAs before transactions.
  Initialize only missing databases; do not claim unidentified existing ones.
  Preserve locking and history symlink resolution. Read-only open validates
  the current schema; reports an upgrade requirement for older versions.
- [x] Replace JSON migration/schema tests and snapshot with SQL contract checks.
  Retire `StoredHistory`, JSON migration, legacy key mapping, and obsolete
  backup errors when their consumers are removed. Keep strict action payload
  serialization inside SQL. Avoid whole-history rewrites as the final write API.
  Add best-effort optimize on writable close, with version-aware analysis_limit.
- [x] Run `cargo xtask test-core` and `cargo xtask history-schema`; expect tests
  passing and stable `docs/history/schema-v1.sql` output on regeneration.
- [x] Commit as `Replace JSON history with validated SQLite storage`.

## Task 2: Generate and verify reversible binary patches

**Files:** Core patch model/export; fs binary_patch/tag_edit/handler;
workspace/fs manifests and lockfile; new `crates/fs/tests/binary_patch.rs` and
`crates/fs/tests/tag_edit.rs`.

**Interfaces:** `byte_identity(bytes: &[u8]) -> ByteIdentity`;
`create_patch_pair(before: &[u8], after: &[u8]) -> FsResult<BinaryPatchPair>`;
`apply_patch(bytes: &[u8], pair: &BinaryPatchPair, direction: HistoryMode)
-> FsResult<Vec<u8>>`. Format identifier is `bsdiff40-v1`. Use the qbsdiff
BSDIFF40 representation and verify the actual library format before accepting it.
`write_tag_candidate(path: &Utf8Path, changes: &[TagValueChange]) -> FsResult`
mutates only the supplied candidate, rereads and verifies requested outcomes.

- [x] Write tests asserting forward output equals all edited bytes and reverse
  output equals all original bytes for empty, arbitrary binary, and real audio
  inputs. Assert stale input, wrong lengths/digests, unknown format, corrupt and
  truncated patches return errors. Tag candidate tests assert requested text,
  locator, and supported encoding changes; missing source matches fail.
- [x] Run `cargo test -p tfmttools-fs --test binary_patch`; expect missing API
  failures before implementation.
- [x] Implement patch generation with qbsdiff and SHA-256 identities; verify
  both complete round trips before returning a pair. Bound reconstruction by
  recorded target length and reject output mismatch. Extract Lofty helpers out
  of handler, preserving descriptions/languages and verifying final encodings.
  Leave initial editing and replay entry points distinct; raw tag serialization
  is never an undo/redo fallback.
- [x] Run `cargo xtask test-fs`; expect passing binary and audio tests.
- [x] Commit as `Add verified reversible binary patches for tag edits`.

## Task 3: Persist pending operations and progress transactionally

**Files:** Core journal/model/runtime/database/error/mod;
`crates/core/tests/history_journal.rs`.

**Interfaces:** `History::begin_operation(kind: OperationKind,
record_id: Option<usize>, metadata: Option<ActionRecordMetadata>)
-> Result<OperationId>`; `append_prepared(id: OperationId,
entry: PreparedAction) -> Result<usize>`; `complete_action(id: OperationId,
position: usize) -> Result<()>`; `finish_operation(id: OperationId)
-> Result<Record>`; `complete_cleanup(id: OperationId, position: usize)
-> Result<()>`; `pending_operations(&self) -> Result<Vec<PendingOperation>>`.
Each mutation commits before returning and refreshes live state only on success.

- [x] Write tests: committed intent and patch BLOBs survive reopen; progress is
  ordered and completion idempotent; invalid record/direction/position and a
  second conflicting pending operation fail. Incomplete operations cannot
  finalize; finalizing apply supersedes undone records only on success;
  finalizing undo/redo changes only the selected record. Inject commit failures
  and assert last committed progress/state/patches remain available.
- [x] Run `cargo test -p tfmttools-core --test history_journal`; expect missing
  journal methods before implementation.
- [x] Implement the shared types, SQL inserts/updates, validation, and atomic
  finalization. Keep journals after finalization until cleanup is acknowledged.
  `remove` refuses unresolved work and closes SQLite before unlinking. Prevent
  the transitional push/save API from bypassing journal invariants for tag edits.
- [x] Run `cargo xtask test-core`; expect passing journal/state tests.
- [x] Commit as `Journal history operations and action progress`.

## Task 4: Prepare, switch, and recover audio files safely

**Files:** Fs file_switch/tag_edit/error/lib/action exports;
`crates/fs/tests/file_switch.rs`.

**Interfaces:** `prepare_tag_edit(path: &Utf8Path, changes: &[TagValueChange])
-> FsResult<PreparedAction>`; `prepare_tag_replay(action: &StoredAction,
pair: &BinaryPatchPair, direction: HistoryMode) -> FsResult<PreparedAction>`;
`install_prepared(entry: &PreparedAction) -> FsResult<()>`;
`recover_prepared(entry: &PreparedAction) -> FsResult<()>`;
`cleanup_prepared(entry: &PreparedAction) -> FsResult<()>`.
Preparation owns candidate files until descriptors can be committed; failures
before commitment clean up only files created by that preparation.

- [x] Write tests: candidate editing preserves original bytes; persisted
  descriptor can resume after interruption before/after each rename; failed
  candidate install restores original; completed switch retains original until
  cleanup. Permissions survive; symlink remains; hard-linked inputs fail.
  Colliding candidate/retained paths and redirected symlinks preserve unrelated
  data. Ambiguous external changes stop without deleting either recovery file.
- [x] Run `cargo test -p tfmttools-fs --test file_switch`; expect missing APIs.
- [x] Implement sibling candidate and retained-path exclusive reservation,
  source identity recheck, file/directory sync, staged original retention and
  candidate installation. Recover by expected paths and hashes, not by progress
  flags alone; never overwrite an unexpected file. Recheck resolved identity.
  Keep originals on post-switch errors. Only clean up verified owned paths after
  the coordinator has durably finalized. Reject unsupported hard-link checks
  on platforms where safety cannot be established.
- [x] Run `cargo xtask test-fs`; expect all switch boundary tests passing.
- [x] Commit as `Switch audio candidates with recoverable original retention`.

## Task 5: Make ordered rename and cleanup effects recoverable

**Files:** Fs recorded_execution/executor/handler/rename_planner/rename_staging;
`crates/fs/tests/recorded_execution.rs`.

**Interfaces:** `ActionExecutor::plan_actions(actions: Vec<RenameAction>)
-> FsResult<Vec<Action>>` resolves concrete move or copy/remove actions before
execution, including staging and directory creation. `prepare_action(action:
&Action, direction: OperationKind) -> FsResult<PreparedAction>` prepares non-tag
preconditions; Task 4 install/recover/cleanup dispatch extends to these variants.

- [x] Write tests: planned swap/chain/case-only rename retains exact order;
  copy/remove plans resume between copy and removal; interrupted staging moves
  resume once; directory creation/removal distinguishes already-completed state
  from conflicts. External target changes reject recovery and remain intact.
  Undo/redo preserves existing copy/remove semantics and action order.
- [x] Run `cargo test -p tfmttools-fs --test recorded_execution`; expect failures
  against the execution-only iterator.
- [x] Expose concrete planning separately from effects. Prepare identity and
  existence descriptors per action immediately before journal append, preserving
  dependencies between actions. Implement idempotent reconciliation with
  checksums for files and safe directory checks. Unsupported ambiguous cases
  stop with actionable recovery paths. Dry-run planning must not reserve files.
- [x] Run `cargo xtask test-fs`; expect existing cycle tests and recovery tests
  passing, including forced-copy behavior.
- [x] Commit as `Record recoverable rename and cleanup progress`.

## Task 6: Coordinate commands through durable operations

**Files:** CLI history execution/mod/formatter; rename apply/finish/session/mod;
validate, undo_redo, clear_history, show_history; fs handler;
`crates/tfmt/tests/history_compatibility.rs` and new
`crates/tfmt/tests/history_recovery.rs`; relevant CLI cases.

**Interfaces:** `recover_pending(history: &mut History, fs: &FsHandler)
-> color_eyre::Result<()>`; `execute_recorded(history: &mut History,
fs: &FsHandler, actions: Vec<Action>, metadata: ActionRecordMetadata)
-> color_eyre::Result<Record>`; `replay_record(history: &mut History,
fs: &FsHandler, record: &Record, direction: HistoryMode)
-> color_eyre::Result<Record>`. The coordinator uses Tasks 2–5 and honors FSMode.

- [x] Write CLI tests: validate/edit, undo, redo compare complete fixture bytes;
  external mutation rejects replay; sequential same-path edits replay correctly;
  mixed rename/tag operations preserve ordered paths and states. Interrupt after
  intent, original retention, installation, progress commit, finalization, and
  cleanup; reopen and assert deterministic recovery without repeated edits.
  Inject database failure after installation and assert original retention.
  Dry runs leave audio/database untouched. Old JSON errors before effects;
  show-history reports pending work read-only; clear-history refuses it.
- [x] Run `cargo test -p tfmt --test history_recovery`; expect failures against
  current post-effects history saves and Lofty replay.
- [x] Route validate/rename/replay through journal-before-effect orchestration.
  Recover before creating plans dependent on filesystem state, except dry runs.
  Finalize replay per record. Integrate cleanup into the same ordered operation;
  avoid holding conflicting Rust borrows between session and mutable history.
  Read-only show opens read-only; reports pending operations without recovery.
  Remove direct unrecorded tag-edit routes and obsolete prepare_save/save callers.
  Preserve history previews, run metadata and command confirmation behavior.
- [x] Run `cargo xtask test-cli` and `cargo xtask test-integration`; expect
  exact-byte replay and all existing rename/validation fixtures passing.
- [x] Commit as `Coordinate file edits and replay with durable history`.

## Task 7: Document the contract and run final verification

**Files:** README, CHANGELOG, docs/history; core AGENTS.md; fixture docs;
xtask help/schema command; remaining obsolete JSON fixtures and schema assets.

**Interfaces:** The final history-schema command generates deterministic SQL;
the supported replay contract is the approved spec, reflected in user docs.

- [x] Update user docs: SQLite history, unsupported JSON handling without import,
  exact replay/stale rejection, temporary original retention, pending recovery,
  staged-rename visibility gap, symlink/hard-link behavior and clear-history rules.
  Remove obsolete automatic JSON migration advice and JSON schema references.
  Preserve intentional historical rejection fixtures; remove redundant assets.
- [x] Regenerate the SQL snapshot and run its contract test. Ensure repeated
  generation is unchanged and all public migration/settings invariants are
  exercised by Tasks 1 and 3.
- [x] Run `cargo +nightly fmt --all`, `cargo test --workspace`,
  `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
  Resolve failures and actionable warnings; record actual results before claiming
  completion. Review diffs for unrecorded filesystem mutations and cleanup paths.
- [x] Commit as `Document SQLite history and exact byte replay`.

## Plan self-review

The tasks cover storage identity/version/constraints, connection durability,
patch integrity, candidate semantics, switches, mixed action recovery, replay
states, dry runs, history commands, documentation, and required verification.
Shared signatures above are the coordinator contract. The five review-focus
conditions each have owning tests. The implementation intentionally keeps the
portable switch and session lock; atomic exchange and performance tuning remain
outside this plan.
