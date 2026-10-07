# Concrete History Format and Legacy Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce concrete, versioned history in core and restore legacy tag-edit replay through JSON migration with safe upgrade persistence.

**Architecture:** Move the existing record operations into core without changing selection or state transitions. Separate executable actions from stable stored actions, migrate JSON before decoding the current model, and validate all conversions before accepting a document. Keep source bytes in runtime state until an atomic upgrade save succeeds.

**Tech Stack:** Rust 1.89.0, edition 2024; Serde/serde_json, camino, chrono, fs-err, thiserror, tracing; add tempfile for atomic replacement and schemars for schema generation (select releases compatible with the MSRV).

**Spec:** `docs/superpowers/specs/2026-10-07-history-schema-design.md`, introduced by `a3732c5633c217cb1a9b87fcf4373d140c98a82d`.

## Global Constraints

- “New files contain `schema_version: 1`.”
- “History IO does not use `fs`; executable filesystem operations remain in `fs`, which already depends on `core`.”
- “Match every variant without wildcard arms so adding an executable action requires an explicit persistence decision.”
- “Reject unknown object fields to avoid silently dropping history data on subsequent saves.”
- “Missing encoding fields remain `None`, as supported since `7eccde3`; never infer old encoding information.”
- “Keep existing ID and timestamp representations, including the existing datetime record type.”
- “Read-only loading never rewrites history or creates backups.”
- “Before the first overwrite of loaded version-0 history, preserve its exact source bytes in `<history filename>.v0.bak` alongside the history file.”
- “This does not introduce concurrent-writer locking or a crash-durability guarantee.”
- No generic history framework, complete historical Rust models, schema validation engine, template translation, or changes to action execution/order.

## Review Focus

1. A bad action late in an undone or superseded record must reject the entire load before any replay; Task 3 tests this.
2. Tag-like text in paths, values, arguments, and templates must remain byte-for-byte identical as strings; Task 3 tests structural-only transformation.
3. Acronym/digit spellings, especially `AppleId3V2ContentGroup`, must follow historical case boundaries; Task 3 freezes and tests the historical mapping.
4. A backup collision or failed replacement must leave the original source intact and permit a safe retry; Task 3 tests backup and write failures.
5. Multiple records with dependent filesystem actions must retain reverse undo and forward redo order; Tasks 1 and 4 test selection and real replay.

---

## File and interface map

Replace `crates/core/src/history.rs` with the directory below; remove `crates/history/` after updating consumers.

| File | Responsibility |
| --- | --- |
| `crates/core/src/history/mod.rs` | Public concrete history exports; existing `HistoryMode` and `LoadHistoryResult` |
| `crates/core/src/history/model.rs` | `Record`, `RecordState`, `ActionRecordMetadata`, named-field `TemplateMetadata` |
| `crates/core/src/history/history.rs` | Runtime `History`, push/selection/state/removal operations |
| `crates/core/src/history/stored.rs` | `StoredHistory`, `StoredAction`, `StoredTagValueChange`, `StoredTagValueKind`; explicit Serde contract |
| `crates/core/src/history/conversion.rs` | Exhaustive action/payload conversions and canonical-key/encoding checks |
| `crates/core/src/history/persistence.rs` | Load, output validation, upgrade backup, atomic save, recovery output |
| `crates/core/src/history/migration.rs` | Version dispatch and structural v0-to-v1 JSON transformation |
| `crates/core/src/history/legacy_tag_keys.rs` | Frozen literal historical spelling-to-canonical mapping |
| `crates/core/src/history/error.rs` | Existing public errors plus contextual decoding/conversion/migration diagnostics |
| `crates/core/src/history/schema.rs` | Deterministic schema generation from `StoredHistory` |
| `crates/core/tests/history_*.rs` | Contract, migration, persistence, and schema tests |
| `crates/core/tests/fixtures/history/` | Hand-authored v0/v1 JSON and frozen historical key expectation data |
| `crates/tfmt/tests/history_compatibility.rs` | Actual CLI replay and failed-load protection |
| `docs/history/schema-v1.json` | Generated schema snapshot |

Keep `History` runtime-only, with `path: Utf8PathBuf`, `records: Vec<Record>`, and `upgrade_source: Option<Vec<u8>>`. `StoredHistory` contains only `schema_version` and the same `Vec<Record>` type. `Record` retains private fields and existing accessors; its actions are `Vec<StoredAction>`, ID is `Option<usize>`, and timestamp is `DateTime<Local>`.

The four tasks are sequential reviewable commits. Task 1 establishes ownership while retaining the legacy wire shape temporarily; Task 2 establishes v1 storage and conversion, still without legacy loading; Task 3 completes compatibility and safe persistence; Task 4 adds schema enforcement and end-to-end evidence. Do not release an intermediate commit.

### Task 1: Move concrete history ownership into core

**Files:** Create `history/{mod,model,history,persistence,error}.rs`; remove `crates/core/src/history.rs` and `crates/history/`; modify root `Cargo.toml`, `Cargo.lock`, `crates/core/Cargo.toml`, `crates/tfmt/Cargo.toml`, `crates/core/src/lib.rs`, and all history consumers listed below. Create `crates/core/tests/history_operations.rs`.

**Interfaces:**
- Produces `History::new(path: Utf8PathBuf) -> Self`, `load(&mut self) -> Result<LoadHistoryResult, HistoryError>`, `save(&mut self) -> Result<(), HistoryError>`.
- Initially `push(&mut self, actions: Vec<Action>, metadata: ActionRecordMetadata) -> Result<(), HistoryError>` and concrete `Record` action accessors use executable actions; Task 2 replaces their storage internally.
- Retains selection methods with `Result<Vec<Record>, HistoryError>`, `get_previous_record() -> Result<Option<Record>, HistoryError>`, `set_record_state(record: Record, state: RecordState) -> Result<Record, HistoryError>`, `remove`, `records`, and `is_empty`.

- [ ] Add failing tests named `undo_is_reverse_and_redo_is_forward`, `push_supersedes_only_undone_records`, `ids_follow_existing_record_count`, and `state_updates_require_unique_saved_id`. Assert undo selects Applied/Redone newest first, redo selects Undone oldest first, limits including zero are respected, pushing retains all records and supersedes Undone only, and missing/duplicate/null IDs retain existing mutation errors. Construct unusual IDs with literal JSON, without introducing new ID policy.
- [ ] Run `cargo test -p tfmttools-core --test history_operations`; expect failure because concrete `History`/`Record` do not yet exist in core.
- [ ] Move operations and errors from the history crate; replace generic containers with concrete types. Move chrono with Serde support, fs-err, and serde_json into core production dependencies. Preserve missing-file behavior, timestamps, and recovery-error behavior. Keep the legacy serialization temporarily so existing tests still exercise the ownership move.
- [ ] Replace `ActionRecord` with `Record` and history-crate imports/generic signatures in `crates/tfmt/src/history/{mod,formatter}.rs`, `cli/args.rs`, `commands/{undo_redo,show_history,clear_history,validate}.rs`, `commands/rename/{finish,template_resolution,planning,session}.rs`, and `crates/tfmt/tests/path_templates.rs`. Remove the workspace member/dependencies and regenerate the lockfile. Audit xtask for explicit history crate lists; current lists operate on the workspace and need no removal.
- [ ] Update ownership references in `README.md`, `docs/agent-map.md`, root `AGENTS.md`, and `crates/core/AGENTS.md` in this commit. Follow writing-for-agents when editing agent guidance.
- [ ] Run `cargo test -p tfmttools-core --test history_operations`, `cargo check --workspace`, and `cargo test -p tfmt`; expect success. Run `rg -n 'tfmttools.history|crates/history|ActionRecord\b' Cargo.toml Cargo.lock crates README.md AGENTS.md docs/agent-map.md`; expect no obsolete references.
- [ ] Commit: `Move concrete history into core`.

### Task 2: Define the v1 wire contract and action conversions

**Files:** Create `history/{stored,conversion}.rs`, `crates/core/tests/history_contract.rs`, and fixed fixtures `v1-all-variants.json` and `v1-null-optionals.json`; modify Task 1 history files, `crates/core/src/action/mod.rs`, CLI formatter, replay, template-resolution, validation, and path-template tests.

**Interfaces:**
- Produces `pub const CURRENT_SCHEMA_VERSION: u64 = 1` and `StoredHistory { schema_version: u64, records: Vec<Record> }` with construction restricted to the current version for saves.
- Produces `impl From<&Action> for StoredAction` and `impl TryFrom<&StoredAction> for Action { type Error = HistoryError; }`; analogous payload conversions use existing `TagValueChange` accessors and constructors.
- `History::push` continues consuming `Vec<Action>` and converts at the recording boundary; `Record::actions() -> &[StoredAction]` and `iter() -> impl DoubleEndedIterator<Item = &StoredAction>`.
- `TemplateMetadata::{FileOrName, InlineTemplate, Validation}` each has `value: String`.

- [ ] Add failing contract tests. Against fixed JSON, assert exact sibling `type`/payload shape for all six actions, three templates, four states, and two tag kinds. Assert paths are strings, `id: null` round-trips, omitted encodings become `None`, and null/string encodings retain their values. Compare a full document against `v1-all-variants.json` as JSON values; separately assert action/record array order and all metadata contents.
- [ ] Add conversion tests for every action and both tag kinds, comparing paths, keys, values, and encodings after round-trip. Add table-driven rejection assertions for `artist`, `TrackArtist`, `date`, unknown keys, and unknown encoding identifiers; `track_artist` and encodings `Latin1`, `UTF16`, `UTF16BE`, `UTF8` must succeed. Empty old/new values remain valid, preserving insertion/removal semantics.
- [ ] Add decoder rejection tests for unknown fields at every object level, missing/unknown discriminator, wrong payload type, and legacy externally tagged actions/templates inside v1. Preserve existing optional/null ID behavior rather than making IDs newly required.
- [ ] Run `cargo test -p tfmttools-core --test history_contract`; expect failures for the missing stored types and contract.
- [ ] Define explicit individual `#[serde(rename = "...")]` attributes for every stored field and variant, internally tagged action/template enums, and `deny_unknown_fields` for document, records, metadata, action/template payloads, and changes. Use the spec's exact names. Retain `DateTime<Local>` and `Option<usize>`; encodings are `Option<String>` with missing-field defaults.
- [ ] Implement exhaustive conversions without wildcard arms. Stored-to-executable conversion requires `parse_item_key(key)` followed by `canonical_tag_name(parsed) == Some(key)`, excluding runtime aliases and computed date. Validate encoding strings with a literal accepted-name match. Action-to-storage conversion copies payloads; validate the complete record before pushing so failure leaves records unchanged. Do not add filesystem existence validation or execute actions during conversion.
- [ ] Serialize only `StoredHistory`; validate its version and all conversions before saving. For this intermediate commit, loading accepts v1 only. Keep serialization failures before file mutation; Task 3 adds legacy dispatch and atomic writes.
- [ ] Change formatter summaries to match `StoredAction` directly. In `perform_undo_redo_actions`, convert stored actions immediately before calling `ActionHandler`; retain reverse iteration for undo and forward iteration for redo. Update all named-field template constructors/patterns; remove the `Script` rename from the current model.
- [ ] Audit actual Serde consumers of `Action`, `TagValueChange`, and `TagValueKind` with `rg`; remove persistence-only derives/imports/default attributes. Keep unrelated Serde uses in core intact.
- [ ] Run `cargo test -p tfmttools-core`, `cargo test -p tfmt`, and `cargo check --workspace`; expect success.
- [ ] Commit: `Define stable versioned history storage`.

### Task 3: Migrate legacy JSON and preserve history on save failures

**Files:** Create `history/{migration,legacy_tag_keys}.rs`, `crates/core/tests/{history_migration,history_persistence}.rs`, and fixtures `v0-all-variants.json`, `v0-pre-canonical-tags.json`, `v0-mixed-tag-keys.json`, `legacy-tag-spellings.json`; modify persistence/error/runtime files and Cargo dependencies for tempfile.

**Interfaces:**
- Produces private `upgrade_document(value: serde_json::Value) -> Result<(StoredHistory, bool), HistoryError>`; boolean indicates v0 migration.
- Produces private `migrate_v0_to_v1(value: serde_json::Value) -> Result<serde_json::Value, HistoryError>` and `canonicalize_legacy_tag_key(key: &str) -> Option<&'static str>`.
- Persistence helper `decode_history(bytes: &[u8]) -> Result<(StoredHistory, bool), HistoryError>` parses JSON, dispatches/migrates, decodes, then checks every conversion.
- Persistence helper `write_atomically(path: &Utf8Path, bytes: &[u8]) -> Result<(), HistoryError>` writes a same-directory temporary file and replaces the destination only on success.

- [ ] Freeze the mapping from `git show ff1b5a7^:crates/core/src/item_keys.rs` and that revision's locked case-conversion versions. Use a throwaway preparation program reproducing `from_case(Case::Pascal).to_case(Flat/Snake/Kebab)` and the `AppleId3V2ContentGroup` substitution. Check in literal normalized spellings/canonical destinations and independent expected fixture data; no production case-conversion dependency or current enum-debug-derived mapping. Cover all 100 historical keys, all five aliases, and special digit/acronym boundaries; accept current canonical keys too.
- [ ] Add failing migration tests covering all legacy variants/states/templates/kinds, omitted encodings, explicit v0 and missing version, and case variants of every frozen spelling. Pin examples: `TrackArtist`, `trackartist`, `track-artist`, `TRACK_ARTIST` → `track_artist`; `AlbumSort` → `album_title_sort_order`; `DiskNumber` → `disc_number`; `Title` → `track_title`.
- [ ] Add `opaque_strings_and_order_survive_migration`: put strings such as `TrackArtist`, `Applied`, and `Script` in paths, tag values, template text, arguments, and run IDs; assert only known structural positions change and IDs/timestamps/orders/encoding strings retain their values. Include Undone and Superseded records.
- [ ] Add version/error tables: null, strings, fractions including `1.0`, negatives, out-of-range unsigned integers, and unsupported positive integers fail; malformed v1 never falls back to migration. Reject unknown variants, multiple legacy variant keys, wrong payloads, conflicting payload `type`, unknown fields, unknown keys, and `date`. Assert diagnostics identify `records[n].actions[n].changes[n]` for a bad key, including a bad final change in a superseded record.
- [ ] Add failing persistence tests: read-only v0 load changes no bytes and creates no backup; first save writes v1 plus an exact byte backup; later saves preserve that backup; matching preexisting backup is reused; mismatching backup or backup directory fails with source unchanged; current v1 saves create no upgrade backup. Failed load retains an existing live record set and pending upgrade state. Missing history keeps existing `LoadHistoryResult::New` behavior.
- [ ] Add safe-write failure tests with deterministic filesystem obstacles (avoid permission tests that pass under privileged users). Exercise a test-only injected replacement failure after temp writing, if a real obstacle cannot reliably reach that stage. Assert source bytes unchanged, a completed upgrade backup may remain, pending source bytes remain, retry succeeds, and temp files are cleaned up. Test nonexistent parents, a relative filename, and destination-is-directory `SaveErrorWithBackup` recovery whose output decodes as v1. Recovery output must not clear pending upgrade state as though the original was replaced.
- [ ] Run `cargo test -p tfmttools-core --test history_migration` and `cargo test -p tfmttools-core --test history_persistence`; expect failures for missing migration/backups/atomic behavior.
- [ ] Implement direct version dispatch with integer representation checks: missing or integer 0 migrates; integer 1 decodes current; everything else produces contextual invalid/unsupported errors. Require exact recognized legacy variant/payload shape and transform only record state, action discriminators/payload structure, change kind/key, and template discriminators. Set version 1 after all transformations succeed. Let current Serde decoding reject remaining unknown fields; never silently discard legacy fields while constructing new objects.
- [ ] Make load transactional: parse/migrate/decode/check all records locally, then assign live records and `upgrade_source` together. Retain raw v0 input bytes, including whitespace, until a successful replacement. No writes on load.
- [ ] Make save prepare and validate all v1 output bytes first, then ensure the destination directory exists. Before upgrading the actual history destination, create `<filename>.v0.bak` exclusively with the captured bytes, or read and compare an existing file. Refuse any mismatch. Remove a newly created partial backup if writing it fails; never truncate an existing backup.
- [ ] Write the prepared bytes through `tempfile::NamedTempFile` in the destination directory and use its replacement/persist operation. Map errors to path-aware history errors. Clear `upgrade_source` only after the actual destination replacement succeeds. Keep destination-is-not-file recovery separate, using the existing recovery path convention and v1 output. Do not introduce locking or fsync durability promises.
- [ ] Run both new suites and `cargo test -p tfmttools-core`; expect success.
- [ ] Commit: `Migrate legacy history with safe upgrade backups`.

### Task 4: Enforce schema compatibility and prove actual replay

**Files:** Create `history/schema.rs`, `crates/core/tests/history_schema.rs`, `docs/history/schema-v1.json`, `crates/tfmt/tests/history_compatibility.rs`, and CLI historical JSON fixtures under `tests/fixtures/cli/history/`; modify core stored/model derives, Cargo manifests/lockfile, `xtask/src/main.rs`, `xtask/Cargo.toml`, path-template tests, README, CHANGELOG, agent map and relevant fixture guidance.

**Interfaces:**
- Produces `pub fn history_schema_json() -> Result<String, serde_json::Error>` derived from `StoredHistory`, returning deterministically ordered pretty JSON plus a trailing newline.
- Produces `cargo xtask history-schema`, regenerating `docs/history/schema-v1.json` from that function; the snapshot test runs as part of normal core/workspace tests.

- [ ] Add a failing `schema_matches_snapshot` test asserting generated bytes equal the checked-in snapshot. Add schema contract assertions: document version is required and constrained to constant 1; tagged unions use exact names and required payloads; unknown fields are prohibited; ID and encodings mirror Serde optional/null behavior; timestamp matches chrono decoding. Configure field/schema attributes or a focused derived-schema adjustment for the constant version, without creating a second handwritten model.
- [ ] Run `cargo test -p tfmttools-core --test history_schema`; expect failure before generation exists.
- [ ] Add MSRV-compatible schemars support (including chrono where needed), derive schema from every concrete stored type, and implement deterministic output sorting independent of serde_json's `preserve_order` feature. Add the xtask command/help and a core dependency. Generate the snapshot with `cargo xtask history-schema`; regenerate twice and assert unchanged output.
- [ ] Add CLI tests using literal historical JSON, with only a designated fixture path placeholder replaced for temporary directories. Do not use the current serializer to create compatibility input. Update `historical_rename` in `path_templates.rs` to seed literal legacy JSON too, preserving its old-Jinja reuse and undo/redo tests.
- [ ] Add `pre_canonical_tag_history_undo_redo_restores_values_and_encoding`: copy an existing MP3 into a temp directory, arrange its applied value/frame encoding with Lofty, seed a pre-change `TrackArtist` tag edit with `old_encoding: "UTF8"` and `new_encoding: "UTF16"`, run actual `tfmt undo` and `redo`, reread values and frame encodings, and assert Undone/Redone states, v1 output, canonical key, and exact original backup. Also replay a record lacking encodings to prove no encoding is inferred.
- [ ] Add `legacy_filesystem_actions_preserve_replay_order` with multiple records and dependent MoveFile/CopyFile/MakeDir/RemoveDir actions; assert inverse undo and forward redo restore expected tree/bytes. Exercise RemoveFile redo with its existing non-restorable undo semantics rather than inventing restoration behavior. Contract/migration tests already cover all six variants; CLI tests prove actual replay semantics.
- [ ] Add `invalid_history_stops_commands_before_actions`: a valid first action and invalid later tag change cause undo/redo failure with audio/files/history unchanged. Also invoke rename and validation fix against invalid/future history and assert no actions occur. Add `show_history_is_read_only_for_v0` and confirm no backup or rewrite; assert formatter summaries retain all six action counts without converting for display.
- [ ] Run `cargo test -p tfmttools-core --test history_schema`, `cargo test -p tfmt --test history_compatibility`, `cargo test -p tfmt --test path_templates`, and `cargo xtask test-cli`; expect success. Add `history_compatibility` to xtask's explicit CLI test steps so its normal CLI/test workflow includes this suite (do not rely only on direct workspace testing).
- [ ] Update README/CHANGELOG: migration in memory on load, upgrade on normal save, exact `.v0.bak` naming/collision behavior, unsupported/invalid history errors, and preserved legacy replay. Replace instructions to clear old tag-key history and the claim that the format is unchanged. Explain schema regeneration and version-bump/migration/fixture review discipline. Keep old-template reuse restrictions explicit; migration does not translate Jinja or any other template text.
- [ ] Recheck architecture/task guidance for the removed crate, including fixture docs if new history fixtures are documented. Verify `rg -n 'tfmttools.history|crates/history|ActionRecord\b' Cargo.toml Cargo.lock crates README.md AGENTS.md docs/agent-map.md` has no obsolete references; historical specs/plans may retain historical paths.
- [ ] Run `cargo +nightly fmt --all`, `cargo test --workspace`, `cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`. Require all commands to succeed; fix or explicitly document actionable lint findings. Confirm dependency Rust version floors remain compatible with 1.89.0 and run `cargo +1.89.0 check --workspace` if the toolchain is installed.
- [ ] Commit: `Check history schema and legacy replay compatibility`.

## Completion review

- [ ] Confirm the spec's model/ownership and all exact v1 names are covered by Tasks 1–2.
- [ ] Confirm version rejection, frozen historical mapping, complete-load validation, opaque-string preservation, and backup/atomic failure behavior are covered by Task 3.
- [ ] Confirm fixed fixtures, actual tag/frame and filesystem replay, schema regeneration/snapshot, and user/repository documentation are covered by Task 4.
- [ ] Confirm there are no legacy aliases in the current decoder, remaining executable-action persistence dependencies, runtime schema-engine dependencies, or duplicated record/metadata models.
- [ ] Review the final diff and verification evidence before integration; do not merge or publish as part of executing this plan without the corresponding user instruction.
