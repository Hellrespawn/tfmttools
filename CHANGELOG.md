# Changelog

## Unreleased

### Added

- Added an independent `tfmttools-picotmpl` workspace crate implementing the new
  path template language, with a Stef layout example and behavior tests.
- Added generated shell completions to release archives and Arch packages.
- Added generated man pages to release archives and Arch packages.
- Added `tfmt validate id3-encoding` to report non-ASCII ID3 text frames
  stored as UTF-8.

### Changed

- Require canonical lowercase snake_case audio tag names, with explicit aliases
  `album`, `artist`, `title`, `album_sort`, and `disk_number`. Remove generated
  casing aliases and use canonical names for new tag-fix history. Existing
  history using old key spellings cannot be replayed; clear it with
  `tfmt clear-history`. The template library preserves metadata name spelling.

- Use workspace Cargo lint settings consistently across all crates; remove
  duplicate crate-level Clippy configuration and retain the test harness's
  specific `must_use_candidate` exception.

- Share tag sanitization between rendering and character fixes, normalize tag
  alias lookup, and use consistent template labels in diagnostics. Preserve
  numeric overflow and whitespace handling.

- Use template terminology throughout the API, including `Template` and
  `BoundTemplate`; preserve the `--script` flag and history serialization.

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
- Preserve legacy history undo/redo; saved Jinja templates require an explicit
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
