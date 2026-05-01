# Changelog

## Unreleased

### Added

- Added generated shell completions to release archives and Arch packages.
- Added generated man pages to release archives and Arch packages.

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
