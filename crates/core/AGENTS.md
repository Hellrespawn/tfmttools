# Core Crate Guidance

This crate owns the pure rename model: rename actions, validation,
template rendering, audio item metadata, item keys, history data shared
with other crates, and UTF-8 path helpers.

## Task Map

- Rename action model: `src/action/rename_action.rs`.
- Rename validation rules: `src/action/validation.rs`.
- Case-insensitive path handling: `src/action/case_insensitive_path.rs`.
- Template rendering context and template wrapper: `src/templates/`.
- Audio metadata model: `src/audiofile.rs`.
- Item key constants: `src/item_keys.rs`.
- Shared history types: `src/history.rs`.
- Shared UTF-8 path helpers: `src/util.rs`.

## Verification

- Core-only changes: `cargo test -p tfmttools-core`.
- If validation affects CLI behavior, also run
  `cargo test -p tfmttools-cli`.
