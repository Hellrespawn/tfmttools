# Simplify SQLite history storage and execution interfaces

Date: 2026-10-10
Status: Approved

## Outcome and scope

Address all seven opportunities identified in the review of the SQLite history
changes through `9178c1d`. Reduce repeated work, implicit state, duplicated
invariants, and unnecessary interfaces while preserving observable command
behavior and durable interruption handling.

This specification builds on
[manual interruption handling](2026-10-10-manual-interruption-handling-design.md)
and [SQLite history and exact replay](2026-10-08-sqlite-exact-replay-design.md).
Their guarantees remain in force except for the explicit model and initial
schema simplifications below.

The SQLite schema remains undeployed. Amend the initial version-1 schema and its
snapshot directly. Existing development databases with the previous structure
must fail schema validation without being rewritten; users can explicitly move
them aside. Do not add a migration or import path for those databases.

Keep Rust 1.91.0 and edition 2024. Add no dependencies. Preserve CLI flags,
record ordering, displayed history, action payloads, patch format, and exact
byte replay. Internal Rust API changes are allowed; update every workspace
consumer in the same change.

## Approach

Use targeted changes within the existing core/filesystem/CLI boundaries. Core
continues to own history invariants and persistence; filesystem code owns
preparation, effects, and artifact housekeeping; CLI code coordinates execution
and reporting.

A complete database-only query API would remove the cache but spread fallible
queries into formatting and selection. A generic repository or execution-state
framework would introduce more machinery than these changes require. Retain
the record cache and direct execution flow, with smaller, explicit interfaces.

## 1. Refresh only records changed by a transaction

Replace `History::refresh`, which decodes every stored action after each
confirmation, with affected-record refreshes.

- Starting an apply run inserts and caches its new empty record.
- Starting replay does not change cached records.
- Confirming apply refreshes the active record. On its first confirmed action,
  also refresh records whose redo eligibility or state actually changes through
  supersession. Later confirmations do not repeat supersession work.
- Confirming undo or redo refreshes only its record.
- Closing a run or resolving an attempt refreshes its record and any records
  affected by first-action supersession.
- Beginning an attempt does not change the record cache.

Determine affected IDs inside the mutation transaction, using predicates shared
with the corresponding updates. Read the resulting records before commit, then
publish them to the cache only after a successful commit. A decoding or
validation failure before commit rolls back the mutation and leaves the cache
unchanged. Avoid a fallible cache reload after a successful commit.

If commit returns an error, treat durable state as uncertain. Invalidate cached
records and require a successful committed-state reload before exposing them or
allowing further mutations. Do not let infallible accessors expose a stale cache:
the session must be discarded or restored through the fallible load boundary.
The CLI error path must read committed attempt/run state for reporting and must
not infer an outcome from cached records or files. Preserve artifact retention
through this path. A dedicated internal unusable-session flag is acceptable if
needed to enforce the boundary; it is not persisted recovery state.

Keep full validation and loading at session open. This change removes repeated
decoding of unrelated history during mutation; it does not promise constant-time
append to a growing record. Reading the active record still decodes its actions.
Do not introduce incremental action-cache mutation unless a separate measured
need justifies its additional synchronization logic.

## 2. Make record progress concrete

Change `Record` progress fields to `applied_count: usize`, `complete: bool`, and
`redo_allowed: bool`. Accessors return those fields directly. Remove fallback
derivation from `RecordState`, optional-field defaults, and `set_progress`.

Construct a loaded record with all of its progress values in one constructor.
Validate cursor bounds and state/cursor consistency before returning it. A
preview record created by `Record::new` has all supplied actions applied,
`complete = true`, and `redo_allowed = true`; its ID remains absent. Stored
records always have an ID. Do not expand this cleanup into a separate preview
type unless another requirement makes that necessary.

Completeness, redo eligibility, and displayed state retain their distinct
meanings. A complete record can become ineligible for redo, and a partial record
can have applied actions available for undo. Do not derive all three from the
cursor or collapse them into a larger state enum.

Record serialization emits the three concrete progress fields. Deserialization
requires them; legacy JSON records without progress are no longer a runtime
contract. Update record fixtures and contract tests accordingly, retaining
strict action/metadata serialization, unknown-field rejection, optional tag
locator behavior, and executable-action conversion coverage. Database JSON
payloads for actions and metadata remain unchanged. Update generated schema
artifacts only where still applicable to the current SQLite tooling.

## 3. Share replay and attempt invariants

Provide pure record helpers for undo/redo eligibility and the next expected
action position for an operation. Use them in record selection, run admission,
attempt creation, and stored-attempt validation.

Preserve these rules:

- Undo is eligible when `applied_count > 0`; its next position is
  `applied_count - 1`.
- Redo requires `redo_allowed`, a cursor below action count, and a state other
  than superseded; its next position is `applied_count`.
- Apply appends at the current action count.
- A replay attempt must match the recorded action and patches at its position.
- Every attempt requires nonempty reporting paths and instructions.
- Tag actions require a valid patch pair; other actions must have none.

Create one shared attempt validator used before insertion and when reading
stored attempts. Separate validation of reporting details from validation that
depends on record contents. Patch format validation belongs to the patch module.
Keep checks at both input and storage boundaries; remove duplicate definitions
of the rules, not protection against malformed databases.

## 4. Represent run requests and execution context explicitly

Replace `begin_run(kind, optional_record_id, optional_metadata)` with a typed
request whose variants are `Apply { metadata }` and
`Replay { record_id, direction: HistoryMode }`. Apply cannot select an existing
record; replay cannot omit its record or accept apply metadata.

Keep `OperationKind` as the persisted apply/undo/redo discriminator. Define its
conversion from replay direction once. Avoid unifying it with `HistoryMode`,
which intentionally represents only undo and redo.

CLI execution must similarly carry either apply context or replay context with
a required record ID and direction. Remove the independently threaded kind and
optional record ID from preparation/execution helpers and remove the replay ID
unwrap. Derive persisted kind from the context at the persistence boundary.

Retain one shared attempt-and-confirm loop. A typed request is an interface
constraint, not a new durable state machine or a run object that owns history.

## 5. Remove redundant storage structures

Return `Record` directly from `database::read_record` and `read_records`; delete
the single-field `DatabaseRecord` wrapper and its mapping code.

Remove `records.position` from the initial schema and record queries. IDs already
increase in insertion order and records are neither reordered nor individually
deleted. Allocate the next ID as today, query records by ascending ID, and select
undo in reverse order. Preserve zero-based record IDs and allow gaps when
reading valid records; action positions remain contiguous and unchanged.

Update schema validation expectations, the SQL snapshot under `docs/history/`,
and schema fixtures. Keep application ID, user version, strict tables, foreign
keys, transactional schema creation, and durability settings. Replacing the
initial-schema migration helper is outside this cleanup's scope.

## 6. Reduce the in-memory query API

Replace undo/redo/all/limited wrappers with
`records_for(mode: HistoryMode, limit: Option<usize>) -> Vec<&Record>` and replace
`get_previous_record` with `previous_record() -> Option<&Record>`. These methods
operate on a successfully loaded, usable cache and return no `Result`.

`None` selects all eligible records; `Some(0)` selects none. Undo order is newest
first; redo order is oldest first. Use the shared eligibility helpers. Remove
the boxed iterator where ordinary branching and a shared collection helper
suffice; do not add a dependency or custom iterator enum for this purpose.

Formatting and previews borrow records. Callers that mutate history while
executing selected records explicitly clone the selected records before taking
a mutable borrow. Cloning at this boundary preserves the current sequential
replay behavior and avoids pervasive lifetimes or mutable cache references.

Update formatter error signatures where they become infallible. Preserve errors
from genuinely fallible output, loading, confirmation, and database operations.

## 7. Give patch persistence one home

Keep `ByteIdentity` and `BinaryPatchPair` public in `history::patch`. Move patch
validation, reading, writing, and action/patch association checks from
`attempt.rs` into that module as internal helpers. Filesystem patch generation
and application stay in the filesystem crate.

Share row decoding and conversion of lengths, hashes, format, and patch bytes
between attempt and confirmed patch storage. Keep separate explicit SQL for
the two tables; avoid dynamically assembled table names or a generic storage
abstraction. Use explicit INSERT column lists, including action insertion in
confirmation, so schema column order is not an implicit contract.

Use consistent error categories: malformed persisted data is a load error;
invalid mutation requests are mutation errors; failures to encode values for
persistence are save errors; SQLite failures retain their underlying database
error. Add record/position or attempt identity to patch errors where available.
Keep the existing public error enum unless these rules cannot be expressed
without changing it. Do not replace typed errors with a generic string wrapper.

## Invariants that must survive

Keep the session lock throughout writable work. For each action preserve:
prepare and retain artifacts, commit intent, execute, transactionally confirm
and clear intent, then perform action-specific housekeeping. At most one run
and one current attempt exist. No subsequent action starts before confirmation.

Manual resolution remains history-only. Ordinary failure and interruption stop
execution, preserve the confirmed prefix, and never resume or classify file
effects. Unresolved attempts block mutations. Partial runs remain undoable and
ineligible for redo. Starting an empty apply run does not supersede redo; its
first confirmed action does. Read-only commands and dry runs preserve their
current reporting and filesystem behavior.

Retain header/schema/integrity validation, stale-input rejection, exact patch
round-trip verification, and tag-original retention until durable confirmation.

## Delivery order

1. Make progress concrete and centralize eligibility/cursor/attempt validation.
2. Remove the storage wrapper and redundant record position; update snapshots.
3. Introduce typed run requests and CLI execution context.
4. Consolidate patch persistence and error mapping.
5. Replace query wrappers and migrate all workspace consumers.
6. Implement affected-record cache updates using the settled interfaces.

Each stage must preserve existing interruption behavior. Keep cache optimization
separate from schema and interface changes so its transaction semantics can be
reviewed directly. This is a delivery outline; detailed implementation tasks
belong in a subsequent plan.

## Verification and acceptance

Extend existing behavior tests rather than adding tests that mirror helper
implementations. Verify:

- Concrete progress survives persistence, partial undo/redo, and manual
  applied/not-applied resolution; previews initialize all fields explicitly.
- Record JSON requires progress while action and metadata contracts remain
  strict and preserve their existing representations.
- Selection and run admission agree for complete, partial, undone, redone,
  superseded, empty, and redo-disabled records, including zero/all limits.
- ID ordering produces the same undo/redo order without record position;
  noncontiguous action positions remain invalid.
- Fresh version-1 schema creation and read-only reopen succeed; old development
  schema shapes, foreign databases, bad versions, and invalid payloads are
  rejected without rewriting them.
- Both patch tables reject malformed format, headers, lengths, hashes, missing
  tag patches, and patches attached to non-tag actions. Exact replay tests pass.
- Apply, undo, redo, and both resolution outcomes update all affected cached
  records; cache contents agree with a fresh reopen after successful mutations.
- Supersession occurs on first confirmation only, including manual resolution
  as applied, and preserves any applied prefix of another record.
- A transaction rollback leaves the cache unchanged; commit errors do not expose
  stale records or allow further mutation without reloading committed state.
- Confirmation with many unrelated records does not read/decode their action
  payloads after supersession. Use scoped test instrumentation or query tracing,
  not a wall-clock threshold; do not add a production telemetry framework.
- Existing ordinary-failure, interruption, artifact retention, locking,
  read-only reporting, dry-run, and cleanup-only-error tests continue to pass.

Run focused core, filesystem, and CLI tests during delivery. Final gates are
`cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, and
`cargo xtask lint`. Update README, CHANGELOG, and relevant fixture documentation
for the initial-schema and record-contract changes. Remove obsolete interfaces,
fallback-specific tests, and documentation rather than retaining compatibility
wrappers solely to reduce the diff.

Completion requires coverage of all seven numbered sections, no full-history
refresh after each confirmation, no optional progress fields, no unchecked
replay-ID unwrap, and no change to the manual interruption guarantees.
