# Filesystem Crate Guidance

This crate owns filesystem effects and filesystem-facing helpers:
action execution, template loading from disk, path iteration, checksums,
direct filesystem operations, and rename staging/planning.

## Task Map

- Planner and prepared-action entry points: `src/action.rs`.
- Rename staging and planning internals: `src/action/`.
- Filesystem mode and path helpers: `src/fs_handler.rs`.
- Template loading from config directories: `src/template.rs`.
- Path discovery: `src/path_iterator.rs`.
- File and tree checksums: `src/checksum.rs`.
- File-or-name path matching helper: `src/file_or_name.rs`.

## Verification

- Filesystem action/planning changes: `cargo test -p tfmttools-fs`.
- If behavior is user-visible through the CLI, also run
  `cargo test -p tfmt`.
