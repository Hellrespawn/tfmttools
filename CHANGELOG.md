# Changelog

## Unreleased

## 0.24.0 - YYYY-MM-DD

### Added

- Added release planning for GitHub and Forgejo source releases.
- Added shared CI and release packaging scripts.
- Added per-platform checksum generation for release artifacts.
- Added example templates to release archives.
- Added GitHub and Forgejo workflow wrappers for Linux and Windows checks.
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
- Windows support targets normal NTFS usage. Long-path mode, network shares,
  unusual Unicode normalization, and cross-volume edge cases are not fully
  covered.
