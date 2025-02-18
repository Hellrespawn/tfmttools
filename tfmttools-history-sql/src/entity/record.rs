use chrono::NaiveDateTime;
use rusqlite::types::{
    FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef,
};
use rusqlite::{Row, ToSql};

use super::action::ActionEntity;
use crate::Connection;

#[repr(i64)]
#[derive(Clone, Copy)]
pub enum RecordState {
    Applied = 0,
    Undone = 1,
    Redone = 2,
    Superseded = 3,
}

impl TryFrom<i64> for RecordState {
    type Error = FromSqlError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0..=3 => {
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
    id: i64,
    state: RecordState,
    datetime: NaiveDateTime,
    template: String,
    arguments: String,
    superseded_by_id: Option<i64>,
}

impl RecordEntity {
    fn get_actions(
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

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            state: row.get(1)?,
            datetime: row.get(2)?,
            template: row.get(3)?,
            arguments: row.get(4)?,
            superseded_by_id: row.get(5)?,
        })
    }
}
