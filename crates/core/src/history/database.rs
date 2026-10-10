use std::collections::BTreeMap;

use camino::Utf8Path;
use rusqlite::{Connection, OpenFlags, Row};
use rusqlite_migration::{M, Migrations};

use super::{
    ActionRecordMetadata, HistoryError, Record, RecordState, Result,
    StoredAction,
};
use crate::action::Action;

pub(super) const APPLICATION_ID: i64 = 0x5446_4d54;
pub(super) const VERSION: i64 = 1;
const SCHEMA: &str = include_str!("schema-v1.sql");

#[must_use]
pub fn history_schema_sql() -> &'static str {
    SCHEMA
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(SCHEMA)])
}

pub(super) fn open(
    path: &Utf8Path,
    read_only: bool,
    new: bool,
) -> Result<Connection> {
    let flags = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else if new {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };
    let mut connection = Connection::open_with_flags(path, flags)?;
    if !new {
        let application: i64 =
            connection
                .pragma_query_value(None, "application_id", |row| row.get(0))?;
        if application != APPLICATION_ID {
            return Err(HistoryError::LoadError(
                "Foreign history database (application_id differs)".into(),
            ));
        }
        let version: i64 =
            connection
                .pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > VERSION {
            return Err(HistoryError::LoadError(format!(
                "Unsupported history database version {version}"
            )));
        }
        if version < VERSION {
            return Err(HistoryError::LoadError(
                "History database requires an unsupported migration".into(),
            ));
        }
        validate_schema(&connection)?;
    }
    connection.pragma_update(None, "foreign_keys", "ON")?;
    let keys: i64 =
        connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    if keys != 1 {
        return Err(HistoryError::LoadError(
            "Foreign key enforcement unavailable".into(),
        ));
    }
    if !read_only {
        let mode: String =
            connection.query_row("PRAGMA journal_mode=DELETE", [], |row| {
                row.get(0)
            })?;
        connection.pragma_update(None, "synchronous", "EXTRA")?;
        let synchronous: i64 =
            connection
                .pragma_query_value(None, "synchronous", |row| row.get(0))?;
        if mode != "delete" || synchronous != 3 {
            return Err(HistoryError::SaveError(
                "Required SQLite durability settings unavailable".into(),
            ));
        }
    }
    if new {
        migrations()
            .to_latest(&mut connection)
            .map_err(|e| HistoryError::SaveError(e.to_string()))?;
        validate_schema(&connection)?;
    }
    Ok(connection)
}

fn schema_objects(connection: &Connection) -> Result<BTreeMap<String, String>> {
    let mut statement = connection.prepare("SELECT name, sql FROM sqlite_schema WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name")?;
    Ok(statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?)
}

pub(super) fn validate_schema(connection: &Connection) -> Result<()> {
    let mut expected = Connection::open_in_memory()?;
    migrations()
        .to_latest(&mut expected)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    if schema_objects(connection)? != schema_objects(&expected)? {
        return Err(HistoryError::LoadError(
            "History database schema differs from the versioned contract"
                .into(),
        ));
    }
    let check: String =
        connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if check != "ok" {
        return Err(HistoryError::LoadError(format!(
            "History database integrity check: {check}"
        )));
    }
    if connection
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some()
    {
        return Err(HistoryError::LoadError(
            "History database has foreign key violations".into(),
        ));
    }
    Ok(())
}

pub(super) struct DatabaseRecord {
    pub record: Record,
    pub finalized: bool,
}

pub(super) fn read_records(
    connection: &Connection,
    finalized_only: bool,
) -> Result<Vec<DatabaseRecord>> {
    let mut statement = connection.prepare("SELECT id, position, state, timestamp, metadata, finalized FROM records WHERE finalized=1 OR ?1=0 ORDER BY position")?;
    let mut rows = statement.query([i64::from(finalized_only)])?;
    let mut records = Vec::new();
    while let Some(row) = rows.next()? {
        records.push(read_record_row(connection, row)?);
    }
    Ok(records)
}

pub(super) fn read_record(
    connection: &Connection,
    id: usize,
) -> Result<DatabaseRecord> {
    let id = i64::try_from(id)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    let mut statement = connection.prepare("SELECT id, position, state, timestamp, metadata, finalized FROM records WHERE id=?1")?;
    let mut rows = statement.query([id])?;
    let row = rows.next()?.ok_or_else(|| {
        HistoryError::LoadError("Pending record missing".into())
    })?;
    read_record_row(connection, row)
}

fn read_record_row(
    connection: &Connection,
    row: &Row<'_>,
) -> Result<DatabaseRecord> {
    let id: i64 = row.get(0)?;
    let position: i64 = row.get(1)?;
    if id < 0 || position < 0 {
        return Err(HistoryError::LoadError("Negative record identity".into()));
    }
    let state: String = row.get(2)?;
    let state = match state.as_str() {
        "applied" => RecordState::Applied,
        "undone" => RecordState::Undone,
        "redone" => RecordState::Redone,
        "superseded" => RecordState::Superseded,
        _ => {
            return Err(HistoryError::LoadError("Unknown record state".into()));
        },
    };
    let timestamp: String = row.get(3)?;
    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    let metadata: String = row.get(4)?;
    let metadata: ActionRecordMetadata = serde_json::from_str(&metadata)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    let mut actions_statement = connection.prepare("SELECT position, payload FROM actions WHERE record_id=?1 ORDER BY position")?;
    let mut action_rows = actions_statement.query([id])?;
    let mut actions = Vec::new();
    while let Some(action_row) = action_rows.next()? {
        let action_position: i64 = action_row.get(0)?;
        if usize::try_from(action_position).ok() != Some(actions.len()) {
            return Err(HistoryError::LoadError(
                "Noncontiguous action ordering".into(),
            ));
        }
        let payload: String = action_row.get(1)?;
        let action: StoredAction = serde_json::from_str(&payload)
            .map_err(|e| HistoryError::LoadError(e.to_string()))?;
        validate_action(&action)?;
        actions.push(action);
    }
    let record = Record::from_storage(
        usize::try_from(id)
            .map_err(|e| HistoryError::LoadError(e.to_string()))?,
        actions,
        state,
        timestamp,
        metadata,
    );
    Ok(DatabaseRecord { record, finalized: row.get::<_, i64>(5)? == 1 })
}

pub(super) fn read_action(
    connection: &Connection,
    record_id: usize,
    position: usize,
) -> Result<StoredAction> {
    let record_id = i64::try_from(record_id)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    let position = i64::try_from(position)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    let payload: String = connection.query_row(
        "SELECT payload FROM actions WHERE record_id=?1 AND position=?2",
        rusqlite::params![record_id, position],
        |row| row.get(0),
    )?;
    let action = serde_json::from_str(&payload)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    validate_action(&action)?;
    Ok(action)
}

pub(super) fn validate_action(action: &StoredAction) -> Result<()> {
    let executable = Action::try_from(action)?;
    if executable.target().as_str().is_empty()
        || executable.source().is_some_and(|p| p.as_str().is_empty())
    {
        return Err(HistoryError::LoadError("Empty action path".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrations_are_valid() {
        migrations().validate().unwrap();
    }
    #[test]
    fn required_settings_apply_to_each_connection() {
        let directory = tempfile::tempdir().unwrap();
        let path =
            camino::Utf8PathBuf::from_path_buf(directory.path().join("h.db"))
                .unwrap();
        for new in [true, false] {
            let connection = open(&path, false, new).unwrap();
            assert_eq!(
                connection
                    .pragma_query_value(None, "journal_mode", |r| {
                        r.get::<_, String>(0)
                    })
                    .unwrap(),
                "delete"
            );
            assert_eq!(
                connection
                    .pragma_query_value(None, "synchronous", |r| {
                        r.get::<_, i64>(0)
                    })
                    .unwrap(),
                3
            );
            assert_eq!(
                connection
                    .pragma_query_value(None, "foreign_keys", |r| {
                        r.get::<_, i64>(0)
                    })
                    .unwrap(),
                1
            );
            assert!(connection.execute("INSERT INTO records(id,position,state,timestamp,metadata,finalized) VALUES('bad',0,'applied','bad','{}',1)", []).is_err());
            assert!(connection.execute("INSERT INTO actions(record_id,position,payload) VALUES(42,0,'{}')", []).is_err());
        }
    }
}
