# SQLite History and Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for native execution or superpowers:subagent-driven-development if the user selects delegation. Implement task by task; steps use checkbox syntax.

**Goal:** Store history in SQLite, restore individual native tag changes, and
recover interrupted in-place operations using a durable journal and one temporary
audio-file backup.

**Architecture:** Core owns concrete stored actions, SQLite persistence, and
journal transitions. Fs owns native tag parsing/writing, effect preparation,
verification, and backup IO. A CLI coordinator commits intent before invoking
effects and completion before deleting the backup or proceeding.

**Tech Stack:** Rust 2024, rusqlite, rusqlite_migration, handwritten SQL, existing
Lofty decoding where useful, native format adapters, fixture-backed CLI tests.

**Spec:** [Approved design](../specs/2026-10-08-sqlite-history-design.md), committed
as `4556f9f`. Read it alongside this plan.

## Global Constraints

- Workspace MSRV: Rust 1.91.0; SQLite dependency provides at least SQLite 3.46.0.
- No ORM, schema generation, migration generation, or runtime schema sync.
- SQLite application ID identifies this new format; first user_version is 1.
- rusqlite_migration owns user_version; migrations are immutable embedded SQL.
- Writable connections: journal_mode=DELETE, synchronous=EXTRA, foreign_keys=ON;
  read back and verify settings before file effects.
- All application tables use STRICT with explicit semantic constraints.
- Read-only sessions never create databases, apply migrations, optimize, or change
  journal mode; missing history remains missing.
- Reject all old JSON history before file effects; no imports or legacy replay.
- Support ID3v2/MP3, Vorbis comments/Ogg, and MP4 ilst/M4A, without adding extensions.
- Preserve unrelated values, including siblings sharing a frame or atom.
- No automatic whole-file restoration or staged replacement of audio files.
- An independent, verified, durable backup precedes every audio-file write.
- Process effects sequentially; stop on failed completion or backup cleanup.
- Existing command-wide OS locking and resolved symlink behavior remain intact.
- Confirmed clear-history can remove incompatible history; retained recovery
  backups remain and their paths are reported. Do not delete real user history.
- Best-effort PRAGMA optimize only after a successful writable session; leave
  cache_size/temp_store defaults; no separate analysis_limit.
- Update README, CHANGELOG, repository guidance, and affected CLI fixture docs.

## Review Focus

1. Hard-linked inputs appearing twice: both paths reference one inode; sequential
   operations must recheck evidence and must not duplicate a stale planned edit.
2. An external process changes a file while its backup is copied: fail preparation
   before writing when identity/content no longer matches the captured source.
3. Disk space disappears after a successful backup: stop after uncertain IO and
   retain that verified backup, including if the database completion also fails.
4. Non-ASCII and symlinked audio/history paths: resolve identity without replacing
   links, and report exact original/backup paths for recovery.
5. Duplicate or multi-value entries reordered by another editor: preserve siblings
   and reject ambiguous selectors rather than overwrite an arbitrary entry.

Tasks 3–5 test entry ambiguity, Task 7 tests backup/source races and paths, and
Tasks 8–10 test hard-linked inputs and disk-full progress.

## File and interface map

Retain `crates/core/src/history/{model,stored,runtime,conversion,error,mod}.rs`.
Replace JSON persistence with focused `database.rs`, `journal.rs`, `rows.rs`,
`locking.rs`, and `migrations/0001.sql`. Retire `migration.rs`,
`legacy_tag_keys.rs`, `schema.rs`, and the JSON wrapper after CLI cutover.

Create `crates/core/src/action/tag_edit.rs` for plain native change data and
`crates/core/src/history/operation.rs` for journal evidence types.
Create `crates/fs/src/tag_edit/{mod,id3v2,vorbis,mp4}.rs` and
`crates/fs/src/action/{prepare,inspect,backup}.rs`. Core never imports fs.
Create `crates/tfmt/src/history/{execution,recovery}.rs` and
`crates/tfmt/src/commands/recover_history.rs`.

All signatures below use core history `Result` or fs `FsResult` as appropriate.
IDs are distinct `RecordId`, `ActionId`, `AttemptId`, and `OperationId` newtypes
over i64; checked conversions expose existing zero-based public IDs to callers.
Native binary selectors are versioned by the SQLite migration contract.

Implement the shared types in Task 2 before dependent tasks:

- `NativeTagFormat::{Id3v2, VorbisComments, Mp4Ilst}`.
- `NativeTagChange { key: String, kind: TagValueKind, old_display: String,
  new_display: String, format: NativeTagFormat, selector: Vec<u8>,
  before: Option<Vec<u8>>, after: Option<Vec<u8>> }`.
- `PreparedOperation { actions: Vec<Action>, evidence: OperationEvidence }`;
  one grouped tag operation can refer to several actions for the same file.
- `PreparedAttempt { record_id: RecordId, attempt_id: AttemptId,
  operation_ids: Vec<OperationId> }`.
- `OperationEvidence` is an explicit enum for tag/file/directory effects. Native
  evidence records file identity and structural checks; filesystem evidence also
  records path existence/type, content digest as needed, and staged path links.
- `EffectObservation::{Before, After, Conflict { reason: String }}`.
- `FileIdentity` stores device/inode on Unix and corresponding stable file
  identity on other supported platforms, with platform tags and checked values;
  modification time and length are evidence, never sole identity.
- `BackupDescriptor { operation_id: OperationId, source: Utf8PathBuf,
  path: Utf8PathBuf, source_identity: FileIdentity, length: u64,
  digest: Vec<u8>, state: BackupState }`.
- `RecoveryChoice::{Resume, Cancel}`; partial undo/redo cancellation is rejected.

## Task 1: Prove native byte capture and selective replacement

**Files:** Create `docs/history/native-edit-probes.md`; keep throwaway Rust
probe code outside production modules (for example under `/tmp`). Read local
Lofty 0.25.4 source and native format specifications, not generated report assets.

**Interfaces:** Consumes the approved format requirements. Produces documented
binary selector/snapshot layouts and evidence that each can preserve siblings,
unknown native entries, and audio payload through resizing.

- [ ] Construct small independent files covering ID3v2.2/v2.3/v2.4 and text
  encodings/unsynchronisation, duplicate Vorbis comments and multi-page packets,
  MP4 UTF8/UTF16/freeform/multi-data atoms and offsets affected by resizing.
- [ ] Probe capture/replace/restore and compare native changed bytes, untouched
  entry bytes, and audio payload. Include editing a sibling after initial apply.
- [ ] Record proven selector layouts, format references, library gaps, and
  format-specific writer decisions in `native-edit-probes.md`. No generic Tag
  reserialization may stand in for native byte preservation.
- [ ] If exact selective restoration is infeasible for any fixture, stop and
  revise the design with the user before implementing an inferior fallback.
- [ ] Commit only the findings: `Document native tag edit byte contracts`.

## Task 2: Define native actions and operation evidence

**Files:** Create core `action/tag_edit.rs`, history `operation.rs`; modify
`action/mod.rs`, history `stored.rs`, `conversion.rs`, `model.rs`, `mod.rs`;
create `crates/core/tests/native_action_contract.rs`.

**Interfaces:** Produces shared types above; add a native executable tag action
alongside the planned decoded edit until cutover. Introduce
`RecordExecutionStatus::{Pending, Complete, Partial, Cancelled}` separately from
existing `RecordState`; introduce attempt/progress/backup states from the spec.

- [ ] Write tests for stored/executable conversion, canonical names, selector
  validation, SQL-null absence versus empty BLOB, and invalid discriminator data.
  Pin the distinction with `assert_ne!(None, Some(Vec::<u8>::new()))` and actual
  round trips for each native format. Record tests exclude pending/cancelled
  runs from undo/redo and include resolved partial apply runs.
- [ ] Run `cargo test -p tfmttools-core --test native_action_contract`; expect the
  new behavior to fail before implementation.
- [ ] Implement the plain types, exhaustive action conversions, and separate
  execution/replay state. Enforce checked public IDs and decode bounds; keep
  the temporary decoded planner path private to the transition, not persisted
  in the new database.
- [ ] Run the same test and existing `history_operations` tests; expect passes.
- [ ] Commit: `Define native tag actions and journal evidence`.

## Task 3: Implement ID3v2 selective edits

**Files:** Create fs `tag_edit/mod.rs`, `tag_edit/id3v2.rs`; modify fs `lib.rs`;
create `crates/fs/tests/native_id3v2.rs` and local native fixture helpers.

**Interfaces:** Shared public interface in `tag_edit/mod.rs`:
`prepare(path: &Utf8Path, changes: &[TagValueChange]) -> FsResult<PreparedOperation>`;
`inspect(path: &Utf8Path, changes: &[NativeTagChange], direction: HistoryMode)
-> FsResult<EffectObservation>`;
`apply(path: &Utf8Path, changes: &[NativeTagChange], direction: HistoryMode)
-> FsResult<()>`. Fresh apply uses forward direction; direction-specific tag
before/after comparison follows the spec. Private ID3 adapter implements these.

- [ ] Write `restores_original_bytes_after_apply_undo_redo`,
  `preserves_later_sibling_edit`, `rejects_ambiguous_duplicate`, and resizing/
  version/unsynchronisation tests from Task 1. Assert container validity and
  independently extracted unaffected bytes, not just decoded text equality.
- [ ] Run `cargo test -p tfmttools-fs --test native_id3v2`; expect failure.
- [ ] Implement native ID3 framing, selectors, value merging, and in-place
  container updates using Task 1's contracts. Preflight all units before writing.
  Do not version-upgrade, normalize other frames, or match by key alone.
- [ ] Run the focused suite; expect passes for every proven variant.
- [ ] Commit: `Preserve native ID3v2 bytes during selective tag edits`.

## Task 4: Implement Vorbis selective edits

**Files:** Create fs `tag_edit/vorbis.rs`; modify `tag_edit/mod.rs`;
create `crates/fs/tests/native_vorbis.rs`.

**Interfaces:** Implements Task 3's shared interface for VorbisComments using
Task 1's comment/occurrence selector and snapshot contracts.

- [ ] Write tests for exact field spelling, duplicate keys, reordered unrelated
  comments, empty entries, vendor/picture preservation, and larger/smaller
  comments spanning Ogg pages. Assert identical audio packets and valid CRCs.
- [ ] Run `cargo test -p tfmttools-fs --test native_vorbis`; expect failure.
- [ ] Implement raw comment replacement and Ogg page rebuilding in place;
  preserve existing packets and reject ambiguity. Add no new extensions/codecs.
- [ ] Run the focused suite; expect passes.
- [ ] Commit: `Preserve native Vorbis comments during selective tag edits`.

## Task 5: Implement MP4 selective edits

**Files:** Create fs `tag_edit/mp4.rs`; modify `tag_edit/mod.rs`;
create `crates/fs/tests/native_mp4.rs`.

**Interfaces:** Implements Task 3's shared interface for Mp4Ilst using Task 1's
atom/data-child selector and snapshot contracts.

- [ ] Write tests for freeform mean/name identity, UTF8/UTF16 and type/locale,
  multiple data children with subsequent sibling edits, ambiguous duplicate
  items, extended sizes, moov before/after media, and growing/shrinking metadata.
  Assert unchanged media payload, unrelated atoms, and correct offsets.
- [ ] Run `cargo test -p tfmttools-fs --test native_mp4`; expect failure.
- [ ] Implement byte-preserving child merges and required size/offset updates
  in place, including checked length arithmetic and unknown atom preservation.
- [ ] Run the focused suite; expect passes.
- [ ] Commit: `Preserve native MP4 items during selective tag edits`.

## Task 6: Add SQLite storage and durable journal transitions

**Files:** Add dependency declarations in root/core Cargo.toml and Cargo.lock;
create core history `database.rs`, `rows.rs`, `journal.rs`, `locking.rs`,
`migrations/0001.sql`; modify `runtime.rs`, `persistence.rs`, `error.rs`, `mod.rs`;
create core tests `history_sqlite.rs`, `history_journal.rs`.

**Interfaces:** Add `History::prepare_write(&mut self) -> Result<()>`,
`begin_apply(&mut self, actions: Vec<StoredAction>, metadata: ActionRecordMetadata)
-> Result<PreparedAttempt>`,
`begin_replay(&mut self, record_id: RecordId, direction: HistoryMode)
-> Result<PreparedAttempt>`,
`prepare_operation(&mut self, id: OperationId, prepared: &PreparedOperation)
-> Result<()>`, `complete_operation(&mut self, id: OperationId) -> Result<()>`,
`mark_conflict(&mut self, id: OperationId, reason: &str) -> Result<()>`,
`unresolved_attempts(&self) -> Result<Vec<JournalAttempt>>`,
`cancel_attempt(&mut self, id: AttemptId) -> Result<()>`, and
`optimize_after_success(&mut self) -> Result<()>`. `JournalAttempt` contains
record/direction/status and ordered operations with evidence/progress/backups.

- [ ] Write tests for new database round trips and exact types/constraints in all
  six tables; ordered IDs, timestamps, and metadata; absent/empty snapshots;
  application/version rejection; settings readback; and no read-only creation.
  Use the fixed application ID `0x54464D54` (ASCII TFMT, signed-i32 compatible).
  Add rollback/retry tests for prepare and complete commits and the invariant
  `assert!(pending_operation_is_visible_from_fresh_connection)` before effects.
- [ ] Run `cargo test -p tfmttools-core --test history_sqlite --test history_journal`;
  expect failures before implementation.
- [ ] Select pinned rusqlite/migration versions compatible with Rust 1.91.0,
  use a bundled SQLite build >=3.46.0, and embed immutable migration SQL.
  Implement relational validation and incremental updates; verify application
  ID before upgrading existing databases. Retain one command lock with shared
  resolution for symlink/direct paths; move existing locking tests to SQLite.
- [ ] Implement the transition methods atomically. Begin attempts do not claim
  completed effects. Complete the last operation and record replay/execution
  state together; supersede redo records only once a new run has actual effects.
  Cancellation retains operation evidence and finalizes only completed apply
  actions; partial undo/redo cancellation fails. Maintain explicit positions.
- [ ] Port existing runtime persistence/locking/operation tests to SQLite when
  their exercised path switches; keep isolated JSON decoder/schema tests until
  removal in Task 10. Run focused tests and `cargo test -p tfmttools-core`; snapshot migrated
  schema independently and run migration validation. Simulate a future database
  and assert its bytes remain unchanged after rejection. Optimize is optional
  and never changes the classification of a completed operation.
- [ ] Include completed operations with outstanding backup cleanup in
  `unresolved_attempts`, and reject subsequent effects until cleanup resolves.
  Cancellation must retain a ready backup until its operation is verified before
  state and safely resolved; never cancel by deleting recovery evidence first.
- [ ] Commit: `Store history and operation progress in SQLite`.

## Task 7: Add durable independent audio backups

**Files:** Create fs action `backup.rs`; add digest dependency declarations as
needed; create `crates/fs/tests/action_backups.rs`; add backup lifecycle methods
to core `journal.rs` and focused transition tests.

**Interfaces:** Fs exports `reserve_backup(source: &Utf8Path, operation_id:
OperationId) -> FsResult<BackupDescriptor>` (describes a unique adjacent path,
does not create it), `create_backup(backup: &BackupDescriptor) ->
FsResult<BackupDescriptor>`, `verify_backup(backup: &BackupDescriptor) ->
FsResult<()>`, `delete_backup(backup: &BackupDescriptor) -> FsResult<()>`.
Core adds `register_backup(&mut self, backup: &BackupDescriptor) -> Result<()>`
and `set_backup_state(&mut self, id: OperationId, state: BackupState)
-> Result<()>`. FileIdentity uses checked platform-specific identity values.

- [ ] Write tests for exclusive creation, byte-identical independent copy,
  restrictive permissions, hard-linked original unchanged, Unicode/symlink paths,
  source mutation during copying, interrupted creation, missing backups, and
  rejection of swapped/symlink backup paths. Verify delete only affects the owned
  backup and directory synchronization participates in durability.
- [ ] Run `cargo test -p tfmttools-fs --test action_backups`; expect failure.
- [ ] Implement copying through resolved source handles; hash bytes with SHA-256
  and verify identity/length/content against captured evidence before ready.
  Document race limits with non-cooperating external writers. Commit building
  before creation and ready after sync; failure never permits original writes.
  Use idempotent cleanup with owned identity checks and no unsafe path reuse.
- [ ] Run the focused suite and core backup transition tests; expect passes.
- [ ] Commit: `Back up audio files before journaled in-place writes`.

## Task 8: Coordinate journaled effects and partial progress

**Files:** Create fs action `prepare.rs`, `inspect.rs`; modify handler/executor;
create CLI history `execution.rs`; modify rename `apply.rs`, `finish.rs`,
`validate.rs`, `undo_redo.rs`, history `mod.rs`; create
`crates/tfmt/tests/journal_execution.rs`.

**Interfaces:** Fs exports `prepare_action(action: &Action) ->
FsResult<PreparedOperation>` and `inspect_operation(operation: &PreparedOperation,
direction: OperationDirection) -> FsResult<EffectObservation>`, with
`OperationDirection::{Apply, Undo, Redo}`. CLI exposes
`execute_apply(history: &mut History, fs: &FsHandler, actions: Vec<Action>,
metadata: ActionRecordMetadata) -> Result<Record>` and
`execute_replay(history: &mut History, fs: &FsHandler, record: &Record,
direction: HistoryMode) -> Result<()>`. These methods own prepare/commit/effect/
sync/verify/commit/cleanup ordering; fs never writes the history database.

- [ ] Write tests asserting no effects before committed evidence/ready backup,
  no subsequent effects after failed completion/cleanup, correct partial apply
  history after a later failure, and replay state unchanged until full completion.
  Include two paths hard-linked to the same file and grouped tag changes.
  Check successful rename staging and copy/remove sequences have concrete paths
  and recoverable evidence before each effect, rather than just initial intent.
- [ ] Run `cargo test -p tfmt --test journal_execution`; expect failure.
- [ ] Implement coordinator using Tasks 2–7. Fully preflight tag conflicts before
  record replay; recheck each effect immediately before execution. Use native
  capture for validation and actual actions for history. Keep previews/dry runs
  read-only. Record errors without erasing pending state; stop immediately when
  the database cannot confirm progress. Flush changed files/directories before
  completion, with documented platform-specific limits.
- [ ] Preserve ordinary move/remove/directory semantics without inventing new
  reversibility. Keep unresolved attempts visible even when an IO error prevents
  writing an additional conflict row. Optional recovery databases retain journal
  and backup references and do not become the normal history implicitly.
- [ ] Run focused integration plus existing rename staging and fs action tests;
  expect passes, including successful native apply/undo/redo for all formats.
- [ ] Commit: `Journal file effects before execution and persist verified progress`.

## Task 9: Reconcile interrupted attempts and guide manual restoration

**Files:** Create CLI history `recovery.rs`, command `recover_history.rs`;
modify CLI `args_definition.rs`, `args.rs`, `options.rs`, command dispatch,
history formatter, show/clear commands; create
`crates/tfmt/tests/journal_recovery.rs`.

Dispatch edits live in `crates/tfmt/src/cli/mod.rs` and
`crates/tfmt/src/commands/mod.rs`; adjust the command/options definitions actually
used there rather than adding a parallel dispatch path.

**Interfaces:** `inspect_attempt(history: &History, fs: &FsHandler, id: AttemptId)
-> Result<RecoveryReport>` is read-only; `recover_attempt(history: &mut History,
fs: &FsHandler, id: AttemptId, choice: RecoveryChoice) -> Result<()>` performs
confirmed recovery. RecoveryReport gives per-operation observations and verified
backup paths. Add `tfmt recover-history` to inspect and offer existing-style
confirmation choices; `--resume` or `--cancel` choose one mutually exclusive
action. Add CLI option conversion in the existing parsing modules.

- [ ] Write tests for pending before/after/mixed states, missing/damaged audio,
  duplicate selector ambiguity, interrupted backup building and cleanup, partial
  apply cancellation, rejected partial replay cancellation, and resume after
  manual restoration. Assert read-only history never reconciles automatically.
  Check prompts name original and verified backup; incomplete building copies
  are never described as recoverable. Clearing retains backups and reports them.
- [ ] Run `cargo test -p tfmt --test journal_recovery`; expect failure.
- [ ] Implement inspection and explicit resume/cancel. Recognized after state
  is verified/synced then completed without repeating the effect. Before state
  awaits the user's choice. Mixed/ambiguous states retain evidence and block
  unrelated mutation. Manual restoration guidance describes copying into the
  existing file to preserve hard links and avoids automatic whole-file overwrite.
  Re-inspect restored files before resuming; cleanup verified backups only after
  resolution. Existing no-confirm/dry-run policies must not introduce implicit
  restoration or recovery during normal load.
- [ ] Format pending/partial/conflicting attempts separately from ordinary
  undo/redo lists, including failed cleanup on a completed operation.
- [ ] Run focused suite and `cargo xtask test-cli`; expect passes.
- [ ] Commit: `Recover interrupted history operations with explicit user choices`.

## Task 10: Remove JSON compatibility and finish contract verification

**Files:** Retire core history JSON `migration.rs`, `legacy_tag_keys.rs`,
`schema.rs` and wrapper code; remove `docs/history/schema-v1.json` and obsolete
compatibility fixtures/tests; modify `xtask/src/main.rs`, relevant Cargo files,
root/core AGENTS.md, README, CHANGELOG, and fixture docs. Replace legacy
`crates/tfmt/tests/history_compatibility.rs` with rejection tests and update
`path_templates.rs` for SQLite. Create `journal_crash_recovery.rs` in CLI tests.

**Interfaces:** Existing `History::load`, record-selection/formatter consumers,
and clear-history now accept SQLite only. Clear incompatible regular files
through a resolved-path lock without decoding; retain existing confirmation.
Remove public JSON schema generation and old-version support completely.

- [ ] Write tests for v0/v1 JSON and zero-length-file rejection before effects,
  exact unchanged source, explicit clear followed by fresh SQLite, symlink and
  lock invariants, recovery files, unsupported databases, and metadata/template
  reuse behavior. Keep minimal old samples solely for rejection tests.
- [ ] Run focused tests; expect the remaining old behavior to fail.
- [ ] Remove obsolete code/tooling and adapt remaining consumers. Audit references
  with rg; remove schemars only where no other consumers remain. Freeze SQL
  migration and create fixed SQLite compatibility fixtures independent of live
  serialization. Document incompatibility, backups, journal recovery, settings,
  partial runs, manual restoration, and platform durability limits.
- [ ] Add subprocess failure injection restricted to test builds/fixtures at
  prepare, ready, mid-write, post-effect, completion, delete, and cleanup commit.
  Kill the child, restart a fresh process, and assert observations/progress/backups.
  Inject disk-full-like IO and database errors, including combined write/commit
  failure. Assert at most one audio file requires recovery and that a ready backup
  stays intact. Do not call these tests proof of hardware power-loss durability.
- [ ] Run `cargo test --workspace`, `cargo +nightly fmt --all`,
  `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
  Fix actionable failures; rerun affected checks after edits.
- [ ] Review final code against each spec section and this plan's five Review
  Focus cases; commit: `Finalize SQLite history recovery and remove JSON support`.

## Handoff and verification notes

The native-format probes are a prerequisite for promising complete raw-byte
support; their outcome can require a design amendment. Do not substitute an
encoding label or Lofty-reserialized bytes if a probe fails.

Each task follows test failure, implementation, test success, and a focused
commit. Implementation begins in an isolated workspace according to the
using-git-worktrees skill. No production changes are made during this planning
stage. Plan review and execution-method selection precede implementation.
