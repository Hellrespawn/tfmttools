# Core Crate Guidance

This crate owns rename actions, validation, template rendering, audio
metadata, item keys, UTF-8 path helpers, and concrete history storage.
History IO lives here; executable filesystem operations live in `fs`.

## Task Map

- Rename action model: `src/action/rename_action.rs`.
- Rename validation rules: `src/action/validation.rs`.
- Case-insensitive path handling: `src/action/case_insensitive_path.rs`.
- Template rendering context and template wrapper: `src/templates/`.
- Audio metadata model: `src/audiofile.rs`.
- Item key constants: `src/item_keys.rs`.
- History model and persistence: `src/history/`.
- Shared UTF-8 path helpers: `src/util.rs`.

## Verification

- Core-only changes: `cargo test -p tfmttools-core`.
- If validation affects CLI behavior, also run
  `cargo test -p tfmt`.

## History compatibility

When changing stored history, read
`../../docs/superpowers/specs/2026-10-10-manual-interruption-handling-design.md`
and its referenced patch/storage contract.
The SQLite schema has not been deployed: edit the initial schema directly.
Future published format changes need explicit compatibility handling. Run `cargo xtask history-schema` to
regenerate the snapshot; core tests compare it automatically.

SQLite migrations own `user_version`; keep `application_id` and strict validation.
Use `journal_mode=DELETE`, `synchronous=EXTRA`, and `foreign_keys=ON`.
`src/history/schema-v1.sql` generates `docs/history/schema-v1.sql`.
Current attempts are reporting data for explicit history-only resolution.
Keep filesystem outcome inference and automatic recovery out of core.
Do not import old JSON histories or persist tag actions without binary patches.
