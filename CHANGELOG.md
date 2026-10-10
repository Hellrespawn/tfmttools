# Changelog

## Unreleased

### Added

- Add versioned, validated SQLite history using `rusqlite` and
  `rusqlite_migration`, with STRICT tables, foreign keys, rollback journaling,
  and `synchronous=EXTRA`. Keep record IDs, ordering, metadata, and replay states.
  `cargo xtask history-schema` now generates the SQL schema snapshot.
- Add forward/reverse `qbsdiff` binary patches for tag edits. Undo/redo restores
  exact recorded bytes and rejects files changed since the recorded operation.
- Write tag edits to verified candidates and save patches before switching.
  Retain the original until durable action confirmation without an extra backup copy.
- Record successful actions incrementally and report the last attempted action
  after errors or interruptions. Add `resolve-history --attempt ID --outcome
  applied|not-applied` for explicit history-only resolution after manual file
  repair. Later sessions never resume or reverse interrupted file operations.
- Keep partial runs undoable and disable their redo. Track partial undo/redo
  progress and block mutations while an attempted action remains unresolved.
- Lock history sessions using a persistent sibling lock file. Report
  contention immediately, and acquire the lock before applying tag fixes.

- Added an independent `tfmttools-picotmpl` workspace crate implementing the new
  path template language, with a Stef layout example and behavior tests.
- Added generated shell completions to release archives and Arch packages.
- Added generated man pages to release archives and Arch packages.
- Added `tfmt validate id3-encoding` to report non-ASCII ID3 text frames
  stored as UTF-8.

### Changed

- Replace the recovery journal with one current-attempt marker and a shared
  apply/undo/redo execution flow. Remove remaining-plan persistence, filesystem
  outcome inference, automatic rollback, and durable cleanup tracking.
- Keep tag backups private to tag replacement; cleanup errors leave the action
  confirmed and report leftover paths. Preserve destination collision checks.
- Define a single initial SQLite schema for incremental history. Remove
  the specific v2 migration for the undeployed recovery schema; retain the
  migration framework for future published changes.
- Plan cleanup confirmation before filesystem effects and protect configuration
  and bin directories. Verify and sync copies before associated source deletion.
- Read native ID3 encodings when planning fixes and verifying candidates; preserve
  ID3 locator frames through native conversion.
- Preserve audio symlinks during edits and reject hard-linked audio replacement.
  Audio replacement currently requires Unix hard-link checks.

- Remove the separate history crate; core now owns the concrete history model
  and persistence. Invalid databases and unsupported schema versions fail
  before actions execute. Validation fixes now load history before changing
  tags. Legacy template text remains unchanged and retains reuse restrictions.
- Raise the minimum supported Rust version to 1.91.0 and use the standard
  UTF-8 boundary function for template diagnostic positions.

- Update Lofty to 0.25.4 and preserve error causes for audio reads and writes.

- Require canonical lowercase snake_case audio tag names, with explicit aliases
  `album`, `artist`, `title`, `album_sort`, and `disk_number`. Remove generated
  casing aliases and use canonical names for new tag-fix history. Historical
  JSON histories are rejected without importing or rewriting them. The template
  library preserves metadata name spelling.

- Use workspace Cargo lint settings consistently across all crates; remove
  duplicate crate-level Clippy configuration and retain the test harness's
  specific `must_use_candidate` exception.

- Share tag sanitization between rendering and character fixes, normalize tag
  alias lookup, and use consistent template labels in diagnostics. Preserve
  numeric overflow and whitespace handling.

- Use template terminology throughout the API, including `Template` and
  `BoundTemplate`; preserve the `--script` flag and stored-action replay in supported databases.

- Compile and bind only the selected rename template; reject ambiguous
  filename stems and allow selection by explicit file path. Remove the
  core template wrappers in favor of the language crate's template types.

- Replaced MiniJinja and TOML frontmatter with the path template language.
  Existing templates require manual migration; declarations and path rules now
  share one document, and inline templates use `path: (...)`.
- Treat zero as present in guards and fallbacks. Stef's migrated layout omits
  the filename artist prefix when the track artist is missing or empty.
- Validate all string/path arguments and defaults against forbidden filename
  characters instead of sanitizing them; preserve accepted argument whitespace.
- Reject old JSON history without import; move it aside explicitly to start new
  history. Saved Jinja text in supported databases requires an explicit
  replacement when reused for a new rename.
- Resolve relative rename input directories before scanning and compare canonical
  paths during cleanup so renamed targets survive alternate path spellings.
- Read separate track/disc/movement total fields and skip empty date sources
  when selecting a fallback date. Preserve final dot components when appending
  the source extension.

- Changed validation commands to use validation types directly:
  `tfmt validate`, `tfmt validate characters`, and
  `tfmt validate id3-encoding`.
- Changed validation fixes to use `--fix`, such as
  `tfmt validate characters --fix` and
  `tfmt validate id3-encoding --fix`.
- Removed the `validate fix id3-encoding --encoding` option; matching ID3 text
  frames are now always rewritten as UTF-16.

## 0.24.0 - 2026-04-30

### Added

- Added release planning for Forgejo source releases.
- Added shared CI and release packaging scripts.
- Added checksum generation for release artifacts.
- Added example templates to release archives.
- Added Forgejo workflow wrappers for Linux checks.
- Added support for in-situ renames, including swaps and cycles.
- Added support for case-only renames on case-insensitive filesystems.
- Added Windows-compatible target path validation.

### Changed

- Rename execution uses temporary staging paths when a plan has source-target
  dependencies.
- Rendered rename plans reject targets that differ only by case.

### Fixed

- Rejected Windows reserved device names in target path components.

### Known Limitations

- Rename operations are preflight-validated but not fully transactional.
- Target paths use a conservative cross-platform length limit.
- Windows support is best-effort. Windows binaries are not release artifacts,
  and Windows-specific filesystem behavior is not a compatibility promise.
