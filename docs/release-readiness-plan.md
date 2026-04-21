# Release Readiness Plan

## Release target

- Release version: `0.24.0`.
- Distribution channels: GitHub and Forgejo.
- Package type: source archives and prebuilt CLI binaries, not crates.io.
- Supported release artifacts:
  - Linux x86_64.
  - Windows x86_64.
- Windows support target: normal NTFS usage. Long-path mode, network shares,
  unusual Unicode normalization, and cross-volume edge cases are not full
  compatibility promises for this release.

## Release blockers

1. Support in-situ renames safely.
   - Handle swaps such as `A.mp3 -> B.mp3` and `B.mp3 -> A.mp3`.
   - Handle cycles such as `A.mp3 -> B.mp3`, `B.mp3 -> C.mp3`,
     `C.mp3 -> A.mp3`.
   - Handle case-only renames such as `track.mp3 -> Track.mp3`, including on
     case-insensitive filesystems.
   - Use temporary paths internally where direct moves are unsafe.
   - Record actual filesystem moves so undo/redo stays reliable.

2. Strengthen cross-platform path validation.
   - Reject exact target collisions.
   - Reject targets that differ only by case, except for a single case-only
     rename of the same source path.
   - Reject Windows reserved device names in every path component on all
     platforms: `CON`, `PRN`, `AUX`, `NUL`, `COM1` through `COM9`, and
     `LPT1` through `LPT9`, including names with extensions.
   - Keep the conservative path length limit for this release.
   - Keep deterministic sanitization of interpolated tag values, then validate
     the final rendered path before any filesystem mutation.

3. Add automated tests.
   - Unit tests for path validation.
   - Filesystem-level tests for swaps, cycles, chains, and case-only renames.
   - CLI integration coverage for undo/redo after in-situ renames where
     practical.
   - Run the integration suite on Windows CI.

4. Add release automation.
   - Shared scripts are the source of truth.
   - GitHub and Forgejo workflows should be thin wrappers.
   - CI runs test, fmt check, clippy, and release build on Linux and Windows.
   - Tag workflows build artifacts, but first-release publication remains
     manual.

5. Update user-facing docs.
   - README quick start for Linux and Windows.
   - Template examples and supported tags.
   - Dry-run, history, undo, and redo behavior.
   - Filename sanitization and validation rules.
   - Safety model and non-transactional limitation.
   - `CHANGELOG.md` and `RELEASE.md`.

## Safety policy

- `tfmt` preflight-validates rename plans before modifying files.
- A full transaction system is not part of `0.24.0`.
- If an unexpected filesystem error occurs mid-run, completed actions are
  recorded where possible and can be reverted with `tfmt undo`.
- Users should run `--dry-run` before applying large rename plans.

## Automation policy

- `scripts/ci.sh` runs the normal quality gate:
  - `cargo test --workspace`
  - `cargo +nightly fmt --all --check`
  - `cargo +nightly clippy --workspace --all-targets`
  - `cargo build --release`
- `scripts/package-release.sh` creates OS-specific archives in `dist/`.
- Release archives include the binary, `README.md`, `LICENSE`,
  `CHANGELOG.md`, and `examples/`.
- `scripts/package-release.sh` writes a per-platform checksum file for the
  archive it creates.
- The final manually published release should combine per-platform checksum
  files into one `SHA256SUMS`.
- GitHub and Forgejo workflows call the shared scripts.
- Release upload and release notes publication remain manual for `0.24.0`.
