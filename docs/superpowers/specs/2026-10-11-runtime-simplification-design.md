# Simplify runtime models, tag inspection, and command plumbing

Date: 2026-10-11
Status: Approved

## Outcome and scope

Address all eight findings from the follow-up maintainability scan: stringly
typed runtime encodings, repeated native ID3 inspection, separate validation
inspection paths, the filesystem-mode wrapper, repeated interruption reporting,
inconsistent input-directory conversion, unreachable verbosity plumbing, and
unnecessary directory-collection allocations and planner wrappers.

Success means fewer definitions of the same behavior, smaller interfaces, and
clear error policies. Reduced line counts alone do not establish success.
Deliver targeted replacements within the existing workspace boundaries, updating
every consumer of changed internal interfaces.

This request authorizes a specification. Product implementation and its detailed
task plan are separate deliverables.

## Existing contracts

Build on these approved designs:

- [Project maintainability](2026-10-10-project-maintainability-design.md).
- [Manual interruption handling](2026-10-10-manual-interruption-handling-design.md).
- [History simplification](2026-10-10-history-simplification-design.md).

Preserve Rust 1.91.0, edition 2024, and the current dependency set. Preserve CLI
flags, template semantics, discovery order and extension matching, confirmation
policy, rename and cleanup ordering, exact-byte replay, and stored action and
metadata formats. Do not change the SQLite schema or binary patch format.

Keep history locking, cache invalidation, strict persisted-data validation,
collision and stale-input rejection, and tag-original retention. Execution
continues in this order: prepare, retain artifacts, commit intent, execute,
confirm history, confirm action-specific housekeeping. No SQLite transaction
spans filesystem effects.

No automatic recovery, filesystem outcome inference, new execution framework,
generic repository, persistent state, or dependency belongs in this change.

Intentional observable changes are limited to reporting an interrupted state
once per command and reporting validation-fix inspection failures instead of
silently skipping them. A fix with incomplete inspection fails before applying
any proposed tag changes. Earlier history admission may still close an abandoned
run without an attempt, under the existing contract.

## Approach and alternatives

Use small concrete helpers and existing types. Keep filesystem inspection and
effects in `fs`, pure metadata interpretation and runtime models in `core`, and
command policy and presentation in `tfmt`.

A cosmetic deletion-only pass would leave repeated metadata reads and invalid
runtime encoding values possible. A generalized validation/execution framework
would introduce additional interfaces and lifecycle states. Targeted extraction
and deletion covers the findings without either limitation.

## 1. Typed runtime tag encodings

In `crates/core/src/action/mod.rs`, change `TagValueChange` encoding fields,
builder arguments, and accessors to use `Option<TagTextEncoding>`. The enum is
small and copyable; accessors return optional values by value. Use the same type
in the CLI's `FieldFix` and any replacement inspection result.

Keep `StoredTagValueChange.old_encoding` and `new_encoding` as optional strings.
The conversion in `history/conversion.rs` is the explicit boundary:

- Runtime to stored: emit the enum's existing canonical `name()`.
- Stored to runtime: parse each supplied name, rejecting unknown and
  noncanonical names with the existing load-error context.
- Absent encodings remain absent; no defaults or additional spellings.

Filesystem writing converts typed values directly to Lofty's `TextEncoding`.
Remove runtime name parsing and branches that silently skip unknown encodings.
Retain independent stored-input validation and all four supported values:
`Latin1`, `UTF16`, `UTF16BE`, and `UTF8`.

This extends the earlier shared encoding-name contract to the runtime action
model. It does not merge executable and stored models or change serialization.
Tag-key typing and unrelated action-model redesign are outside scope.

## 2. Shared native ID3 inspection

Extract filesystem-facing native tag reading from the CLI into a focused `fs`
module. Keep format support explicit: validation remains MP3-only for its
existing ID3-encoding policy; candidate verification retains MPEG, WAV, and AIFF
support. Do not broaden validation discovery or extension matching here.

Expose fallible native reading, distinguishing a missing native tag from a read
or parse failure. Move the pure rule matching text/user-text frames to item keys
into a focused core helper. Preserve both existing matching conditions for user
text: description equals the mapped ID, or the description maps to the item key.

The matcher must support borrowing all matching frames/encodings. It must not
clone an entire tag to look up one item. Callers retain their policies:

- Validation uses the first matching frame, preserving current semantics.
- Candidate verification requires at least one matching frame and verifies all
  matching frames against the requested encoding.
- Encoding modification updates all matching frames as today.

Read native candidate metadata once per candidate when encoding verification
is needed, then reuse it across requested changes. Keep the post-write read:
verification must inspect persisted output, not the in-memory requested tag.
Do not weaken source-value checks, locator preservation, patch verification,
or candidate verification to save reads.

Keep the fixture harness's raw byte-level ID3 inspection independent. It can
share encoding names, but must not verify output solely through the production
reader, matcher, or writer.

## 3. One per-file validation inspection path

Restructure `commands/validate.rs` around one per-file inspection operation that
loads generic audio metadata and, when the selected policy needs it, native ID3
metadata. Return concrete findings and proposed tag changes from that inspection.
Check and fix commands consume those results rather than maintaining separate
file-reading and field-selection pipelines.

Keep character sanitization and the non-ASCII UTF8-to-UTF16 predicate shared
between reporting and fixing. Preserve text versus locator distinctions,
canonical action keys, current issue labels, and first-frame validation policy.
Check-only `validate` still supports all checks; bare `validate --fix` remains
rejected. Do not add a validation-rule registry or trait hierarchy.

For ID3 fixes, remove the whole-input `HashMap` of native tags. Inspect one file,
derive its changes, and release its metadata before inspecting the next file.
Collect planned actions and diagnostics across the scan; do not execute inside
the inspection loop. Progress reflects the number of selected files and is
finished or cleared on failure as well as success.

Error policy:

- Check-only mode collects read/parse errors alongside findings and returns
  `ValidationFailed`, as it does for other validation failures.
- Fix mode collects and prints read/parse errors, returns an operational error,
  and executes no planned fixes if any required inspection failed.
- A successfully read file without an applicable native ID3 tag is not a read
  failure and produces no ID3 fix.
- Native metadata is only required for checks selected by the command; a
  characters-only operation does not fail because of an unused native reader.
- No findings after a complete successful fix inspection retains the existing
  no-change message. An incomplete inspection must not print that message.
- Dry-run fixes obey the same inspection/error policy and never mutate files
  or history. All traversal must already have succeeded before inspection.

Execution still prepares and validates each candidate against then-current
files. A successful inspection is not permission to bypass stale-input checks.
This policy prevents silent skips; it does not promise atomicity once recorded
execution begins.

## 4. One filesystem mode

Add `FSMode::is_dry_run(self) -> bool` in `core::util`. Delete `FsHandler`, its
module/export, and construction at the CLI boundary. It currently owns no
filesystem operations beyond its duplicate mode field.

Command entry points derive the mode from their existing `TFMTOptions`.
Execution helpers that do not need the full options object accept `FSMode`.
Remove session fields and parameters used solely to carry `FsHandler`; update
all tests and workspace consumers. Do not replace it with another wrapper or
pass the full CLI options into filesystem helpers.

Keep `get_longest_common_prefix`, currently colocated with the wrapper, in a
focused filesystem path-helper module and update its imports. Remove only the
wrapper's responsibilities, not this used utility.

## 5. Separate interruption presentation and admission

Split the current `check_interrupted` responsibilities into a fallible report
formatter, a command-boundary presentation/admission step, and a quiet mutation
guard. Reuse history's existing attempt/run queries and invariants; do not add
an admission token, cached checked flag, or new session state machine.

At the start of rename, validation fix, and undo/redo, report existing interrupted
state once. Writable commands reject unresolved attempts and may close an
abandoned run with no attempt, reporting that transition once. Dry runs report
without closing or resolving anything. Check-only validation retains its current
history-independent behavior.

Keep quiet checks at apply/replay execution boundaries so direct internal callers
cannot bypass unresolved-attempt blocking or cache usability. Those checks must
not duplicate startup output or automatically execute remaining actions.
The core mutation boundaries continue to enforce their own invariants.

Preserve failure reporting in `finish_run`: after an execution failure, reload
committed history before reporting an unresolved attempt. This describes newly
failed work and must not be suppressed as a duplicate startup report. Preserve
the original error and retained-artifact diagnostics.

Read-only history display and explicit resolution keep their existing report
and mutation policies. Multi-record replay reports startup state once; each
record still receives the quiet execution guard.

## 6. Consistent input-scope normalization

Introduce a small shared conversion for input directory and recursion depth in
`cli/options.rs`. Rename and validation use it without merging their unrelated
option structures or adding a new configurable policy framework.

Resolve the current directory once per conversion. A missing input directory
selects it; a relative supplied path is joined to it; an absolute supplied path
remains absolute. Verify the resulting directory using the existing helper,
which permits nonexistent paths and rejects an existing file. Do not canonicalize
paths, resolve symlinks, or require directory existence as part of this cleanup.
Default recursion depth remains 4; explicitly supplied zero remains zero.

Keep template and trash-directory conversion unchanged. Physical targets and
discovery order must remain the same. Preserve current user-facing relative-path
presentation where normalization would otherwise change it, using the existing
display-prefix conventions rather than introducing persisted path state.

## 7. Remove unreachable verbosity plumbing

Remove `TFMTOptions.verbosity`, its accessor, and its constant-zero initialization.
Remove the unreachable verbose selection branch in `show_history`.
Do not add a new CLI flag to justify the field.

Audit `HistoryFormat::Verbose` and its formatter branches after removing that
caller. Remove production variants, builders, and branches that have no remaining
production use; update tests that only exercise deleted unreachable behavior.
Keep formatter functionality still used by other commands. Default displayed
history remains unchanged.

## 8. Simplify directory collection and planner helpers

In `RenameAction::intermediate_directories`, iterate directly over target parent
ancestors, omit empty paths, and collect into a `BTreeSet` before producing the
ordered vector. Remove temporary target-reference and per-parent vectors and the
`HashSet`-then-sort pipeline. Preserve unique lexical ordering, inclusion of the
absolute root, and relative-path behavior.

Replace `RenameCycleDetector` with a private
`needs_temporary_staging(&[RenameAction]) -> bool` function. Keep the existing
case-insensitive source-set membership rule. Despite the old type name, it
detects overlapping paths including chains and case-only renames; do not replace
it with a narrower graph-cycle algorithm or alter staging policy.

Keep the stateful staging planner and rename planner unless their own state is
demonstrably unnecessary. This item authorizes removal of the identified
one-method slice wrapper, not a general planner rewrite.

## Delivery order

Each stage must leave the workspace buildable and include its relevant tests
and documentation:

1. Delete unreachable verbosity and simplify directory collection/staging helper.
2. Replace `FsHandler` and normalize shared input-scope conversion.
3. Type runtime encodings while preserving stored payloads.
4. Extract native ID3 reading and matching, then migrate candidate verification.
5. Unify per-file validation inspection and implement the explicit error policy.
6. Separate interruption presentation from quiet guards.

Stages 4 and 5 depend on the runtime encoding interface from stage 3. These are
delivery boundaries, not a detailed implementation plan or approval to execute.

## Acceptance and verification

Use existing behavior tests wherever possible. Add targeted regression coverage
for changed contracts rather than tests that mirror mechanical helper bodies.

- All four encodings round-trip through runtime/stored conversion; unknown and
  noncanonical stored names fail. Stored JSON and the SQL snapshot are unchanged.
- Native frame tests cover standard text, user text, no match, multiple matches,
  and differing encodings. Validation retains first-match semantics; candidate
  verification rejects any mismatching frame. MPEG/WAV/AIFF verification support
  and locator preservation remain covered independently by fixture assertions.
- ID3 validation reads native metadata once per file; candidate verification
  reads it once per candidate. Establish this through local structure or scoped
  test instrumentation, without timing thresholds or production telemetry.
- Check and fix agree on eligible character and encoding changes. A scan with a
  valid fixable file and an unreadable/malformed file reports failure and leaves
  both tags and recorded actions unchanged. Exercise dry run too. Inject errors
  deterministically instead of relying exclusively on permission bits.
- Relative, absolute, omitted, nonexistent, and existing-file input paths behave
  consistently for rename and validation; default/zero depth and display paths
  retain their stated contracts. Include paths containing spaces and symlinks
  where supported, without canonicalizing away user path components.
- Interrupted apply and multi-record replay produce one startup report; direct
  execution remains guarded. Dry runs do not close abandoned runs. Existing
  cache-poisoning, failure-after-effect, backup-retention, partial-replay, manual
  resolution, and post-confirmation housekeeping tests continue to pass.
- History's default output is unchanged after verbosity deletion. Directory
  collection preserves ordering and roots; swaps, chains, longer cycles,
  case-only renames, forced copies, and cleanup planning retain existing behavior.
- Search confirms no remaining `FsHandler`, unreachable verbosity plumbing,
  runtime encoding string parsing, full-scan native-tag map, or
  `RenameCycleDetector` wrapper.

Run affected crate tests during delivery. Final gates are `cargo test --workspace`,
`cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
Run fixture integration tests for validation diagnostics, interruption reporting,
and path display. Report unavailable platform/toolchain checks explicitly.

Update README.md, CHANGELOG.md, relevant fixture documentation, and crate task
maps alongside user-visible changes and removed exports/modules. Final review
must verify that all eight findings are addressed and no persisted contract,
history safety check, independent raw fixture verification, or candidate
verification has been weakened.
