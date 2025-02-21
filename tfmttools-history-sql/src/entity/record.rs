use chrono::{DateTime, Local, NaiveDateTime};
use rusqlite::types::{
    FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef,
};
use rusqlite::{OptionalExtension, Row, ToSql};
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_history_core::Record;

use super::action::ActionEntity;
use crate::Connection;

#[repr(i64)]
#[derive(Clone, Copy)]
pub enum RecordState {
    Applied = 0,
    Undone = 1,
    Redone = 2,
    Superseded = 3,
    TransactionError = 4,
}

impl TryFrom<i64> for RecordState {
    type Error = FromSqlError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0..=4 => {
                Ok(unsafe { std::mem::transmute::<i64, RecordState>(value) })
            },
            n => {
                Err(FromSqlError::Other(
                    (format!("Invalid RecordState '{}'", n)).into(),
                ))
            },
        }
    }
}

impl FromSql for RecordState {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Integer(integer) = value {
            RecordState::try_from(integer)
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

impl ToSql for RecordState {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let int = *self as i64;

        Ok(ToSqlOutput::Owned(int.into()))
    }
}

pub struct RecordEntity {
    pub id: i64,
    pub state: RecordState,
    pub datetime: DateTime<Local>,
    pub template: String,
    pub arguments: String,
}

impl RecordEntity {
    pub fn insert(
        conn: &mut Connection,
        template: &str,
        arguments: &str,
    ) -> rusqlite::Result<Self> {
        let mut stmt = conn.0.prepare("INSERT INTO records (state, datetime, template, arguments) VALUES (?1  ?2  ?3 ?4) RETURNING id")?;

        let params = (RecordState::Applied, Local::now(), template, arguments);

        let id: i64 = stmt.query_row(params, |row| row.get(0))?;

        Ok(Self {
            id,
            state: RecordState::Applied,
            datetime: Local::now(),
            template: template.to_owned(),
            arguments: arguments.to_owned(),
        })
    }

    pub fn get_previous(
        conn: &mut Connection,
    ) -> rusqlite::Result<Option<RecordEntity>> {
        conn.0
            .prepare("SELECT * FROM records ORDER BY datetime DESC LIMIT 1")?
            .query_row([], RecordEntity::from_row)
            .optional()
    }

    pub fn get_actions(
        &self,
        conn: &mut Connection,
    ) -> rusqlite::Result<Vec<ActionEntity>> {
        let mut stmt =
            conn.0.prepare("SELECT * FROM actions WHERE record_id = ?")?;

        let actions = stmt
            .query_map([self.id], ActionEntity::from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(actions)
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            state: row.get(1)?,
            datetime: row.get(2)?,
            template: row.get(3)?,
            arguments: row.get(4)?,
        })
    }
}

impl From<RecordEntity> for Record<Action, ActionRecordMetadata> {
    fn from(entity: RecordEntity) -> Self {
        todo!()
    }
}
