# SQLite history and exact audio replay

Date: 2026-10-08
Status: Approved

## Outcome and agreed choices

Replace JSON history with a versioned, validated SQLite database using
`rusqlite` and `rusqlite_migration`. Keep record IDs, timestamps, metadata,
record/action ordering, and applied/undone/redone/superseded semantics.
Reject existing JSON histories without importing, rewriting, or deleting them.
This design supersedes the JSON storage and migration portions of the
2026-10-07 history design.

Tag edits record forward and reverse `qbsdiff` binary patches. Undo and redo
restore the recorded bytes rather than rerunning the tag serializer, and reject
files whose bytes differ from the expected input.

The user approved retaining the original through the file switch. No additional
backup copy is made: the original is moved aside temporarily, then removed only
after replacement and durable history finalization succeed.

## Storage and ownership

Core owns SQLite persistence, migrations, validation, record selection, and
operation journaling. Filesystem code owns candidate preparation, verification,
patch generation/application, switching, and recovery file operations. The CLI
coordinates them; filesystem code does not write history itself.

Keep the existing history pathname so an old JSON file cannot silently become
an apparently empty history. Inspect existing files before SQLite opens them;
reject JSON with a clear unsupported-format message. Never fall back to empty
history following a decoding or validation error.

Use a database application identifier and migration-managed `user_version`.
Reject foreign databases and newer schema versions before applying migrations.
Validate schema structure, foreign keys, stored discriminators, action payloads,
IDs, order, timestamps, and patch descriptors before accepting loaded records.
Failed loads preserve the previous in-memory view and existing files.

Use `STRICT` tables alongside `NOT NULL`, `CHECK`, uniqueness, and foreign-key
constraints. Strict typing supplements application validation; it does not
replace checks on action payloads or patch descriptors. Require SQLite 3.37.0
or newer for this schema.

Tables contain records, ordered actions, binary patch pairs, and pending
operations with ordered progress entries. Metadata and action payloads may use
strictly validated JSON inside SQLite; binary patches are BLOBs, not JSON arrays.
Explicit sequence columns preserve order independently of row IDs. Constraints
enforce unique IDs and action positions, valid states, foreign keys, and required
patch pairs for tag edits. Preserve metadata strings and datetime offsets.

Use SQLite transactions for related writes.
Retain the session-wide history lock to serialize tfmt writers. Do not hold an
uncommitted transaction across a file switch: recovery requires committed
journal entries to survive interruption. Pending operations block conflicting
history mutations until recovery completes.

## SQLite connection settings

Use rollback journaling with `PRAGMA journal_mode=DELETE` and set
`PRAGMA synchronous=EXTRA` on every writable connection before transactions.
EXTRA also synchronizes the journal directory after deletion at commit, making
it appropriate for the recovery journal's durability requirements. Do not use
`synchronous=NORMAL`: losing a committed patch or intent after power loss could
leave a surviving audio replacement without its recovery information.

The session lock already serializes tfmt access, so WAL's concurrent-reader
benefit is limited. Rollback journaling avoids persistent WAL sidecars and WAL's
network-filesystem restriction. This choice does not guarantee correct operation
on filesystems that do not provide reliable locking and synchronization. If WAL
is adopted later, require `synchronous=FULL` and review backup and cleanup rules.

Explicitly enable `PRAGMA foreign_keys=ON` on every connection before starting
transactions or migrations. Check the effective setting rather than relying on
SQLite defaults. Run `PRAGMA foreign_key_check` when validating existing history
and after migrations; enabling enforcement alone does not validate existing rows.
Read back journal mode and synchronous settings on writable connections and fail
before filesystem effects if the required settings cannot be established.

Use migration-managed `user_version` for schema versions and `application_id`
for database identity as described above. Keep default cache and temporary-store
settings until measurements justify tuning. Run `PRAGMA optimize` as best-effort
maintenance before closing a writable connection, outside recovery-critical
transactions. With SQLite 3.46.0 or newer, it bounds its analysis automatically;
on supported older versions, set `analysis_limit=400` first. Read-only commands
do not run maintenance that writes to the database.

These choices follow the consistency advice in the
[SQLite pragma cheatsheet](https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/),
with stronger durability than its NORMAL recommendation. The authoritative
contracts are SQLite's [synchronous settings](https://sqlite.org/pragma.html#pragma_synchronous),
[foreign-key settings](https://sqlite.org/pragma.html#pragma_foreign_keys),
[STRICT tables](https://sqlite.org/stricttables.html), and
[optimize guidance](https://sqlite.org/pragma.html#pragma_optimize).

## Patch contract

Each tag edit stores source and destination byte lengths, cryptographic
digests, forward and reverse patches, and a patch-format identifier. Keep tag
changes for previews and history display, but never use them for replay.
Use a cryptographic digest rather than the existing short filename checksum.

Candidate preparation reads the original bytes, copies to a separate file in
the same directory, applies all tag/encoding writes there, then rereads it.
Verification checks audio readability and requested tag values and encodings;
unfulfilled edits fail rather than silently recording a no-op. Generate both
patches and apply each to its source in memory to verify exact round trips.

Undo requires the current file to match the recorded destination; redo requires
the recorded source. Verify length and digest before patching, and verify the
reconstructed result afterward. Write and sync a candidate before switching.
Corrupt patches and stale files leave the original untouched. Recheck the input
immediately before switching; this does not promise exclusion against arbitrary
external writers that ignore tfmt's locks.

## Durable switch and operation progress

1. Prepare, verify, and sync the candidate, preserving original permissions.
2. Commit patch pairs and a pending operation, including record metadata,
   expected hashes, candidate/retained-original paths, and action progress.
3. Recheck the original, move it to an exclusively reserved sibling path, and
   move the candidate into the original path. Sync the containing directory
   where supported. If installation fails, restore the retained original.
4. Commit action completion and, when all actions finish, the record state and
   redo supersession changes. A partially completed run remains recoverable
   through its journal; do not report it as a completed run.
5. Remove the retained original only after completion is durable. Persist
   cleanup completion; failed cleanup keeps recovery information available.

Use portable staged renames initially. They retain the original without making
another copy, but there is a brief interval with no file at the original path.
Atomic exchange can be added as a platform optimization without changing the
journal protocol. Ordinary overwriting rename is insufficient.

Journal every ordered filesystem action in a mixed rename/tag-edit run, including
staging moves and cleanup, so interruption cannot lose track of completed work.
Undo processes actions in reverse; redo processes them forward. Persist progress
per action and finalize each record independently rather than saving all replay
states only at the end. Failures stop subsequent actions.

Recovery inspects journal progress and actual paths/hashes. Complete or resume
only a uniquely identifiable expected state: source intact, source retained
with target absent, or expected target installed. If external changes make the
state ambiguous, report the paths and stop without overwriting or deleting them.
Keep candidate and retained original files on uncertain failures. Read-only
history display reports pending work without modifying audio; mutating commands
recover pending work before starting a new operation.

## Edge cases and boundaries

Dry runs perform no audio writes, patch persistence, or recovery switches.
Resolve audio symlinks to their targets so replacement preserves the link.
Reject multiply hard-linked audio files before editing: replacement would change
the existing in-place semantics for aliases. Clear-history must refuse while an
operation needs recovery, and remove the database only after closing SQLite.

No automatic JSON import, permanent full-file backups, replay through Lofty,
new backup CLI option, or all-or-nothing transaction across an entire directory.
The protocol handles interrupted tfmt operations; it does not make SQLite and
the filesystem one atomic transaction or guarantee preservation of all platform
metadata such as ACLs and extended attributes.

## Verification and documentation

Replace JSON migration/schema tests with database schema and compatibility
fixtures. Cover unsupported JSON/foreign/future databases, malformed rows,
ordering, metadata, stable IDs, state selection and supersession, locking,
transaction failures, and migration validation.
Verify effective connection settings, foreign-key enforcement on each new
connection, rejection of existing foreign-key violations, and STRICT/constraint
rejection of malformed writes. Ensure read-only access does not perform database
maintenance writes.

Audio tests compare complete bytes before edit, after edit, after undo, and after
redo. Cover stale inputs, corrupt patches, failed candidate verification,
permissions, symlinks, hard links, dry runs, and interruptions around each journal
commit and file rename. Include mixed rename/tag edits and partial replay.

Update README, CHANGELOG, history docs, relevant fixtures, crate guidance, and
the history-schema xtask to describe the SQLite schema contract instead of a
published JSON document. Preserve readable tag-change history previews.

Run focused core, filesystem, and CLI tests, followed by `cargo test --workspace`,
`cargo +nightly clippy --workspace --all-targets`, and `cargo xtask lint`.
