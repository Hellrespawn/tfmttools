use color_eyre::Result;
use rusqlite::types::{
    FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef,
};
use rusqlite::{Row, ToSql};
use tfmttools_core::action::Action;

use crate::conn::Connection;

#[repr(i64)]
#[derive(Clone, Copy)]
pub enum ActionType {
    MoveFile = 0,
    CopyFile = 1,
    RemoveFile = 2,
    MakeDir = 3,
    RemoveDir = 4,
}

impl From<&Action> for ActionType {
    fn from(value: &Action) -> Self {
        match value {
            Action::MoveFile(..) => Self::MoveFile,
            Action::CopyFile(..) => Self::CopyFile,
            Action::RemoveFile(..) => Self::RemoveFile,
            Action::MakeDir(..) => Self::MakeDir,
            Action::RemoveDir(..) => Self::RemoveDir,
        }
    }
}

impl TryFrom<i64> for ActionType {
    type Error = FromSqlError;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        match value {
            0..=4 => {
                Ok(unsafe { std::mem::transmute::<i64, ActionType>(value) })
            },
            n => {
                Err(FromSqlError::Other(
                    (format!("Invalid RecordState '{}'", n)).into(),
                ))
            },
        }
    }
}

impl FromSql for ActionType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        if let ValueRef::Integer(integer) = value {
            ActionType::try_from(integer)
        } else {
            Err(FromSqlError::InvalidType)
        }
    }
}

impl ToSql for ActionType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let int = *self as i64;

        Ok(ToSqlOutput::Owned(int.into()))
    }
}

pub struct ActionEntity {
    id: isize,
    action_type: ActionType,
    target: String,
    source: Option<String>,
    record_id: isize,
}

impl ActionEntity {
    pub fn insert(
        conn: &mut Connection,
        action_type: ActionType,
        target: String,
        source: Option<String>,
        record_id: i64,
    ) -> rusqlite::Result<Self> {
        todo!()
    }

    pub fn insert_from_action(
        conn: &mut Connection,
        action: &Action,
        record_id: i64,
    ) -> rusqlite::Result<Self> {
        let action_type = action.into();
        let target = action.target().to_string();
        let source = action.source().map(|s| s.to_string());

        Self::insert(conn, action_type, target, source, record_id)
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            action_type: row.get(1)?,
            target: row.get(2)?,
            source: row.get(3)?,
            record_id: row.get(4)?,
        })
    }
}
