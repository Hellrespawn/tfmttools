# SQLite history and selective raw-byte tag undo

Date: 2026-10-08
Status: Draft for review; no implementation changes made.

## Intent and scope

Replace JSON history persistence with SQLite and record original bytes for
individual tag edits. Undo and redo restore only the recorded change and
preserve unrelated edits made afterward. Support every tag format currently
edited by validation: ID3v2 in MP3, Vorbis comments in Ogg, and MP4 ilst in M4A.
This does not expand the CLI's supported extensions or modify secondary tags.
Journal apply, undo, and redo durably before their file effects. Keep in-place
audio writes and make a temporary independent backup before each audio-file
write. Delete it after durable success. If a write is interrupted or damages the
file, retain the backup and prompt for manual restoration; tfmt does not replace
or restore the original automatically.

Preserve record selection rules, action ordering, metadata, and the
applied/undone/redone/superseded transitions for new histories. This is an
intentional compatibility break: invalidate all old JSON histories and remove
their migration and value-based replay support. Do not import old records or
attempt to reconstruct their original bytes from current audio files.

Reject existing old histories before file effects, with instructions to move
the file aside or explicitly clear history. Leave the file untouched on ordinary
load failure; never silently discard it or start an empty history in its place.

The merged concrete history model in core is the starting point. Keep stored
actions separate from executable actions and keep executable file IO in fs.

## Approach

Use rusqlite directly with handwritten SQL and rusqlite_migration to execute
explicit, ordered migrations. Do not introduce an ORM, schema generation,
migration generation, or runtime model-to-schema synchronization. Select and
pin dependency versions compatible with the workspace MSRV of Rust 1.91.0.

Use relational tables for records, ordered actions, and individual tag changes,
with BLOB columns for native entry snapshots. Avoid placing the complete history
or actions in JSON blobs. Small metadata such as CLI arguments may remain JSON
TEXT with explicit validation; raw bytes never pass through JSON.

Alternatives considered: storing whole records as binary blobs would simplify
persistence but obscure individual changes and future database migrations;
normalizing every piece of command metadata would add tables without helping
selective undo. The proposed schema normalizes the data that is queried or
updated and stores opaque command arguments together.

Use a single SQLite connection owned by History. Retain the current in-memory
record selection interface where useful, but replace apply-then-save execution
with explicit prepare/complete journal operations. Commit preparations before
file effects and commit progress after each verified effect. Only insert new
rows and update changed states; do not rewrite existing records. Refresh cached
state after successful commits and retain pending operations on failure.

## Database contract

Assign a tfmt application ID and use PRAGMA user_version for database migrations.
rusqlite_migration owns user_version as the number of applied migrations; do not
maintain a competing version counter. Commit each migration as immutable SQL
and embed it in the binary. The initial migration creates the complete schema;
future migrations append to the ordered list. Check the application ID and
supported version before running migrations, and never migrate a foreign or
future database. Read-only commands do not apply pending migrations; write
preparation applies supported upgrades under the command lock before file effects.
The first SQLite format has version 1 in a new, independently identified format
lineage. No old JSON version is accepted. Reject foreign application IDs,
unsupported database versions, malformed
rows, unknown discriminators, and invalid executable conversions before accepting
loaded records. Enable foreign keys on every connection.

Tables:

| Table | Main columns and constraints |
| --- | --- |
| records | internal integer primary key; unique position; unique non-null public ID; replay state; execution status; timestamp text; template type/value; arguments JSON; run ID |
| actions | record foreign key; position; action type; source/target/path fields with variant-specific checks; unique record/position |
| tag_changes | action foreign key; position; canonical key; value kind; old/new display text; native format; selector BLOB; before/after BLOBs; unique action/position |
| attempts | record foreign key; direction (apply/undo/redo); status (pending/complete/cancelled/conflict); start/completion timestamps; error context |
| operations | attempt foreign key; ordered position; action reference or grouped tag-action references; expected source/target evidence; progress (pending/complete/cancelled/conflict); error context; unique attempt/position |
| file_backups | operation foreign key; unique backup path; source identity; expected length/digest; lifecycle (building/ready/cleanup_pending/removed); error context |

Keep operation progress separate from the record's replay state. New runs have
execution status pending until completed; incomplete records are displayed but
excluded from ordinary undo/redo selection. An undo/redo attempt changes the
record replay state only when every required operation completes. Never erase
previous attempts when retrying. Enforce one unresolved attempt per record.

Use internal row IDs independently of public record IDs and explicit positions
rather than relying on SQLite row order or AUTOINCREMENT. New persisted records
always have public IDs; keep zero-based allocation consistent with current
behavior. Keep any unsaved optional ID exclusively in the runtime model.

Create all application tables with STRICT typing. Use SQLite INTEGER, TEXT, and
BLOB storage types explicitly, with NOT NULL, foreign keys, uniqueness, and CHECK
constraints for valid states and variant-specific payloads. STRICT typing does
not replace these semantic constraints. Select a SQLite dependency providing at
least SQLite 3.46.0, which includes STRICT support and bounded PRAGMA optimize.

All stored tag changes use native replay; there is no legacy discriminator or
fallback. Enforce required fields for each native format. For native snapshots,
SQL NULL represents absence and a zero-length BLOB represents an empty entry.
Selectors and snapshots have a documented format-specific binary contract;
database migrations also cover changes to that contract.

Check in the actual schema SQL and fixed database compatibility fixtures. Remove
the old JSON migrations, frozen historical tag-key mapping, JSON-only storage
wrapper and conversion paths, obsolete compatibility fixtures, JSON schema
snapshot, and cargo xtask history-schema command. Replace their documentation
and repository guidance with the SQLite contract and schema checks. Retain
action conversions and validation that are still needed by native stored
actions. Future SQLite schema changes require deliberate migrations and fixed
compatibility fixtures; this break does not waive future version discipline.
Use rusqlite_migration validation to check migration execution, plus independent
SQLite schema assertions for columns, indexes, foreign keys, and CHECK constraints.
Test upgrades with fixed older SQLite databases and verify their data survives;
successful execution alone does not establish migration correctness.

## Native tag edit module

Put format handling behind a focused module in fs. Core owns plain stored and
executable change data, not container parsing or writing. The module prepares an
edit from the current file, applies it, and replays captured edits in either
direction. ActionExecutor must return the actual captured action rather than
echoing a planned value change as it currently does.

Capture bytes from the source file before decoding. Prepare the replacement
bytes before writing, then verify the affected entries after the write. Store
decoded old/new text only for previews and history display. Encoding information
for new records is part of the native representation rather than optional string
fields used for reconstruction.

Format units:

- ID3v2: preserve original tag version and the affected frame/value bytes,
  including encoding and relevant flags. Identify frames using native IDs and
  qualifiers such as description/language. Handle tag/frame unsynchronisation
  at the correct layer. Do not upgrade the tag version during replay.
- Vorbis comments: preserve native field spelling and raw comment value bytes.
  Replace the specific comment occurrence while keeping the vendor string,
  unrelated comments, pictures, and audio packets. Rebuild Ogg framing and CRCs
  when packet lengths change.
- MP4 ilst: identify item atoms, including freeform mean/name qualifiers and
  individual data children. Preserve type/locale metadata and untouched atom
  bytes. Update enclosing sizes and media offsets where resizing requires it.

Multiple logical values can occupy one native entry. If a frame or atom contains
several values, merge only the changed value into its current entry. Whole-entry
replacement is allowed only when that entry contains exactly the changed unit
or all of its values are part of the same recorded change. A later edit to a
sibling value must survive undo. Store sufficient original substructure to
restore the changed value faithfully.

Selectors must not use absolute file offsets or a global occurrence index as
identity: resizing or inserting another entry makes those unstable. Combine
native identity, qualifiers, expected bytes, and relative duplicate information.
Match only when the target is unambiguous; missing or ambiguous targets are
conflicts. Identical duplicate entries require explicit multiplicity handling
and tests, not an arbitrary first match.

Undo checks the changed unit against the recorded after state; redo checks it
against the before state. If it changed again, fail with path and entry context
before replacing it. Preflight every change in one file against one current
snapshot and perform one coordinated write for that file. In-place IO can still
fail halfway through the write; preflight does not make it atomic.
New records never use decoded-value reconstruction as a replay fallback.

Preserve unrelated native metadata bytes and audio payload. Structural sizes,
offsets, padding, packet boundaries, and checksums may change as required to keep
the container valid. This is not a promise of byte-identical whole files.

## Persistence, locking, and old-history rejection

Keep the configured history path and resolve symlink chains as today. Existing
files must have the SQLite header and the supported tfmt database contract.
Reject JSON and other non-SQLite files with a clear incompatible-history error;
do not invoke a legacy decoder or create a replacement database. Invalid SQLite
must not be treated as empty history. Existing zero-length files are invalid too.

Read-only commands neither replace old histories nor create new databases at
missing paths. Create a new database only for a write operation. Do not replace
an existing destination while a SQLite connection to it is open.

Explicit clear-history must work on incompatible old histories without loading
their records. Acquire the same resolved-path lock and use the existing clear
command's confirmation policy before removing the resolved regular file. Preserve
symlinks, reject non-file targets, and do not remove old .v0.bak/.v1.bak backups.
Changing the format is not authorization to delete existing user history during
implementation or testing.

Keep the existing adjacent OS lock for the entire load/apply/save session,
including missing files, rejected old histories, and clear-history. SQLite transactions
protect database updates but do not serialize audio-file work across commands.
Keep immediate lock-conflict reporting and symlink/direct-path lock equivalence.

Configure writable connections explicitly with journal_mode=DELETE,
synchronous=EXTRA, and foreign_keys=ON before starting transactions. Read back
the settings and fail preparation if required settings did not take effect.
Enable and verify foreign_keys on read-only connections too; they must not change
the persistent journal mode or perform maintenance writes. Keep application_id
validation and rusqlite_migration-managed user_version as described above.

Rollback-journal mode fits the current serialized CLI sessions. EXTRA adds
directory synchronization after rollback-journal deletion for durable
prepare/completion commits, subject to the filesystem's guarantees. Do not use
synchronous=NORMAL for the operation journal. WAL with synchronous=FULL may be
evaluated later if measurements justify it; changing journal mode requires
review of recovery, backup, and clear-history handling.

Use explicit transactions for each journal update. Only insert new rows and update
changed states. Clearing history preserves the lock, closes the connection, and
removes the database and any owned journal artifacts, preserving current CLI
missing-history behavior.

Run PRAGMA optimize as best-effort maintenance at the end of a successful writable
session, after operations and backup cleanup are resolved and outside their
transactions. Do not run it for read-only commands or failed/interrupted runs.
With SQLite 3.46.0 or newer, optimize bounds analysis automatically; do not set
analysis_limit separately. A maintenance failure must not reclassify completed
file operations as failed. Keep cache_size and temp_store at their defaults;
only tune memory/performance settings after measuring this workload.

prepare_save checks predictable destination, schema, and write-transaction
failures before file effects. Preserve recovery output for late save failures, using an
exclusively named valid SQLite recovery database containing the pending history;
report its path and do not claim the normal history save succeeded.

## Durable operation journal and recovery

SQLite and audio files do not share a transaction. The journal makes interrupted
work detectable and recoverable when files remain intact. A temporary full-file
backup provides a manual recovery source for interrupted in-place writes. This
does not make a run atomic or automatically repair a damaged audio file.

Preparation must occur in the CLI coordinator before calling the fs effect.
Keep core independent of fs: core persists operations; fs prepares, performs,
and inspects them. Preparation determines actual staged rename actions as well
as native tag snapshots; do not journal only a high-level rename request when
execution will use additional intermediate paths.

Protocol for each operation:

1. Resolve and preflight the target, capture native before bytes, construct exact
   after bytes, and gather identity and structural verification evidence. For
   tag edits the operation groups all changes written to one file.
2. Commit the record/action payloads and pending operation to SQLite. A failed
   preparation commit prevents the effect. Commit the known ordered plan before
   starting it; capture dependent operations' concrete evidence before their
   own effects and commit that evidence before proceeding.
   For an audio-file write, also record a unique adjacent backup path and
   building state before creating the backup. Copy into a new independent file
   with exclusive creation; never use a hard link. Validate the copy's bytes and
   source identity, sync the backup and containing directory where supported,
   then commit ready state. Do not modify the original unless the backup is
   ready and its readiness is durably recorded. On any failure during backup
   preparation, leave the original untouched.
3. Recheck file preconditions immediately before execution. Apply in place using
   the existing operation semantics. The history lock excludes other tfmt
   commands using this history, not unrelated external file editors.
4. Flush file changes as required for durability and verify the changed entries
   and valid container. Commit operation completion. For renames and directory
   operations sync affected directories on supported platforms before marking
   completion. Define platform-specific durability limits explicitly.
5. Complete the attempt and transition record state in the same transaction as
   the last operation's completion. Do not proceed to another operation if a
   completion commit fails. The previously committed pending entry remains the
   recovery source of truth even if a recovery database cannot be written.
6. After durable completion, mark the backup cleanup_pending, delete the owned
   backup, sync its directory where supported, then mark it removed. Retry
   interrupted cleanup idempotently. Do not start another file effect until
   completion and backup cleanup succeed. A cleanup failure stops the run and
   identifies the retained backup; it does not undo a successful file write.

Execute one file operation at a time. Only one audio file can be in the active
write/verification window, and only one ready recovery backup may remain before
execution stops. Completed earlier files remain completed. Temporary disk space
is approximately one audio file per active session, not the entire run. Use
restrictive backup permissions, keep recorded ownership and identity evidence,
and remove only the exact backup created by this operation. An unfinished
building backup can be incomplete and is never offered as a verified recovery
source. If interruption preceded ready state, the protocol guarantees tfmt had
not yet started writing the original.

Preflight known tag conflicts for the selected record before its first tag
write. Commit each successful operation independently so an ordinary later
failure cannot discard earlier progress. Apply the same protocol to undo/redo,
with reversed before/after expectations and the existing action ordering.

On restart, display unresolved attempts. A mutating command inspects them under
the history lock before starting unrelated work; read-only history display never
modifies files or journal progress. Reconciliation classifies each pending
operation using native selectors and current structural/file evidence:

- Before state: the operation is still unapplied and can be resumed or cancelled.
- After state: the effect took place; verify durability and record completion
  without applying it again.
- Mixed, missing, ambiguous, unreadable, or structurally invalid state: conflict;
  preserve the journal and any ready backup and stop without guessing or
  rewriting the file. Display the original and backup paths and prompt the user
  to restore manually if the file was damaged. Do not prescribe whole-file
  restoration for a valid file containing unrelated subsequent edits.

When offering manual restoration, explain that copying backup bytes into the
existing file preserves its identity and hard-link relationships, while replacing
the path may not. tfmt never performs either restoration automatically. After
restoration, re-inspect identity and before/after evidence before resuming. A
verified before state allows retry or cancellation; retain the backup until the
attempt is safely resolved. Restoration may itself be interrupted, so retain
the verified backup until recovery succeeds.

When before and after states are identical, use direction-specific identity and
structural evidence; if completion cannot be distinguished, report ambiguity.
For grouped tag edits, all affected units must match the expected side. Preserve
unrelated edits; a whole-file hash is not the sole test for native tag recovery.

Provide explicit recovery choices to resume an intact pending attempt or cancel
its unapplied remainder. Do not automatically replay pending file effects during
load. Cancellation never changes already applied files: a partially applied new
run is finalized as a partial record containing only its completed actions,
while retaining cancelled operation evidence in the journal. It can then be
undone through normal journaled replay. For a partially completed undo/redo,
resume the attempt; cancellation is allowed only if no operation completed.
Rolling back partially completed replay is outside this initial recovery design.

Journal existing filesystem actions too. Record path existence, type, content
identity where appropriate, destination preconditions, and staged-path links.
The journal adds progress evidence, not new reversibility: do not claim it can
restore deleted data unless the existing action sequence retains a recoverable
copy. Ambiguous copy/remove/rename results require explicit conflict resolution.
After a conflict, re-inspection may allow recovery if the user restores an
expected state; no force-overwrite recovery is included.

History display identifies pending, partial, cancelled, and conflicting attempts.
Block unrelated writes until unresolved work is reconciled or explicitly
cancelled. Explicit clear-history warns that it also discards pending recovery
information; its existing confirmation policy still applies. Retained backups
are left in place when clearing history and their paths are reported.

In-place writing preserves current file identity and hard-link relationships;
the backup does not require replacing that identity. Ordinary writes may still
change timestamps or special metadata such as set-ID bits, so do not promise
that every metadata field stays unchanged. In-place writes cannot guarantee an
intact original after disk-full or process/power failure, but the verified backup
is retained for manual restoration. There are no staged replacement files,
run-wide all-or-nothing guarantees, or automatic repair in this scope. A backup
on the same filesystem does not protect against catastrophic drive/filesystem
failure or unrelated external writers.

## Verification and documentation

Use minimal fixed old JSON samples to verify rejection before file effects and
unchanged source bytes, without retaining the old migration test suite. Verify
that explicit clear-history can remove incompatible history and a subsequent
write starts fresh. Cover read-only missing paths, IDs, metadata, ordering, state
transitions, symlink chains, foreign/future databases, corrupt rows, transaction
rollback, retry, lock exclusion, clearing, and recovery databases.
Inject failures before journal preparation commits, before file effects, during
in-place writes, after effects but before completion commits, and between files.
Restart with a fresh process and verify before/after/conflict classification,
idempotent reconciliation, explicit resume/cancel, partial runs, partial replay,
and unchanged unrelated values. Cover apply, undo, redo, staged renames, and
database-full failures. Assert no effect occurs without a committed preparation
and no further effect occurs after failed completion persistence. Verify SQLite
durability settings; distinguish process-crash tests from power-loss guarantees.
Check STRICT table definitions, rejection of incompatible storage types, semantic
constraints, and connection settings. Read-only tests verify no maintenance or
journal-mode writes. Verify optional optimization failures do not change operation
completion status.
Inject failures during backup creation, readiness commits, deletion, and cleanup
commits. Verify exclusive creation, independent backup bytes, hard-link identity
of the original, no writes before a ready backup, one active file, and no further
file effects after uncertain writes or failed cleanup. Verify retained backup
paths in manual recovery prompts, cleanup retry after successful effects, and
re-inspection after manual restoration. Clearing history must retain recovery
backups. Backup permission tests cover disclosure of previously private files.

For each supported format, test apply/undo/redo using independently constructed
native fixtures. Assert restored changed bytes, preserved unrelated entry bytes
and audio payload, valid containers, subsequent unrelated edits, duplicate keys,
multiple values in one entry, empty values, encoding-only edits, size changes,
missing targets, and conflicts. Include ID3 version/encoding variants and MP4
atom/Ogg packet resizing. No format may fall back to decoded-value replay for
new records merely because native capture is difficult.

Before committing to format-writing implementation, inspect each format and
prove raw capture and selective replacement with throwaway probes. Lofty's
decoded generic Tag is insufficient evidence of byte preservation. The design
must be revised if a probe contradicts a promised guarantee.

Update README, CHANGELOG, history documentation, and affected CLI fixtures.
Run focused core, fs, and CLI suites, then cargo test --workspace,
cargo +nightly clippy --workspace --all-targets, and cargo xtask lint.

## Proposed implementation sequence

1. Prove the three native format edits and finalize selectors/byte contracts.
2. Add SQLite schema, validation, journal transitions, and persistence tests.
3. Remove old JSON support, add rejection and explicit reset behavior, preserve
   locks/symlinks/recovery, and adapt CLI consumers and repository guidance.
4. Prepare native edits and journal apply/undo/redo for every action and format.
5. Add per-file backups, restart reconciliation, manual restoration guidance,
   recovery choices, and partial-progress reporting.
6. Verify interruption failures, rejection, reporting, and documentation.

Keep SQLite and native replay in one reviewed design, with separate reviewable
implementation steps. Detailed implementation planning follows design review.

## Sources

- Existing history contract:
  docs/superpowers/specs/2026-10-07-history-schema-design.md.
- SQLite transaction semantics: https://www.sqlite.org/lang_transaction.html.
- SQLite commit durability: https://www.sqlite.org/atomiccommit.html.
- File and directory synchronization: https://man7.org/linux/man-pages/man2/fsync.2.html.
- SQLite version/application identifiers: https://www.sqlite.org/pragma.html.
- SQLite STRICT tables: https://www.sqlite.org/stricttables.html.
- SQLite synchronization: https://www.sqlite.org/pragma.html#pragma_synchronous.
- SQLite bounded optimization: https://www.sqlite.org/pragma.html#pragma_analysis_limit.
- Reviewed recommendations: https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/.
- SQLite foreign keys: https://www.sqlite.org/foreignkeys.html.
- Explicit migration runner: https://docs.rs/rusqlite_migration/latest/rusqlite_migration/.
- Vorbis comment representation: https://xiph.org/vorbis/doc/v-comment.html.
- Lofty 0.25.4 source inspected locally: tag types, file primary tag mappings,
  generic tag read/write path, Vorbis comment decoding, and MP4 data decoding.
