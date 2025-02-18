use camino::Utf8Path;
use color_eyre::Result;
use rusqlite::Connection as SqliteConnection;

pub struct Connection(pub SqliteConnection);

impl Connection {
    pub fn open(path: &Utf8Path) -> Result<Connection> {
        let mut conn = SqliteConnection::open(path)?;

        Self::init_database(&mut conn)?;

        Ok(Connection(conn))
    }

    pub fn in_memory() -> Result<Connection> {
        let mut conn = SqliteConnection::open_in_memory()?;

        Self::init_database(&mut conn)?;

        Ok(Connection(conn))
    }

    fn init_database(conn: &mut SqliteConnection) -> Result<()> {
        conn.execute_batch(
            "
    PRAGMA journal_mode = wal;
    PRAGMA synchronous = normal;
    PRAGMA foreign_keys = on;
    ",
        )?;

        crate::migration::migrate_database(conn)?;

        Ok(())
    }

    fn close_database(&mut self) -> Result<()> {
        let _ = self.0.execute_batch(
            "
    PRAGMA analysis_limit=400; -- make sure pragma optimize does not take too long
    PRAGMA optimize; -- gather statistics to improve query optimization
    ",
        );

        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.close_database();
    }
}
