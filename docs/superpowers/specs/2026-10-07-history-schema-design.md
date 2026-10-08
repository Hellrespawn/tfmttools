# Concrete History Format and Legacy Migration

**Date:** 2026-10-07
**Status:** Superseded for storage and replay by
[SQLite history and exact replay](2026-10-08-sqlite-exact-replay-design.md).

## Goal

Give tfmt a stable, versioned history format with a small, concrete model.
Move history into `core`, separate stored actions from executable actions, and
upgrade existing JSON before decoding it. Maintain one current model and JSON
migration functions rather than complete Rust models for each old version.

Restore undo/redo for tag-edit histories affected by `ff1b5a7` (Require
canonical audio tag names with explicit aliases). Keep existing record ordering,
state transitions, IDs, timestamps, and action execution semantics.

## Model and ownership

Replace `History<A, M>` and `Record<A, M>` with concrete types in
`crates/core/src/history/`:

- `History`: file path, records, and runtime-only upgrade-backup state.
- `StoredHistory`: `schema_version` and records; a small document wrapper.
- `Record`: ID, `Vec<StoredAction>`, state, timestamp, and metadata.
- `RecordState`, `ActionRecordMetadata`, and `TemplateMetadata`: history-only
  types shared by persistence and callers.
- `StoredAction`, `StoredTagValueChange`, and `StoredTagValueKind`: stable
  storage representations of executable actions and their tag-change payloads.

The concrete Rust storage model is the source of truth for JSON. `History` and
`StoredHistory` use the same records; do not duplicate record, state, or metadata
models. Retire the redundant `ActionRecord` alias. Only actions and their
payloads have separate executable and stored representations.

Use ordinary conversion functions or `From`/`TryFrom` implementations between
`Action` and `StoredAction`. Match every variant without wildcard arms so adding
an executable action requires an explicit persistence decision. Convert applied
actions when recording a run; convert stored actions when replaying it. Loading
validates that all stored actions can be converted before accepting records.
CLI history formatting reads stored actions directly.

Retain straightforward concrete `load`, `save`, `push`, and record-selection
operations. No schema policy, codec, registry, associated-type framework, or
generic storage containers are needed. Use focused model, storage, persistence,
and migration files where useful.

Remove `crates/history` and its dependencies from `core` and the CLI. Update
workspace membership, lockfile, imports, xtask crate lists, and repository
architecture documentation. Move required history dependencies into `core`.
History IO does not use `fs`; executable filesystem operations remain in `fs`,
which already depends on `core`.

Audit Serde usage on executable actions and remove persistence-only derives
where they have no other consumers. History writes always use `StoredHistory`.

## Version 1 contract

One unsigned integer document version covers the complete format, independently
of application versions. New files contain `schema_version: 1`.

```json
{
  "schema_version": 1,
  "records": [
    {
      "id": 0,
      "actions": [
        {
          "type": "move_file",
          "source": "/music/a.mp3",
          "target": "/music/b.mp3"
        }
      ],
      "state": "applied",
      "timestamp": "2026-10-07T12:00:00+02:00",
      "metadata": {
        "template": {"type": "inline_template", "value": "path: $title"},
        "arguments": [],
        "run_id": "example"
      }
    }
  ]
}
```

### Actions and metadata

Use internally tagged enums (`#[serde(tag = "type")]`) for stored actions and
history template metadata. The discriminator and payload fields are siblings.
Reserve `type` for the discriminator.

| Stored action type | Required payload fields |
| --- | --- |
| move_file | source, target |
| copy_file | source, target |
| remove_file | path |
| make_dir | path |
| remove_dir | path |
| edit_tag_values | path, changes |

Stored paths are strings. Stored remove-file and directory variants have named
`path` fields; executable newtype variants can stay unchanged.

| Template metadata type | Required payload |
| --- | --- |
| file_or_name | value: string |
| inline_template | value: string |
| validation | value: string |

Change `TemplateMetadata` to named `value` fields and update its callers.
Replace its existing `Script` compatibility rename with the new explicit
`inline_template` name. Migration handles old `Script` objects; do not add
legacy aliases to the current decoder. Template payloads keep their exact
string contents and existing meanings.

Record state strings are `applied`, `undone`, `redone`, and `superseded`.
Stored tag-value kind strings are `text` and `locator`.

### Stable names and fields

Pin each storage field and variant with an individual explicit Serde name.
These names define the storage contract independently of executable Rust
identifiers; JSON does not embed Rust struct names.

Keep these field names:

- Document: `schema_version`, `records`.
- Record: `id`, `actions`, `state`, `timestamp`, `metadata`.
- Action: `type`, plus its payload fields from the table above.
- Tag change: `key`, `kind`, `old_value`, `new_value`, `old_encoding`,
  `new_encoding`.
- Record metadata: `template`, `arguments`, `run_id`.
- Template metadata: `type`, `value`.

Keep existing ID and timestamp representations, including the existing datetime
record type. Preserve optional/null behavior for IDs and encodings. Missing
encoding fields remain `None`, as supported since `7eccde3`; never infer old
encoding information. Reject unknown object fields to avoid silently dropping
history data on subsequent saves.

Canonical tag keys and encoding strings are storage identifiers too. Deriving
these names from dependency enum debug output is not part of the contract.

## Loading and migration

Use one migration entry point and one current-version constant:

```text
bytes -> JSON Value -> inspect version -> migrate if needed
      -> decode StoredHistory -> check action conversions -> accept records
```

For the initial implementation, direct dispatch is sufficient:

- Missing version or explicit integer 0: call `migrate_v0_to_v1`.
- Integer 1: decode the current model without migration.
- Any other value: fail with an invalid-version or unsupported-version error.

Reject null, strings, fractions, negative values, and integers outside the
supported range. Do not retry failed version-1 decoding as legacy history.
When version 2 is actually introduced, add its migration and sequential dispatch;
no migration framework is needed now.

Migration transforms JSON only. It does not touch files, execute templates,
change tags, or alter record states. Accept records only after the whole
migration, decoding, and conversion check succeeds. A failed load leaves live
records and source bytes unchanged and stops commands before applying actions;
it must not be treated as empty history.

### Version 0 to version 1

All existing unversioned history is version 0, including files written before
and after `ff1b5a7`. Migrate every record, including undone and superseded ones.

| Legacy representation | Version 1 representation |
| --- | --- |
| `{"MoveFile":{"source":"a","target":"b"}}` | `{"type":"move_file","source":"a","target":"b"}` |
| `{"CopyFile":{"source":"a","target":"b"}}` | `{"type":"copy_file","source":"a","target":"b"}` |
| `{"RemoveFile":"a"}` | `{"type":"remove_file","path":"a"}` |
| `{"MakeDir":"a"}` | `{"type":"make_dir","path":"a"}` |
| `{"RemoveDir":"a"}` | `{"type":"remove_dir","path":"a"}` |
| `{"EditTagValues":{"path":"a","changes":[]}}` | `{"type":"edit_tag_values","path":"a","changes":[]}` |
| `{"FileOrName":"name"}` | `{"type":"file_or_name","value":"name"}` |
| `{"Script":"text"}` | `{"type":"inline_template","value":"text"}` |
| `{"Validation":"text"}` | `{"type":"validation","value":"text"}` |

Rename legacy state strings `Applied`, `Undone`, `Redone`, and `Superseded` to
their explicit lowercase names, and tag kinds `Text`/`Locator` to `text`/`locator`.
Canonicalize tag-change keys as described below, then set the document version
to 1 after the transformation succeeds.

Transform only known structural positions. Preserve paths, template contents,
arguments, timestamps, IDs, values, encoding strings, record order, and action
order. Never recursively replace arbitrary strings. Require one recognized
legacy variant key and its correct payload shape; reject extra variant keys,
unknown variants, and conflicting payload `type` fields. Current-format decoding
rejects legacy action and template shapes in version-1 documents.

### Historical tag keys

`ff1b5a7` changed replay from permissive tag-key lookup to strict canonical names.
Restore compatibility in migration without relaxing the current tag parser.

Use a frozen literal mapping based on the accepted key set in
`ff1b5a7^:crates/core/src/item_keys.rs`: all 100 historical keys, five aliases,
and the special `AppleId3V2ContentGroup` spelling. Reproduce historical
case-insensitive flat, snake, and kebab spellings. Establish acronym boundaries
using the historical conversion algorithm when preparing the mapping and tests;
do not derive them from current dependency enum names or retain removed
case-conversion dependencies in production.

Examples include `TrackArtist`, `trackartist`, `track-artist`, and `TRACK_ARTIST`
becoming `track_artist`; `AlbumSort` becoming `album_title_sort_order`;
`DiskNumber` becoming `disc_number`; and `Title` becoming `track_title`.
Already canonical keys remain canonical, allowing mixed legacy/current keys
within the same version-0 document.

Unknown keys fail migration. Computed `date` is not a writable tag. Current
stored keys must be canonical even when runtime APIs accept aliases. Enforce
this and existing action semantics through the stored-to-executable conversion
check, with small explicit checks where required. Serde handles structural
validation; do not introduce a separate general validation framework.

## Persistence and errors

Read-only loading never rewrites history or creates backups. A subsequent
normal save writes version 1. Prepare and validate the complete output before
changing files.

Before the first overwrite of loaded version-0 history, preserve its exact
source bytes in `<history filename>.v0.bak` alongside the history file. Create
exclusively. Reuse an existing backup only if its bytes match; otherwise fail
without overwriting it or the source. Keep original bytes in runtime-only state
until the upgrade save succeeds.

Write to a temporary file in the destination directory, then atomically replace
the destination after the write succeeds. On a backup or write failure, preserve
the original history and report the error. A failed replacement may leave a
valid upgrade backup. This does not introduce concurrent-writer locking or a
crash-durability guarantee.

Keep existing `SaveErrorWithBackup` recovery for a destination that exists but
is not a file, using version-1 serialization for the recovery output. This
recovery file is distinct from the legacy upgrade backup.

Give errors enough context to identify unsupported versions, invalid documents,
failed migration locations (record/action/change), and IO failures. Never skip
unknown records or actions. Preserve existing missing-file behavior and record
selection/state transitions.

## Schema snapshot and compatibility checks

Generate deterministic JSON Schema from `StoredHistory` and check it into
`docs/history/schema-v1.json`. It is derived from the concrete Rust model,
not maintained as a second source of truth. Describe version 1 as a constant,
required fields, tagged unions, optional/null behavior, and unknown-field
rejection consistently with the decoder.

Provide a simple regeneration command and one automated snapshot comparison
in the normal test/check workflow. No PR-base comparison or schema-registry
machinery is needed. Production decoding uses Serde and action conversions,
without a JSON Schema validation engine.

An intentional published-format change requires a version bump, a migration,
an updated schema snapshot, and compatibility fixtures. Enforce that discipline
through review. The snapshot detects structural drift; historical fixtures
and replay tests detect semantic changes such as tag-key interpretation.

Use fixed historical JSON fixtures independent of the current serializer.
Focused checks cover:

- Every legacy action, state, template variant, and tag kind; optional encodings;
  representative pre-`ff1b5a7` histories and the historical key mapping.
- Current-format output and action conversions, preserving all record data,
  opaque strings, and ordering.
- Malformed/future versions, missing or unknown discriminators, incorrect
  payloads, unknown fields/keys, and noncanonical version-1 tag keys.
- Failed loads, unchanged files on read-only loads, exact upgrade backups,
  backup collisions, and safe write failures.
- Real undo/redo of a pre-change tag-edit record, checking values and recorded
  encoding, plus legacy filesystem-action replay.

## Implementation outline and documentation

1. Move history into `core` and replace generic containers with concrete types.
2. Define stored actions, tagged template metadata, and direct action conversions.
3. Add version dispatch, the legacy migration, backups, and atomic persistence.
4. Add schema/compatibility checks and update documentation.

Keep changes reviewable around these four steps; detailed execution planning
can follow approval of this spec.

Update README and CHANGELOG with migration-on-load, upgrade-on-save, backups,
and unsupported-version behavior. Replace the `ff1b5a7` recommendation to clear
old tag-key history with automatic compatibility support. Update repository
maps and task guidance for removal of the history crate. Do not imply that
legacy template languages are translated: replay uses stored actions, and
reuse of old template text retains existing restrictions.

Run focused core history-module and CLI checks, then `cargo test --workspace`,
`cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.

## Non-goals

Reusable generic history frameworks, duplicate history-only runtime/storage
models, complete historical Rust type sets, per-record versions, SQLite,
automatic template-language translation, downgrade writing, concurrent-writer
locking, and changes to action execution or undo/redo ordering.
