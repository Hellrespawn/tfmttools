use std::sync::LazyLock;

use include_dir::include_dir;
use rusqlite_migration::{M, Migrations};

use crate::Connection;

// Define migrations. These are applied atomically.
static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::from_iter(
        include_dir!("$CARGO_MANIFEST_DIR/migrations").files().map(|file| {
            M::up(file.contents_utf8().unwrap_or_else(|| {
                panic!(
                    "Migration {} is not UTF-8 encoded.",
                    file.path().to_string_lossy()
                )
            }))
        }),
    )
});

pub fn migrate_database(
    connection: &mut Connection,
) -> rusqlite_migration::Result<()> {
    // Update the database schema, atomically
    MIGRATIONS.to_latest(&mut connection.0)?;
    Ok(())
}
