# Runtime Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Execution method requires user selection before implementation.

**Goal:** Address all eight runtime maintainability findings without weakening filesystem or history guarantees.

**Architecture:** Replace redundant plumbing with concrete helpers within the existing core/filesystem/CLI boundaries. Type runtime encodings, share native ID3 inspection, and gather complete validation results before execution. Separate command reporting from quiet execution guards.

**Tech Stack:** Rust workspace, Camino, Lofty, SQLite, existing Cargo fixture harness.

**Spec:** [Approved runtime simplification design](../specs/2026-10-11-runtime-simplification-design.md).

## Global Constraints

- Preserve Rust 1.91.0, edition 2024, and the current dependency set.
- Do not change the SQLite schema or binary patch format.
- Preserve CLI flags, template semantics, discovery order and extension matching, confirmation policy, rename and cleanup ordering, exact-byte replay, and stored action and metadata formats.
- Execution continues in this order: prepare, retain artifacts, commit intent, execute, confirm history, confirm action-specific housekeeping.
- No SQLite transaction spans filesystem effects.
- No automatic recovery, filesystem outcome inference, new execution framework, generic repository, persistent state, or dependency belongs in this change.
- A fix with incomplete inspection fails before applying any proposed tag changes.
- Keep the fixture harness's raw byte-level ID3 inspection independent.

Read the root and applicable crate AGENTS.md files before execution. Use an
isolated worktree only at implementation time, preserving these approved
documents. Keep each task buildable; stage only that task's files when committing.

## Review Focus

- Multiple native frames for one item key: validation uses the first, verification rejects any mismatch (task 5).
- A malformed file after a valid fixable file: no tag changes or new records, including dry run (task 6).
- Symlinked input directories and paths with spaces: preserve path components and displayed paths (task 3).
- An unresolved attempt encountered through a direct executor call: mutation remains blocked with no effects (task 7).
- Failure after effects or after durable confirmation: committed-state reporting and retained originals remain intact (task 7).

## File responsibilities and sequence

Tasks 1–3 remove option/planner plumbing. Task 4 changes runtime encoding types.
Task 5 creates `core/src/id3.rs` for pure frame matching and `fs/src/id3.rs` for
native reads. Task 6 creates `tfmt/src/commands/validate/inspection.rs` for
per-file findings and proposed changes; `validate.rs` retains command policy
and reporting. Task 7 changes interruption orchestration in the existing
execution module. Tasks 5–6 depend on task 4; task 7 depends on task 2.

These changes share workspace consumers and verification contracts, so retain
one ordered plan rather than independent subsystem plans.

## Task 1: Remove dead verbosity and simplify planning helpers

**Files:** Modify `crates/tfmt/src/cli/options.rs`,
`crates/tfmt/src/commands/show_history.rs`,
`crates/core/src/action/rename_action.rs`,
`crates/fs/src/action/rename_cycles.rs`,
`crates/fs/src/action/rename_planner.rs`.
Existing tests: `crates/tfmt/src/history/formatter.rs`,
`crates/fs/src/action/planning.rs`.

**Interfaces:** Replace the slice wrapper with
`pub(super) fn needs_temporary_staging(actions: &[RenameAction]) -> bool`.
Keep `RenameAction::intermediate_directories(&[RenameAction]) -> Vec<Utf8Directory>`.
No interfaces produced for later tasks.

- [ ] Remove the verbosity field, accessor, constant initialization, and
  unreachable `show_history` branch. Keep `HistoryFormat::Verbose`, its builder,
  and template formatting: `history::load_history` uses them in debug logging.
- [ ] Replace directory collection with direct target-parent ancestor iteration,
  filtering empty paths and collecting `BTreeSet<Utf8Directory>` into a vector.
  Update existing relative/absolute ancestor tests to exercise the public helper
  and remove obsolete private helper tests.
- [ ] Replace `RenameCycleDetector` with the function above and migrate the
  planner caller, retaining the case-insensitive overlap predicate unchanged.
- [ ] Run `cargo test -p tfmttools-core`, `cargo test -p tfmttools-fs`, and
  `cargo test -p tfmt`. Expect existing ordering, staging, and default history
  formatting tests to pass; inspect the diff for no algorithm-policy changes.
- [ ] Commit the task as `Simplify planner helpers and remove unused verbosity`.

## Task 2: Replace the filesystem-mode wrapper

**Files:** Modify `crates/core/src/util.rs`, `crates/fs/src/lib.rs`,
`crates/tfmt/src/cli/{mod,args}.rs`,
`crates/tfmt/src/commands/{undo_redo,validate}.rs`,
`crates/tfmt/src/commands/rename/{mod,session,cleanup}.rs`,
`crates/tfmt/src/history/execution.rs`, `crates/fs/AGENTS.md`, `CHANGELOG.md`.
Move `crates/fs/src/fs_handler.rs` to `crates/fs/src/path_helpers.rs` and delete
the wrapper from the moved file.

**Interfaces:** Produce `FSMode::is_dry_run(self) -> bool`.
`Command::run` (the existing dispatch type's method), `rename`, `validate`,
`undo_redo`, and `RenameSession::from_args` drop their `FsHandler` parameter.
Preserve each remaining parameter's type and order.
Execution signatures become:

```rust
check_interrupted(history: &mut History, mode: FSMode) -> Result<()>;
execute_recorded(history: &mut History, mode: FSMode,
    actions: Vec<Action>, metadata: ActionRecordMetadata) -> Result<Record>;
replay_record(history: &mut History, mode: FSMode,
    record: &Record, direction: HistoryMode) -> Result<Record>;
```

- [ ] Add `FSMode::is_dry_run`, replace wrapper uses with mode values, and remove
  the wrapper field/accessor from `RenameSession`. Cleanup reads its mode from
  session options. Preserve execution behavior while changing signatures.
- [ ] Move `get_longest_common_prefix` unchanged to `path_helpers.rs` and preserve
  its public export. Delete `FsHandler` and update filesystem guidance and the
  changelog for the removed internal workspace API.
- [ ] Migrate execution unit tests from handler construction to `FSMode` values.
  Run `cargo test -p tfmttools-fs` and `cargo test -p tfmt`, including existing
  dry-run and interruption integration tests. Expect success.
- [ ] Search Rust sources for `FsHandler` and `fs_handler`; expect no remaining
  code references. Commit as `Use filesystem mode directly across commands`.

## Task 3: Normalize input scope consistently

**Files:** Modify `crates/tfmt/src/cli/options.rs`,
`crates/tfmt/src/commands/validate.rs` for display-prefix use if needed;
create `crates/tfmt/tests/input_scope.rs` for process-level path regressions.

**Interfaces:** Private helper in `options.rs`:

```rust
fn resolve_input_scope(input: Option<Utf8PathBuf>, depth: Option<usize>)
    -> Result<(Utf8Directory, usize)>;
```

Both `RenameOptions` and `ValidateOptions` conversions consume this helper.
No new public scope type or stored display field.

- [ ] Add options tests `relative_input_is_absolute_without_canonicalization`,
  `absolute_and_default_input_scopes`, and `depth_zero_and_missing_paths`.
  Assert relative paths equal `cwd.join(input)`, absolute paths remain identical,
  `None` equals cwd, default depth is 4, explicit zero remains 0, nonexistent
  directories are accepted, and existing files are rejected. Avoid changing cwd
  in concurrently run unit tests.
- [ ] Add process-level tests in `input_scope.rs` using temp directories and
  child-process cwd: rename and validate select the same files through relative
  and absolute inputs with spaces. On Unix include a symlinked input and assert
  normalization preserves the symlink component. Pin existing relative-path
  diagnostic presentation before changing conversion.
- [ ] Run `cargo test -p tfmt --test input_scope` and the new options tests before
  implementation; expect relative validation normalization to fail its assertion.
- [ ] Implement the helper, resolve cwd once, join relative inputs without
  canonicalization, and migrate both conversions. Preserve diagnostic display
  with existing prefix formatting where needed. Keep template/trash conversion
  unchanged.
- [ ] Run `cargo test -p tfmt`; expect all tests and fixture expectations to pass
  without unrelated output updates. Commit as `Normalize command input scopes consistently`.

## Task 4: Type runtime encodings at the persistence boundary

**Files:** Modify `crates/core/src/action/mod.rs`,
`crates/core/src/history/conversion.rs`, `crates/fs/src/action/tag_edit.rs`,
`crates/tfmt/src/commands/validate.rs`, and affected test constructors;
extend `crates/core/tests/tag_encoding.rs` and
`crates/core/tests/history_contract.rs`.

**Interfaces:** `TagValueChange::with_encoding(self,
old_encoding: Option<TagTextEncoding>, new_encoding: Option<TagTextEncoding>) -> Self`;
`old_encoding(&self)` and `new_encoding(&self)` return `Option<TagTextEncoding>`.
`FieldFix` uses the same optional enum. Stored fields remain strings.

- [ ] Extend encoding tests to construct runtime changes for all four values,
  convert through `StoredAction`, assert exact canonical stored names, convert
  back, and assert the enum values. Test absent fields and stored names
  `utf8`, `UTF-8`, `unknown`, and ` UTF8`; each invalid name must be a load error.
- [ ] Run `cargo test -p tfmttools-core --test tag_encoding` before implementation;
  expect typed accessor/builder assertions to fail compilation.
- [ ] Change runtime fields and boundary conversions; migrate CLI construction,
  writer conversion, and workspace tests. Remove runtime parse/unknown-skip
  branches while keeping strict stored-name rejection and context.
- [ ] Run core, filesystem, and CLI tests. Inspect stored contract assertions and
  `git diff` to confirm no schema, stored field, or canonical spelling changes.
  Commit as `Represent runtime tag encodings with typed values`.

## Task 5: Share native ID3 reading and frame matching

**Files:** Create `crates/core/src/id3.rs`, `crates/fs/src/id3.rs`,
`crates/fs/tests/id3.rs`; modify both crate `lib.rs` files,
`crates/fs/src/action/tag_edit.rs`, `crates/tfmt/src/commands/validate.rs`,
and existing `crates/fs/tests/tag_edit.rs`.

**Interfaces:**

```rust
// core::id3, used by validation, modification, and verification
pub fn frame_matches_item_key(frame: &Frame, key: ItemKey) -> bool;
pub fn text_encodings_for_key(tag: &Id3v2Tag, key: ItemKey)
    -> impl Iterator<Item = TagTextEncoding> + '_;
// fs, exported native reader; format selects an explicit parser
pub fn read_native_id3v2(path: &Utf8Path, format: lofty::file::FileType)
    -> FsResult<Option<Id3v2Tag>>;
```

`read_native_id3v2` supports MPEG/WAV/AIFF; other formats return `Ok(None)`.
Supported formats propagate open/parse errors and return `Ok(None)` for no tag.
Use `ParseOptions::new().read_properties(false)`.

- [ ] Add core matcher tests for text IDs, both user-text matching conditions,
  unrelated/URL frames, missing keys, and multiple matches with differing
  encodings. Assert iterator order equals native order and preserves all matches.
- [ ] Add native-reader tests for supported formats, no native tag, malformed
  input, and unsupported format. Extend candidate tests with two matching
  user-text frames where one has the wrong encoding; verification must reject
  any mismatch rather than accept the first match.
- [ ] Run the new core/fs tests; expect missing helper APIs to fail compilation.
- [ ] Implement the pure matcher and native reader, retaining typed error/path
  context. Migrate validation to `.next()` for first-match policy, writer
  modification to the shared predicate, and verification to all-match checks.
  Parse the persisted candidate once when any change requests encoding, outside
  the change loop. Keep post-write generic validation and locator handling.
- [ ] Run `cargo test -p tfmttools-core`, `cargo test -p tfmttools-fs`, and
  `cargo test -p tfmt --test integration`. Expect raw fixture encoding assertions
  and exact candidate checks to pass. Inspect that no full-tag clone occurs per
  lookup and native candidate reads occur outside the changes loop; leave the
  independent harness reader unchanged. Commit as `Share native ID3 inspection`.

## Task 6: Inspect validation files once before executing fixes

**Files:** Create `crates/tfmt/src/commands/validate/inspection.rs` and
`crates/tfmt/tests/validation_inspection.rs`; modify
`crates/tfmt/src/commands/validate.rs`, `README.md`, `CHANGELOG.md`,
`tests/fixtures/cli/README.md`, `crates/tfmt/AGENTS.md`.

**Interfaces:** Move `ValidationScope`, `FieldFix`, issue types, and pure finding
logic into the inspection module with parent-module visibility. Produce:

```rust
pub(super) struct FileInspection {
    pub(super) tag_issues: Vec<TagValueIssue>,
    pub(super) id3_encoding_issues: Vec<Id3EncodingIssue>,
    pub(super) changes: Vec<TagValueChange>,
}
pub(super) fn inspect_file(path: &Utf8Path, scope: ValidationScope)
    -> FsResult<FileInspection>;
```

`inspect_file` loads generic audio once and native MPEG metadata at most once
when scope includes ID3 and extension is exactly `mp3`. The parent scan owns
progress, collects read errors, and converts nonempty changes to actions.

- [ ] Add `malformed_audio_blocks_all_fixes` in `validation_inspection.rs`:
  create a valid MP3 with a fixable title plus a discovered `.mp3` containing
  invalid bytes; exercise character/ID3 fixes and their dry runs. Assert failure,
  a diagnostic naming the malformed file, unchanged bytes for every file, no
  new record/attempt, and no `No tag values changed` success message. Reuse
  existing fixture/setup conventions; use valid non-ASCII UTF8 for ID3 cases.
- [ ] Add `check_and_fix_agree_on_findings` and
  `characters_only_does_not_require_native_inspection`: cover no applicable ID3
  tag, valid ASCII, non-ASCII UTF8, UTF16, text and locator character fixes.
  Check-only errors retain `ValidationFailed` and need no writable history.
- [ ] Run `cargo test -p tfmt --test validation_inspection` before implementation;
  expect malformed-input fix tests to fail because files are silently skipped.
- [ ] Implement `inspect_file` and migrate check/fix to one scan. Drop each
  file's metadata before the next file; remove the full-scan native-tag map and
  separate fix readers. Finish progress before returning collected diagnostics.
  On fix read errors, print the errors then return an operational error before
  calling `execute_recorded`; dry run follows the same policy. Keep history
  admission before discovery and candidate preparation at execution time.
- [ ] Update README/changelog/fixture guidance for the incomplete-inspection
  failure policy and task maps for the new module. Run `cargo test -p tfmt` and
  `cargo test -p tfmt --features debug`. Expect validation, replay, and fixture
  tests to pass, including stale-input rejection and raw encoding checks.
  Commit as `Unify validation inspection and reject incomplete fixes`.

## Task 7: Report interruption once and retain quiet execution guards

**Files:** Modify `crates/tfmt/src/history/execution.rs`,
`crates/tfmt/src/commands/{undo_redo,validate}.rs`,
`crates/tfmt/src/commands/rename/session.rs`,
`crates/tfmt/tests/history_interruption.rs`, `README.md`, `CHANGELOG.md`.

**Interfaces:** Keep `interruption_report(&History) -> Result<Option<String>>`.
Replace `check_interrupted` with:

```rust
pub(crate) fn admit_command(history: &mut History, mode: FSMode) -> Result<()>;
fn guard_execution(history: &mut History, mode: FSMode) -> Result<()>;
```

`admit_command` prints the initial report then calls the quiet guard. For a
writable abandoned run without an attempt it also prints the existing closure
transition. The guard ensures usable history, rejects unresolved writable
attempts, and quietly closes writable abandoned runs without attempts; dry runs
neither close nor resolve. Direct execution uses the guard; normal commands use
admission before planning. No additional guard state is introduced.

- [ ] Extend interruption tests to count startup report occurrences for dry-run
  rename/fix and multi-record replay with an unresolved attempt. Assert exactly
  one report per invocation, no effects, and unchanged attempt/run state for dry
  run. Add writable abandoned-run coverage asserting one initial report and one
  closure transition, without a duplicate from subsequent execution.
- [ ] Extend execution unit tests to call `execute_recorded` and `replay_record`
  directly with unresolved attempts: assert errors and no effects. Keep failure
  after install/commit poisoning and housekeeping failure tests proving that
  newly failed work still reports committed state and retains originals.
- [ ] Run `cargo test -p tfmt --test history_interruption` and execution unit tests
  before implementation; expect duplicate startup-report assertions to fail.
- [ ] Implement the split, update all three command entry points, and use quiet
  guards in apply/replay. Keep `finish_run`'s reload and fresh failure report.
  Preserve read-only display and resolution behavior. Document single startup
  reporting without claiming recovery or atomic execution.
- [ ] Run `cargo test -p tfmt` and `cargo test -p tfmt --features debug`.
  Expect all interruption, dry-run, partial replay, and artifact tests to pass.
  Commit as `Separate interruption reporting from execution guards`.

## Final verification and review

- [ ] Run `cargo +nightly fmt --all`, then `cargo test --workspace`.
- [ ] Run `cargo +nightly clippy --workspace --all-targets` and `cargo xtask lint`;
  fix actionable findings and rerun affected checks after changes. Record any
  unavailable toolchain/platform checks without claiming they passed.
- [ ] Search production Rust sources for `FsHandler`, `RenameCycleDetector`,
  dead verbosity plumbing, full-scan native tag maps, and runtime encoding
  parsing. Stored conversion parsing and logger verbose formatting must remain.
- [ ] Compare the final diff against all eight spec sections. Verify unchanged
  stored payload/schema contracts, independent raw fixture verification, ordered
  staging/cleanup, and intent-before-effect/artifact-retention guarantees.
- [ ] Record commands and results in
  `docs/superpowers/plans/2026-10-11-runtime-simplification-verification.md`.
  Review the complete branch before integration; do not publish or merge merely
  because the implementation plan is complete.
