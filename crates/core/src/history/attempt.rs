use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::{
    ActionRecordMetadata, BinaryPatchPair, ByteIdentity, History, HistoryError,
    Record, Result, StoredAction, database,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Apply,
    Undo,
    Redo,
}
impl OperationKind {
    fn name(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Undo => "undo",
            Self::Redo => "redo",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s {
            "apply" => Ok(Self::Apply),
            "undo" => Ok(Self::Undo),
            "redo" => Ok(Self::Redo),
            _ => Err(invalid("Unknown operation kind")),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum AttemptOutcome {
    Applied,
    NotApplied,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptDetails {
    pub paths: Vec<String>,
    pub instructions: String,
}
#[derive(Debug, Clone)]
pub struct CurrentAttempt {
    pub id: AttemptId,
    pub run_id: RunId,
    pub record_id: usize,
    pub kind: OperationKind,
    pub action_position: usize,
    pub action: StoredAction,
    pub details: AttemptDetails,
    pub patches: Option<BinaryPatchPair>,
}
fn invalid(s: &str) -> HistoryError {
    HistoryError::MutError(s.into())
}
fn number(n: usize) -> Result<i64> {
    i64::try_from(n).map_err(|e| invalid(&e.to_string()))
}
fn index(n: i64) -> Result<usize> {
    usize::try_from(n).map_err(|e| invalid(&e.to_string()))
}
fn encode<T: Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v).map_err(|e| invalid(&e.to_string()))
}
fn decode<T: serde::de::DeserializeOwned>(v: &str) -> Result<T> {
    serde_json::from_str(v).map_err(|e| invalid(&e.to_string()))
}

impl History {
    fn connection(&self) -> Result<&Connection> {
        self.connection.as_ref().ok_or_else(|| invalid("History is not open"))
    }

    fn refresh(&mut self) -> Result<()> {
        self.records = database::read_records(self.connection()?)?
            .into_iter()
            .map(|r| r.record)
            .collect();
        Ok(())
    }

    pub fn begin_run(
        &mut self,
        kind: OperationKind,
        record_id: Option<usize>,
        metadata: Option<ActionRecordMetadata>,
    ) -> Result<RunId> {
        self.ensure_connection()?;
        let tx = self.connection.as_mut().unwrap().transaction()?;
        if tx.query_row("SELECT count(*) FROM runs", [], |r| {
            r.get::<_, i64>(0)
        })? != 0
        {
            return Err(invalid(
                "Previous run requires manual history resolution",
            ));
        }
        let record_id = match kind {
            OperationKind::Apply => {
                if record_id.is_some() {
                    return Err(invalid("New run must not select a record"));
                }
                let metadata = metadata
                    .ok_or_else(|| invalid("New run requires metadata"))?;
                let id: i64 = tx.query_row(
                    "SELECT coalesce(max(id)+1,0) FROM records",
                    [],
                    |r| r.get(0),
                )?;
                let pos: i64 = tx.query_row(
                    "SELECT coalesce(max(position)+1,0) FROM records",
                    [],
                    |r| r.get(0),
                )?;
                tx.execute("INSERT INTO records(id,position,state,timestamp,metadata,complete,applied_count,redo_allowed) VALUES(?1,?2,'applied',?3,?4,0,0,1)", params![id,pos,chrono::Local::now().fixed_offset().to_rfc3339(),encode(&metadata)?])?;
                index(id)?
            },
            OperationKind::Undo | OperationKind::Redo => {
                let id = record_id
                    .ok_or_else(|| invalid("Replay requires a record"))?;
                let record = database::read_record(&tx, id)?.record;
                let eligible = match kind {
                    OperationKind::Undo => record.applied_count() > 0,
                    OperationKind::Redo => {
                        record.redo_allowed()
                            && record.applied_count() < record.len()
                            && record.state() != super::RecordState::Superseded
                    },
                    OperationKind::Apply => unreachable!(),
                };
                if !eligible {
                    return Err(invalid("Record is not eligible for replay"));
                }
                id
            },
        };
        tx.execute("INSERT INTO runs(record_id,kind) VALUES(?1,?2)", params![
            number(record_id)?,
            kind.name()
        ])?;
        let id = RunId(tx.last_insert_rowid());
        tx.commit()?;
        self.refresh()?;
        Ok(id)
    }

    pub fn begin_attempt(
        &mut self,
        run: RunId,
        action_position: usize,
        action: &StoredAction,
        details: &AttemptDetails,
        patches: Option<&BinaryPatchPair>,
    ) -> Result<AttemptId> {
        self.ensure_connection()?;
        database::validate_action(action)?;
        if details.paths.is_empty()
            || details.paths.iter().any(String::is_empty)
            || details.instructions.is_empty()
        {
            return Err(invalid(
                "Attempt requires paths and inspection instructions",
            ));
        }
        let tx = self.connection.as_mut().unwrap().transaction()?;
        let (rid, kind) = run_info(&tx, run)?;
        let record = database::read_record(&tx, rid)?.record;
        let expected = match kind {
            OperationKind::Apply => record.len(),
            OperationKind::Undo => {
                record
                    .applied_count()
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Nothing to undo"))?
            },
            OperationKind::Redo => record.applied_count(),
        };
        if action_position != expected {
            return Err(invalid("Attempt position differs from replay cursor"));
        }
        if kind != OperationKind::Apply
            && record.actions().get(expected) != Some(action)
        {
            return Err(invalid("Replay action differs from history"));
        }
        if matches!(action, StoredAction::EditTagValues { .. })
            != patches.is_some()
        {
            return Err(invalid("Tag actions require binary patches only"));
        }
        if let Some(pair) = patches {
            validate_patch(pair)?;
            if kind != OperationKind::Apply
                && read_patch(&tx, rid, expected)?.as_ref() != Some(pair)
            {
                return Err(invalid("Replay patches differ from history"));
            }
        }
        tx.execute("INSERT INTO attempts(run_id,action_position,payload,details) VALUES(?1,?2,?3,?4)", params![run.0,number(action_position)?,encode(action)?,encode(details)?])?;
        let id = AttemptId(tx.last_insert_rowid());
        if let Some(pair) = patches {
            insert_attempt_patch(&tx, id, pair)?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn current_attempt(&self) -> Result<Option<CurrentAttempt>> {
        self.connection.as_ref().map_or(Ok(None), read_current_attempt)
    }

    pub fn confirm_attempt(&mut self, id: AttemptId) -> Result<()> {
        self.ensure_connection()?;
        let a = self
            .current_attempt()?
            .filter(|a| a.id == id)
            .ok_or_else(|| invalid("Unknown attempt"))?;
        let tx = self.connection.as_mut().unwrap().transaction()?;
        confirm(&tx, &a)?;
        tx.commit()?;
        self.refresh()
    }

    pub fn close_run(&mut self, id: RunId, complete: bool) -> Result<Record> {
        self.ensure_connection()?;
        let tx = self.connection.as_mut().unwrap().transaction()?;
        let (rid, kind) = run_info(&tx, id)?;
        if tx.query_row(
            "SELECT count(*) FROM attempts WHERE run_id=?1",
            [id.0],
            |r| r.get::<_, i64>(0),
        )? != 0
        {
            return Err(invalid("Attempt requires manual resolution"));
        }
        if kind == OperationKind::Apply {
            tx.execute(
                "UPDATE records SET complete=?2,redo_allowed=?2 WHERE id=?1",
                params![number(rid)?, complete],
            )?;
        }
        if !complete {
            tx.execute("UPDATE records SET redo_allowed=0 WHERE id=?1", [
                number(rid)?,
            ])?;
        }
        tx.execute("DELETE FROM runs WHERE id=?1", [id.0])?;
        let record = database::read_record(&tx, rid)?.record;
        tx.commit()?;
        self.refresh()?;
        Ok(record)
    }

    /// Stored run identity for read-only interruption reporting.
    pub fn open_run(&self) -> Result<Option<(RunId, usize, OperationKind)>> {
        let Some(c) = &self.connection else { return Ok(None) };
        let row: Option<(i64, i64, String)> = c
            .query_row("SELECT id,record_id,kind FROM runs", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .optional()?;
        row.map(|(id, rid, kind)| {
            Ok((RunId(id), index(rid)?, OperationKind::parse(&kind)?))
        })
        .transpose()
    }

    pub fn close_abandoned_run(&mut self) -> Result<Option<Record>> {
        if self.current_attempt()?.is_some() {
            return Err(invalid("Attempt requires manual resolution"));
        }
        let Some(c) = &self.connection else { return Ok(None) };
        let id: Option<i64> =
            c.query_row("SELECT id FROM runs", [], |r| r.get(0)).optional()?;
        id.map(|id| self.close_run(RunId(id), false)).transpose()
    }

    pub fn resolve_attempt(
        &mut self,
        id: AttemptId,
        outcome: AttemptOutcome,
    ) -> Result<Record> {
        let a = self
            .current_attempt()?
            .filter(|a| a.id == id)
            .ok_or_else(|| invalid("Unknown attempt"))?;
        self.ensure_connection()?;
        let tx = self.connection.as_mut().unwrap().transaction()?;
        match outcome {
            AttemptOutcome::Applied => confirm(&tx, &a)?,
            AttemptOutcome::NotApplied => {
                tx.execute("DELETE FROM attempts WHERE id=?1", [id.0])?;
            },
        }
        tx.execute("UPDATE records SET redo_allowed=0,complete=CASE WHEN ?2='apply' THEN 0 ELSE complete END WHERE id=?1",params![number(a.record_id)?,a.kind.name()])?;
        tx.execute("DELETE FROM runs WHERE id=?1", [a.run_id.0])?;
        let record = database::read_record(&tx, a.record_id)?.record;
        tx.commit()?;
        self.refresh()?;
        Ok(record)
    }

    pub fn patches(
        &self,
        record_id: usize,
        position: usize,
    ) -> Result<Option<BinaryPatchPair>> {
        self.connection
            .as_ref()
            .map_or(Ok(None), |c| read_patch(c, record_id, position))
    }
}
pub(super) fn read_current_attempt(
    c: &Connection,
) -> Result<Option<CurrentAttempt>> {
    let mut s = c.prepare("SELECT a.id,a.run_id,r.record_id,r.kind,a.action_position,a.payload,a.details FROM attempts a JOIN runs r ON r.id=a.run_id")?;
    let mut rows = s.query([])?;
    let Some(r) = rows.next()? else { return Ok(None) };
    let id = AttemptId(r.get(0)?);
    let action: StoredAction = decode(&r.get::<_, String>(5)?)?;
    database::validate_action(&action)?;
    let details: AttemptDetails = decode(&r.get::<_, String>(6)?)?;
    let patches = read_attempt_patch(c, id)?;
    if matches!(action, StoredAction::EditTagValues { .. }) != patches.is_some()
    {
        return Err(invalid("Attempt patch data differs from action"));
    }
    let attempt = CurrentAttempt {
        id,
        run_id: RunId(r.get(1)?),
        record_id: index(r.get(2)?)?,
        kind: OperationKind::parse(&r.get::<_, String>(3)?)?,
        action_position: index(r.get(4)?)?,
        action,
        details,
        patches,
    };
    validate_attempt(c, &attempt)?;
    Ok(Some(attempt))
}

fn validate_attempt(c: &Connection, a: &CurrentAttempt) -> Result<()> {
    let record = database::read_record(c, a.record_id)?.record;
    let expected = match a.kind {
        OperationKind::Apply => Some(record.len()),
        OperationKind::Undo => record.applied_count().checked_sub(1),
        OperationKind::Redo => Some(record.applied_count()),
    };
    if expected != Some(a.action_position)
        || a.details.paths.is_empty()
        || a.details.paths.iter().any(String::is_empty)
        || a.details.instructions.is_empty()
    {
        return Err(invalid(
            "Invalid attempted action position or reporting details",
        ));
    }
    if a.kind != OperationKind::Apply
        && (record.actions().get(a.action_position) != Some(&a.action)
            || a.patches != read_patch(c, a.record_id, a.action_position)?)
    {
        return Err(invalid("Attempt differs from recorded action or patches"));
    }
    Ok(())
}
fn run_info(c: &Connection, id: RunId) -> Result<(usize, OperationKind)> {
    let (rid, kind): (i64, String) = c.query_row(
        "SELECT record_id,kind FROM runs WHERE id=?1",
        [id.0],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((index(rid)?, OperationKind::parse(&kind)?))
}
fn confirm(c: &Connection, a: &CurrentAttempt) -> Result<()> {
    let rid = number(a.record_id)?;
    match a.kind {
        OperationKind::Apply => {
            c.execute("INSERT INTO actions VALUES(?1,?2,?3)", params![
                rid,
                number(a.action_position)?,
                encode(&a.action)?
            ])?;
            if let Some(pair) = &a.patches {
                insert_patch(c, a.record_id, a.action_position, pair)?;
            }
            // Preserve an applied prefix of a partially undone record while invalidating its redo suffix.
            c.execute("UPDATE records SET redo_allowed=0,state=CASE WHEN applied_count=0 THEN 'superseded' ELSE state END WHERE applied_count < (SELECT count(*) FROM actions WHERE record_id=records.id) AND id<>?1",[rid])?;
            c.execute(
                "UPDATE records SET applied_count=applied_count+1 WHERE id=?1",
                [rid],
            )?;
        },
        OperationKind::Undo => {
            c.execute("UPDATE records SET applied_count=applied_count-1,state=CASE WHEN applied_count=1 THEN 'undone' ELSE state END WHERE id=?1",[rid])?;
        },
        OperationKind::Redo => {
            c.execute("UPDATE records SET applied_count=applied_count+1,state='redone' WHERE id=?1",[rid])?;
        },
    }
    c.execute("DELETE FROM attempts WHERE id=?1", [a.id.0])?;
    Ok(())
}
fn insert_attempt_patch(
    c: &Connection,
    id: AttemptId,
    p: &BinaryPatchPair,
) -> Result<()> {
    c.execute(
        "INSERT INTO attempt_patches VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            id.0,
            p.format,
            i64::try_from(p.before.length)
                .map_err(|e| invalid(&e.to_string()))?,
            i64::try_from(p.after.length)
                .map_err(|e| invalid(&e.to_string()))?,
            p.before.sha256.as_slice(),
            p.after.sha256.as_slice(),
            p.forward,
            p.reverse
        ],
    )?;
    Ok(())
}
fn read_attempt_patch(
    c: &Connection,
    id: AttemptId,
) -> Result<Option<BinaryPatchPair>> {
    // Reuse patch decoding through a parameterized query with the same column order.
    read_patch_query(
        c,
        "SELECT format,before_length,after_length,before_hash,after_hash,forward,reverse FROM attempt_patches WHERE attempt_id=?1",
        &[&id.0],
    )
}
fn insert_patch(
    c: &Connection,
    record: usize,
    position: usize,
    p: &BinaryPatchPair,
) -> Result<()> {
    validate_patch(p)?;
    c.execute(
        "INSERT INTO patches VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            number(record)?,
            number(position)?,
            p.format,
            i64::try_from(p.before.length)
                .map_err(|e| HistoryError::SaveError(e.to_string()))?,
            i64::try_from(p.after.length)
                .map_err(|e| HistoryError::SaveError(e.to_string()))?,
            p.before.sha256.as_slice(),
            p.after.sha256.as_slice(),
            p.forward,
            p.reverse
        ],
    )?;
    Ok(())
}
fn validate_patch(p: &BinaryPatchPair) -> Result<()> {
    if p.format != "bsdiff40-v1"
        || !p.forward.starts_with(b"BSDIFF40")
        || !p.reverse.starts_with(b"BSDIFF40")
        || p.forward.len() < 32
        || p.reverse.len() < 32
    {
        return Err(HistoryError::LoadError(
            "Invalid patch format or header".into(),
        ));
    }
    Ok(())
}
fn read_patch(
    c: &Connection,
    record: usize,
    position: usize,
) -> Result<Option<BinaryPatchPair>> {
    read_patch_query(
        c,
        "SELECT format,before_length,after_length,before_hash,after_hash,forward,reverse FROM patches WHERE record_id=?1 AND action_position=?2",
        &[&number(record)?, &number(position)?],
    )
}
fn read_patch_query(
    c: &Connection,
    sql: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Option<BinaryPatchPair>> {
    let raw = c
        .query_row(sql, values, |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Vec<u8>>(3)?,
                r.get::<_, Vec<u8>>(4)?,
                r.get::<_, Vec<u8>>(5)?,
                r.get::<_, Vec<u8>>(6)?,
            ))
        })
        .optional()?;
    raw.map(|(format, before, after, bh, ah, forward, reverse)| {
        let pair = BinaryPatchPair {
            format,
            before: ByteIdentity {
                length: u64::try_from(before)
                    .map_err(|e| HistoryError::LoadError(e.to_string()))?,
                sha256: bh.try_into().map_err(|_| {
                    HistoryError::LoadError("Invalid hash length".into())
                })?,
            },
            after: ByteIdentity {
                length: u64::try_from(after)
                    .map_err(|e| HistoryError::LoadError(e.to_string()))?,
                sha256: ah.try_into().map_err(|_| {
                    HistoryError::LoadError("Invalid hash length".into())
                })?,
            },
            forward,
            reverse,
        };
        validate_patch(&pair)?;
        Ok(pair)
    })
    .transpose()
}

pub(super) fn validate_patches(
    c: &Connection,
    records: &[database::DatabaseRecord],
) -> Result<()> {
    for stored in records {
        for (position, action) in stored.record.iter().enumerate() {
            let pair = read_patch(c, stored.record.id().unwrap(), position)?;
            if matches!(action, StoredAction::EditTagValues { .. })
                != pair.is_some()
            {
                return Err(invalid("Action patch data differs from action"));
            }
        }
    }
    Ok(())
}
