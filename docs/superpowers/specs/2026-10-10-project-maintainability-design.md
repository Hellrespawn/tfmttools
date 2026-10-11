# Simplify execution, command orchestration, and verification

Date: 2026-10-10
Status: Approved

## Outcome

Address the eight findings from the project maintainability review: competing
filesystem execution APIs, misleading planning names and messages, incomplete
test discovery, duplicated audio discovery, scattered process exits, optional
history plumbing, repeated encoding conversions, and obsolete scaffolding.

Success means fewer independently maintained behavior definitions, clearer
planning/execution boundaries, and checks that automatically cover new tests.
Line-count reduction alone is insufficient. Preserve Rust 1.91.0, edition 2024,
the existing crate responsibilities, and the dependency set.

The request authorizes this specification. Implementation and its detailed
task plan are separate deliverables.

## Existing contracts and scope

This specification builds on the approved
[history simplification](2026-10-10-history-simplification-design.md) and
[manual interruption handling](2026-10-10-manual-interruption-handling-design.md)
designs. Their transaction, cache, patch, and interruption contracts remain in
force. The dependency simplification work is independent: do not rewrite the
template parser, confirmation implementation, or snapshot system here.

Preserve CLI flags, template semantics, action ordering, cleanup selection,
confirmation policy, history serialization, SQLite schema, and exact-byte tag
replay. Internal Rust interfaces may change; update all workspace consumers.
Public filesystem exports removed below are intentional Rust API changes and
must be called out in CHANGELOG.md. Do not retain a second implementation just
to preserve those exports.

Intentional observable changes are limited to:

- Cleanup output distinguishes proposed work, dry runs, and successful runs.
- Filesystem traversal errors fail a command rather than silently truncating
  discovery. Fixture discovery likewise fails visibly.
- Normal command completion and failure return through the binary boundary,
  allowing owned guards to drop.
- Standard xtask test commands discover every CLI test target automatically.

Do not introduce a new executor framework, recovery engine, generic repository,
plugin system, dependency, persistent field, or schema migration.

## Approach and alternatives

Use incremental replacements within existing crate boundaries. Retain prepared
filesystem actions and the shared CLI attempt-and-confirm loop as the execution
path. Extract only the duplicated audio discovery pipeline and encoding
conversion contract, and remove obsolete APIs after consumers move.

A minimal cosmetic cleanup would leave competing execution behavior and test
omissions intact. A broad redesign around a generic execution service would add
interfaces and state beyond these findings. Targeted replacement provides the
benefits without either limitation.

## 1. One filesystem action execution path

`crates/fs/src/action/executor.rs` currently combines pure planning with direct
execution through `ActionHandler`; the CLI instead executes prepared actions
through `crates/tfmt/src/history/execution.rs`. Their move, copy, undo, and
directory semantics can drift independently.

Replace the public `ActionExecutor` with a planning interface:

```rust
pub fn plan_actions(
    rename_actions: Vec<RenameAction>,
    move_mode: MoveMode,
) -> FsResult<Vec<Action>>;
```

Keep rename staging and cycle planning internal. Planning can inspect the
filesystem for existing directories, temporary paths, and device boundaries;
it must not modify files, create directories, or write history. It needs no
`FsHandler` reference. Preserve the existing order of staging and directory
actions, including explicit copy followed by removal for forced-copy and
cross-device moves.

Remove `ActionHandler`, its exports, and the direct `apply_rename_actions`,
`apply_actions`, and `remove_directories` APIs. Migrate their behavior tests to
the public planner plus prepared action preparation/execution/confirmation.
Tests that promise history semantics must exercise recorded CLI execution,
rather than simulate confirmation with direct filesystem calls.

Keep `FsHandler` operations that still have real callers. Audit its former
handler-only methods and result types and remove those without consumers;
do not expand this into redesigning unrelated file utilities.

Prepared actions continue to own validation, effects, synchronization, and
action-specific housekeeping. CLI execution continues to own durable intent
and confirmation. Preserve the exact order: prepare, retain artifacts, commit
intent, execute, confirm history, confirm housekeeping. Never add automatic
recovery or allow tests to bypass required copy verification and durability.

## 2. Name and report planning honestly

In `crates/tfmt/src/commands/rename/`, rename `apply` to `confirmation` and
its entry point to `confirm_and_plan`. Replace `RenameExecutionResult` with
`RenameDecision`, whose variants are `Confirmed { actions, unchanged_files,
metadata }`, `NothingToRename`, and `Aborted`. The no-work variant has no payload.

Rename `finish.rs` to `cleanup.rs`, and rename helpers to reflect their effects:
`handle_remaining_files` becomes `append_cleanup_actions`, `execute_cleanup`
becomes `plan_cleanup_actions`, and `move_files` becomes `plan_file_moves`.
Avoid unrelated renames or a general phase/type-state abstraction.

Cleanup discovery, selection, preview, confirmation, and action planning happen
before recorded execution. Preserve the policy of skipping cleanup when its
protected-path or remaining-path resolution fails, with an explicit diagnostic.
The primary rename can proceed under that existing policy. Declining cleanup
still allows the confirmed primary rename to proceed.

Before execution, previews use prospective language, such as “Files to delete”
or “Would delete” for dry runs. Remove “Deleted” and “Removed empty folders”
messages from planning. Directory removal can legitimately be a no-op for a
nonempty directory, so do not infer actual removal from a planned action count.

After successful `execute_recorded`, print one run-completion message and the
existing saved-history message for writable runs. Do not print a whole-run
success message on execution or post-confirmation housekeeping failure.
Existing interruption reporting describes the confirmed prefix on failure.
Dry runs print prospective output and never claim a saved or executed run.
No persistent cleanup progress or additional execution summary state is needed.

## 3. Let Cargo discover CLI tests

Replace the manually maintained CLI integration target list in
`xtask/src/main.rs`. `test-cli` invokes `cargo test -p tfmt`; `test` invokes
`cargo test --workspace`. Both may pass `-- --nocapture` consistently if needed
for the custom harness. Each invocation must include unit tests, automatically
discovered integration tests, the custom harness, and applicable doc tests.

Keep `test-integration` as the focused custom-harness command, including its
existing argument forwarding. Keep focused core/filesystem tasks. Remove the
obsolete test argument constants and repeated orchestration and update help.

Acceptance requires evidence that `confirmation.rs` runs under both broad
tasks. A newly added conventional integration target must need no xtask edit.
Do not add a second test-target discovery mechanism.

## 4. Share audio discovery and propagate traversal errors

Add a focused CLI helper in `crates/tfmt/src/commands/audio_discovery.rs`:

```rust
pub(crate) fn gather_audio_paths(
    app_options: &TFMTOptions,
    options: &PathIteratorOptions<'_>,
) -> Result<Vec<Utf8PathBuf>>;
```

It owns the common progress spinner, total/found counters, audio predicate,
debug delay, and successful completion. Rename and validation construct their
existing traversal options and use this helper. Preserve traversal order,
depth, ignore behavior, and extension matching.

Propagate traversal failures with directory context. Ensure the spinner is
finished or cleared on both success and failure. Complete discovery before
planning or applying mutations; an incomplete scan must not become a successful
partial rename or validation fix. Existing history interruption checks may
still run before discovery under their current contract.

In `crates/test-cli/src/runner.rs`, replace fixture-discovery `.flatten()` with
fallible collection and attach the fixture directory to errors. Preserve sorted
case names and the separate error for an empty case directory.

Audit other `PathIterator` consumers for silent error dropping. Where their
output drives mutation or test selection, make them fallible and propagate
errors through consumers. Do not change explicitly best-effort reporting or
cleanup-skipping policy without documenting why it belongs to this scope.
Avoid a configurable discovery-policy framework.

## 5. Return normal command status to the binary

Introduce a small CLI command outcome with `Success` and `ValidationFailed`.
Validation findings return `ValidationFailed` after printing their existing
report; operational errors continue to return `Err`. Other successful commands
return `Success`. Update command dispatch and `cli::run` accordingly.

`cli::run` returns `Result<ExitCode>`. In normal builds it renders operational
errors with the existing command-specific formatter and returns failure. In
debug builds it preserves rich error propagation. `main` returns
`color_eyre::Result<std::process::ExitCode>` and delegates to `cli::run`.
Validation failure remains exit status 1 without an extra operational-error
message. Successful commands remain exit status 0.

Remove normal-path `process::exit` calls from command handlers and `cli::run`.
Owned history sessions, progress UI, and tracing guards drop on return. Preserve
existing Clap help/argument handling. Signal and panic hooks are outside this
normal-exit change; retain their interruption behavior rather than introducing
signal coordination or a cancellation state machine.

## 6. Require history in validation fix helpers

Select check versus fix at the validation entry point. Load and check history
only in the fix branch, preserving the existing dry-run load mode. Pass
`&mut History` to `fix_characters`, `fix_id3_encoding`, and
`apply_and_store_fix`; remove the optional-history arguments and `expect`.

Dry-run fixes keep their existing prospective reporting and perform no history
mutations. Empty fixes still report that nothing changed. Check-only validation
does not acquire a writable history session. Keep one recorded execution loop;
do not create a second validation executor.

## 7. One encoding-name contract

Add a small core module for a typed tag text encoding with exactly four values:
`Latin1`, `UTF16`, `UTF16BE`, and `UTF8`. Give it explicit string parsing,
canonical naming, and conversion to/from Lofty's `TextEncoding`. Do not derive
a parser that accepts additional spellings or silently defaults unknown values.

Use this contract in CLI validation, filesystem tag editing, stored-history
validation, and test reporting. Keep stored action encoding fields as their
current optional strings, preserving serialized payloads and current validation
boundaries. The type represents supported values locally; it does not require
a schema or action-model migration.

Unknown or noncanonical stored encodings remain load errors. Preserve existing
behavior for unsupported Lofty values at each caller and document any such
case explicitly rather than mapping it to a supported encoding.

Keep the test harness's independent raw ID3 frame inspection. Sharing the
encoding-name vocabulary must not replace byte-level verification with the
production writer or its assumptions.

## 8. Remove leftovers and align documentation

- Remove `run_cargo_with_env`: its only caller passes an empty environment and
  its body sets each variable twice. Construct the Cargo command directly.
- Remove same-name field `serde(rename)` attributes in history models. Use
  enum-level `rename_all = "snake_case"` where it produces exactly the existing
  names; retain real exceptions, defaults, strictness, and tagged layouts.
- Update root guidance from `docs/history/schema-v1.json` to the SQL snapshot,
  and filesystem guidance from `src/fs.rs` to `src/fs_handler.rs`.
- Update crate task maps for moved/removed execution and rename modules.
- Update README.md, CHANGELOG.md, and relevant fixture documentation for the
  visible output, traversal-error, and exit-handling changes. Update affected
  output expectations without weakening assertions.

Keep this cleanup mechanical. Do not merge executable and stored action models
or remove strict database/payload validation to reduce conversion code.

## Delivery sequence

Deliver independently reviewable changes in this order:

1. Cargo test discovery and obsolete xtask scaffolding.
2. Filesystem planner extraction and removal of direct action execution APIs.
3. Rename planning names, no-work payload removal, and truthful output.
4. Shared discovery and explicit traversal-error handling.
5. Normal exit outcomes and validation's required history arguments.
6. Shared encoding conversions, serialization cleanup, and guidance updates.

Each change must leave the workspace buildable. Update its documentation and
tests with the change rather than postponing all documentation to the end.
The final detailed implementation plan can split these into smaller tasks;
this sequence defines dependencies, not authorization to implement.

## Acceptance and verification

Use behavior tests for changes that affect execution or observable output:

- Planner tests cover swaps, longer cycles, chains, existing directories,
  case-only renames, forced copies, and supported cross-device behavior without
  planning effects. Execute/replay those plans through prepared actions.
- Existing history failure and interruption tests continue to prove durable
  intent, confirmed-prefix preservation, unresolved-attempt blocking, exact-byte
  tag replay, and retained originals. Preserve collision and stale-input tests.
- Cleanup tests cover accepted/declined cleanup, dry runs, no-op directory
  removal, and execution failure after planning. No premature success output.
- Discovery tests inject a traversal failure deterministically and prove that
  rename/fix perform no file effects from an incomplete scan. Do not rely solely
  on permission-bit tests that behave differently under privileged users.
- Fixture discovery surfaces traversal failure rather than reporting an
  apparently complete set. Valid case ordering remains stable.
- CLI tests cover success, validation findings, and operational failures with
  their expected statuses and diagnostics in normal/debug configurations.
  Verify return-based cleanup with an owned drop guard or observable logger
  flush, without making timing-sensitive log assertions.
- All four encoding names round-trip; invalid names remain rejected. Existing
  serialized action/metadata contract tests and the SQL snapshot stay unchanged.
- Broad xtask runs include the confirmation target and custom fixture harness.

For mechanical deletions and documentation edits, use existing coverage and
inspection rather than tests that merely duplicate the implementation.

Run affected crate tests during each change. Final gates are
`cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, and
`cargo xtask lint`. Also run `cargo xtask test` and `cargo xtask test-cli` when
verifying their revised orchestration, and focused fixture tests for changed
output. Record any unavailable toolchain/platform checks accurately.

The final review must identify the removed competing execution behavior,
duplicated discovery pipeline, manual test list, optional-history panic, and
encoding mappings. Confirm no additional persisted state, schema changes,
dependencies, automatic recovery, or weakened verification were introduced.
