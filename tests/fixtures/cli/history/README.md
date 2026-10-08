# History rejection fixtures

These fixed historical JSON documents exercise rejection without import or file
changes. They are intentionally independent of current serialization. Supported
SQLite replay and interrupted-operation tests live in
`crates/tfmt/tests/history_recovery.rs` and core history database/journal tests.
The fixture-backed validation cases also exercise exact recorded replay.
