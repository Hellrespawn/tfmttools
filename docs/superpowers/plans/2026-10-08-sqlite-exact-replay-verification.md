# SQLite history implementation verification

All seven implementation tasks are complete on `sqlite-exact-replay`.
The approved design remains the storage and recovery contract.

## Verification

- `cargo test --workspace`: passed, including all 12 CLI fixtures.
- `cargo +nightly clippy --workspace --all-targets`: passed without warnings.
- `cargo xtask lint`: passed.
- `cargo +1.91.0 check --workspace`: passed.
- Repeated `cargo xtask history-schema`: identical SQL snapshot.

The fresh branch review found one Critical and five Important issues. All were
fixed in one pass with regression tests observed failing before implementation:

- Validate completed copy destinations before resuming destructive removals;
  changed/missing destinations preserve the original. Already-applied removal
  still resumes correctly.
- Validate Copy source-removal flags and Directory direction both on append and
  database load; malformed descriptors cannot change the recorded effects.
- Distinguish identical-byte candidates from already-installed originals.
- Use consistent path identities for dotted paths and parent symlink aliases;
  recognize a case alias through its actual directory entry without conflating
  distinct case-sensitive hard links.
- Clear the opened history database while preserving its configured symlink.
- Undo standalone copies while preserving matching originals and rejecting
  externally changed originals.

No review minors were deferred. Case-insensitive and non-Unix runtime behavior
was not exercised on this Linux host; a conditional case-only test is provided.
The portable replacement protocol retains its documented pathname visibility
gap and filesystem synchronization assumptions.

## Execution decisions

- use a FixedOffset timestamp internally for persisted records — Local decoding normalizes offsets and cannot preserve recorded offsets; callers only display timestamps — cost: internal timestamp type changes for callers.
- make journal entries appendable as each dependent action is prepared — whole-run pre-preparation fails for staged renames and sequential edits; pending run remains unfinalized until all actions complete — cost: more preparation transactions and incremental recovery.
- reconstruct native ID3 URL frames while saving candidates — Lofty 0.25.4 parses Locator values but its generic URL conversion accepts Text only and discards them; verified candidate URL test caught this — costs interoperability if mappings differ.
- add set_operation_plan before any effects — append-only progress lacks the remaining actions after interruption, so durable recovery otherwise cannot finish mixed runs — costs an extra initial transaction.
- prepare copies in synced sibling candidates as well — writing a destination directly creates ambiguous partial copies after interruption — costs one temporary file per copy.
- verify the final touched-path state before artifact cleanup — earlier tag edit paths may be renamed or edited again within the same run; checking each historical intermediate output would incorrectly block cleanup — costs a final identity pass over touched paths.
- allow cancellation only for operations with zero prepared entries — interruption between begin and saved plan cannot have file effects and can be safely cleared — costs possible unused record ID reuse before any record is published.
- read native ID3 encodings when planning/checking fixes — generic conversion normalizes encoding names, causing needless changes and incorrect old_encoding metadata — costs one native tag parse per file.
- plan cleanup before effects and exclude active config/bin directories — remaining actions must be journaled before any mutation, and moving the live database would break recovery — costs moving cleanup confirmation earlier in the run.
- preserve permissions, with ACLs and extended attributes outside this change — approved contract deliberately excludes them — cost: replacement may lose those additional attributes.
- retain the staged-rename visibility gap — approved portable protocol keeps originals without another backup copy — cost: observers can briefly see a missing original pathname.
- rely on identity rechecks rather than excluding arbitrary external writers — tfmt's session lock cannot constrain other programs — cost: races after checks remain possible.
- keep JSON import, permanent backups, and directory-wide atomicity excluded — these are the user's approved boundaries — cost: old histories need explicit handling and interrupted runs recover incrementally.
- require reliable filesystem locking and synchronization — source-level sync checks cannot establish hardware power-loss behavior — cost: unreliable/network filesystems can weaken durability.
- defer atomic exchange and performance tuning — portable correctness is the approved initial scope — cost: visibility and performance opportunities remain.
- provide case-alias handling with conditional tests, without claiming non-Unix runtime verification — this Linux host cannot exercise those environments — cost: platform-specific behavior needs further verification.
