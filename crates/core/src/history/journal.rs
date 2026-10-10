use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::{
    ActionRecordMetadata, BinaryPatchPair, ByteIdentity, History, HistoryError,
    Record, Result, StoredAction, database,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationId(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    fn parse(value: &str) -> Result<Self> {
        match value {
            "apply" => Ok(Self::Apply),
            "undo" => Ok(Self::Undo),
            "redo" => Ok(Self::Redo),
            _ => Err(HistoryError::LoadError("Invalid operation kind".into())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields, rename_all = "snake_case")]
pub enum RecoveryDescriptor {
    FileSwitch {
        path: String,
        resolved: String,
        candidate: String,
        retained: String,
        before: ByteIdentity,
        after: ByteIdentity,
    },
    Move {
        source: String,
        target: String,
        identity: ByteIdentity,
    },
    Copy {
        source: String,
        target: String,
        identity: ByteIdentity,
        remove_source: bool,
        candidate: String,
    },
    Remove {
        path: String,
        identity: ByteIdentity,
    },
    Directory {
        path: String,
        before_exists: bool,
        after_exists: bool,
    },
    Noop,
}
#[derive(Debug, Clone)]
pub struct PreparedAction {
    pub action: StoredAction,
    pub recovery: RecoveryDescriptor,
    pub patches: Option<BinaryPatchPair>,
}
#[derive(Debug, Clone)]
pub struct PendingEntry {
    pub position: usize,
    pub action_position: usize,
    pub prepared: PreparedAction,
    pub completed: bool,
    pub cleaned: bool,
}
#[derive(Debug, Clone)]
pub struct PendingOperation {
    pub id: OperationId,
    pub kind: OperationKind,
    pub record_id: usize,
    pub plan: Option<Vec<StoredAction>>,
    pub entries: Vec<PendingEntry>,
    pub finalized: bool,
}

pub(super) fn encode<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|e| HistoryError::SaveError(e.to_string()))
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value)
        .map_err(|e| HistoryError::LoadError(e.to_string()))
}
fn number(value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|e| HistoryError::MutError(e.to_string()))
}
fn index(value: i64) -> Result<usize> {
    usize::try_from(value).map_err(|e| HistoryError::LoadError(e.to_string()))
}

// Mutation preconditions need counts and flags, not reconstructed recovery
// entries or patch blobs. Full snapshots are reserved for load and recovery.
struct OperationState {
    record_id: usize,
    kind: OperationKind,
    finalized: bool,
    planned: Option<usize>,
    prepared: usize,
    unfinished: bool,
}

impl History {
    pub fn cancel_unstarted(&mut self, id: OperationId) -> Result<()> {
        let operation = self.operation_state(id)?;
        if operation.finalized || operation.prepared != 0 {
            return Err(HistoryError::MutError(
                "Cannot discard an operation with recorded effects".into(),
            ));
        }
        let connection = self.connection.as_mut().unwrap();
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM operations WHERE id=?1", [id.0])?;
        if operation.kind == OperationKind::Apply {
            transaction.execute("DELETE FROM actions WHERE record_id=?1", [
                number(operation.record_id)?,
            ])?;
            transaction.execute(
                "DELETE FROM records WHERE id=?1 AND finalized=0",
                [number(operation.record_id)?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn begin_operation(
        &mut self,
        kind: OperationKind,
        record_id: Option<usize>,
        metadata: Option<ActionRecordMetadata>,
    ) -> Result<OperationId> {
        self.ensure_connection()?;
        let connection = self.connection.as_mut().unwrap();
        let transaction = connection.transaction()?;
        if transaction.query_row(
            "SELECT count(*) FROM operations",
            [],
            |r| r.get::<_, i64>(0),
        )? != 0
        {
            return Err(HistoryError::MutError(
                "History has pending recovery work".into(),
            ));
        }
        let id = match kind {
            OperationKind::Apply => {
                if record_id.is_some() {
                    return Err(HistoryError::MutError(
                        "New run must not select a record".into(),
                    ));
                }
                let metadata = metadata.ok_or_else(|| {
                    HistoryError::MutError("New run requires metadata".into())
                })?;
                let id: i64 = transaction.query_row(
                    "SELECT coalesce(max(id)+1,0) FROM records",
                    [],
                    |r| r.get(0),
                )?;
                let position: i64 = transaction.query_row(
                    "SELECT coalesce(max(position)+1,0) FROM records",
                    [],
                    |r| r.get(0),
                )?;
                transaction.execute(
                    "INSERT INTO records VALUES(?1,?2,'applied',?3,?4,0)",
                    params![
                        id,
                        position,
                        chrono::Local::now().fixed_offset().to_rfc3339(),
                        encode(&metadata)?
                    ],
                )?;
                id
            },
            OperationKind::Undo | OperationKind::Redo => {
                if metadata.is_some() {
                    return Err(HistoryError::MutError(
                        "Replay must retain recorded metadata".into(),
                    ));
                }
                let id = number(record_id.ok_or_else(|| {
                    HistoryError::MutError("Replay requires a record".into())
                })?)?;
                let state: Option<String> = transaction
                    .query_row(
                        "SELECT state FROM records WHERE id=?1 AND finalized=1",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let allowed = match kind {
                    OperationKind::Undo => {
                        matches!(state.as_deref(), Some("applied" | "redone"))
                    },
                    OperationKind::Redo => state.as_deref() == Some("undone"),
                    OperationKind::Apply => false,
                };
                if !allowed {
                    return Err(HistoryError::MutError(
                        "Record state does not permit requested replay".into(),
                    ));
                }
                id
            },
        };
        transaction.execute("INSERT INTO operations(record_id,kind,finalized,plan) VALUES(?1,?2,0,'null')",params![id,kind.name()])?;
        let operation = OperationId(transaction.last_insert_rowid());
        transaction.commit()?;
        Ok(operation)
    }

    pub fn set_operation_plan(
        &mut self,
        id: OperationId,
        plan: &[StoredAction],
    ) -> Result<()> {
        for action in plan {
            database::validate_action(action)?;
        }
        let operation = self.operation_state(id)?;
        if operation.planned.is_some() {
            return Err(HistoryError::MutError(
                "Operation plan is already saved".into(),
            ));
        }
        let connection = self.connection.as_mut().unwrap();
        let transaction = connection.transaction()?;
        if operation.kind == OperationKind::Apply {
            for (position, action) in plan.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO actions VALUES(?1,?2,?3)",
                    params![
                        number(operation.record_id)?,
                        number(position)?,
                        encode(action)?
                    ],
                )?;
            }
        } else {
            let record =
                database::read_record(&transaction, operation.record_id)?
                    .record;
            let expected: Vec<_> = if operation.kind == OperationKind::Undo {
                record.iter().rev().cloned().collect()
            } else {
                record.iter().cloned().collect()
            };
            if plan != expected {
                return Err(HistoryError::MutError(
                    "Replay plan differs from recorded actions".into(),
                ));
            }
        }
        transaction.execute(
            "UPDATE operations SET plan=?1 WHERE id=?2",
            params![encode(&plan)?, id.0],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn append_prepared(
        &mut self,
        id: OperationId,
        entry: &PreparedAction,
    ) -> Result<usize> {
        let operation = self.operation_state(id)?;
        let planned = operation.planned.ok_or_else(|| {
            HistoryError::MutError(
                "Save the operation plan before preparing effects".into(),
            )
        })?;
        let position = operation.prepared;
        if operation.finalized || operation.unfinished {
            return Err(HistoryError::MutError(
                "Previous action still requires recovery".into(),
            ));
        }
        if position >= planned {
            return Err(HistoryError::MutError(
                "Progress outside operation plan".into(),
            ));
        }
        let action_position = if operation.kind == OperationKind::Undo {
            planned - 1 - position
        } else {
            position
        };
        let expected = database::read_action(
            self.connection()?,
            operation.record_id,
            action_position,
        )?;
        if entry.action != expected {
            return Err(HistoryError::MutError(
                "Prepared action differs from saved plan".into(),
            ));
        }
        validate_prepared(entry, operation.kind)?;
        let connection = self.connection.as_mut().unwrap();
        let transaction = connection.transaction()?;
        if operation.kind == OperationKind::Apply {
            if let Some(pair) = &entry.patches {
                insert_patch(
                    &transaction,
                    operation.record_id,
                    action_position,
                    pair,
                )?;
            }
        } else if let Some(pair) = &entry.patches {
            let saved =
                read_patch(&transaction, operation.record_id, action_position)?
                    .ok_or_else(|| {
                        HistoryError::MutError(
                            "Recorded patches missing".into(),
                        )
                    })?;
            if pair != &saved {
                return Err(HistoryError::MutError(
                    "Replay patches differ from recorded patches".into(),
                ));
            }
        }
        transaction.execute(
            "INSERT INTO progress VALUES(?1,?2,?3,?4,0,0)",
            params![
                id.0,
                number(position)?,
                number(action_position)?,
                encode(&entry.recovery)?
            ],
        )?;
        transaction.commit()?;
        Ok(position)
    }

    pub fn complete_action(
        &mut self,
        id: OperationId,
        position: usize,
    ) -> Result<()> {
        let changed = self.connection()?.execute(
            "UPDATE progress SET completed=1
             WHERE operation_id=?1 AND position=?2",
            params![id.0, number(position)?],
        )?;
        if changed != 1 {
            return Err(HistoryError::MutError(
                "Unknown action position".into(),
            ));
        }
        Ok(())
    }

    pub fn finish_operation(&mut self, id: OperationId) -> Result<Record> {
        let operation = self.operation_state(id)?;
        let planned = operation.planned.ok_or_else(|| {
            HistoryError::MutError("Operation plan is missing".into())
        })?;
        if operation.prepared != planned || operation.unfinished {
            return Err(HistoryError::MutError(
                "Operation has unfinished actions".into(),
            ));
        }
        let connection = self.connection.as_mut().unwrap();
        let transaction = connection.transaction()?;
        if !operation.finalized {
            match operation.kind {
                OperationKind::Apply => {
                    transaction.execute("UPDATE records SET state='superseded' WHERE state='undone' AND finalized=1",[])?;
                    transaction.execute(
                        "UPDATE records SET finalized=1 WHERE id=?1",
                        [number(operation.record_id)?],
                    )?;
                },
                OperationKind::Undo => {
                    transaction.execute(
                        "UPDATE records SET state='undone' WHERE id=?1",
                        [number(operation.record_id)?],
                    )?;
                },
                OperationKind::Redo => {
                    transaction.execute(
                        "UPDATE records SET state='redone' WHERE id=?1",
                        [number(operation.record_id)?],
                    )?;
                },
            }
            transaction
                .execute("UPDATE operations SET finalized=1 WHERE id=?1", [
                    id.0,
                ])?;
        }
        let records: Vec<_> = database::read_records(&transaction, true)?
            .into_iter()
            .map(|r| r.record)
            .collect();
        let record = records
            .iter()
            .find(|r| r.id() == Some(operation.record_id))
            .cloned()
            .ok_or_else(|| {
                HistoryError::MutError("Operation record missing".into())
            })?;
        if operation.prepared == 0 {
            transaction
                .execute("DELETE FROM operations WHERE id=?1", [id.0])?;
        }
        transaction.commit()?;
        self.records = records;
        Ok(record)
    }

    pub fn complete_cleanup(
        &mut self,
        id: OperationId,
        position: usize,
    ) -> Result<()> {
        let connection = self.connection.as_mut().ok_or_else(|| {
            HistoryError::MutError("Unknown pending operation".into())
        })?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE progress SET cleaned=1
             WHERE operation_id=?1 AND position=?2 AND completed=1
               AND EXISTS(SELECT 1 FROM operations WHERE id=?1 AND finalized=1)",
            params![id.0, number(position)?],
        )?;
        if changed != 1 {
            return Err(HistoryError::MutError(
                "Cleanup requires durable finalization".into(),
            ));
        }
        let remaining: i64 = transaction.query_row(
            "SELECT count(*) FROM progress WHERE operation_id=?1 AND cleaned=0",
            [id.0],
            |r| r.get(0),
        )?;
        if remaining == 0 {
            transaction
                .execute("DELETE FROM operations WHERE id=?1", [id.0])?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn pending_operations(&self) -> Result<Vec<PendingOperation>> {
        match &self.connection {
            Some(c) => {
                let transaction = c.unchecked_transaction()?;
                let pending = read_pending(&transaction, None)?;
                transaction.commit()?;
                Ok(pending)
            },
            None => Ok(vec![]),
        }
    }

    fn connection(&self) -> Result<&Connection> {
        self.connection.as_ref().ok_or_else(|| {
            HistoryError::MutError("Unknown pending operation".into())
        })
    }

    fn operation_state(&self, id: OperationId) -> Result<OperationState> {
        let (record_id, kind, finalized, planned, prepared, unfinished) = self
            .connection()?
            .query_row(
                "SELECT record_id, kind, finalized,
                        CASE WHEN json_type(plan)='array'
                             THEN json_array_length(plan) END,
                        (SELECT count(*) FROM progress WHERE operation_id=?1),
                        EXISTS(SELECT 1 FROM progress
                               WHERE operation_id=?1 AND completed=0)
                 FROM operations WHERE id=?1",
                [id.0],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, bool>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| {
                HistoryError::MutError("Unknown pending operation".into())
            })?;
        Ok(OperationState {
            record_id: index(record_id)?,
            kind: OperationKind::parse(&kind)?,
            finalized,
            planned: planned.map(index).transpose()?,
            prepared: index(prepared)?,
            unfinished,
        })
    }

    pub fn patches(
        &self,
        record_id: usize,
        position: usize,
    ) -> Result<Option<BinaryPatchPair>> {
        match &self.connection {
            Some(c) => read_patch(c, record_id, position),
            None => Ok(None),
        }
    }
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
    let raw=c.query_row("SELECT format,before_length,after_length,before_hash,after_hash,forward,reverse FROM patches WHERE record_id=?1 AND action_position=?2",params![number(record)?,number(position)?],|r| Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,Vec<u8>>(3)?,r.get::<_,Vec<u8>>(4)?,r.get::<_,Vec<u8>>(5)?,r.get::<_,Vec<u8>>(6)?))).optional()?;
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

fn validate_copy_direction(
    recovery: &RecoveryDescriptor,
    kind: OperationKind,
) -> Result<()> {
    if let RecoveryDescriptor::Copy { remove_source, .. } = recovery
        && *remove_source != (kind == OperationKind::Undo)
    {
        return Err(HistoryError::LoadError(
            "Recovery copy removal differs from action direction".into(),
        ));
    }
    Ok(())
}

fn validate_directory_direction(
    action: &StoredAction,
    kind: OperationKind,
    before_exists: bool,
    after_exists: bool,
) -> Result<()> {
    let creates = matches!(action, StoredAction::MakeDir { .. })
        != (kind == OperationKind::Undo);
    // Removing a nonempty directory is an intentional no-op. Creation
    // always leaves a directory; removal cannot create one.
    if (creates && !after_exists)
        || (!creates && !before_exists && after_exists)
    {
        return Err(HistoryError::LoadError(
            "Recovery directory transition differs from action direction"
                .into(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Keep exhaustive action/descriptor matching together.
fn validate_prepared(
    entry: &PreparedAction,
    kind: OperationKind,
) -> Result<()> {
    database::validate_action(&entry.action)?;
    validate_copy_direction(&entry.recovery, kind)?;
    match (&entry.action, &entry.recovery, &entry.patches) {
        (
            StoredAction::EditTagValues { path, .. },
            RecoveryDescriptor::FileSwitch {
                path: recovery_path,
                resolved,
                candidate,
                retained,
                before,
                after,
            },
            Some(pair),
        ) => {
            validate_patch(pair)?;
            let (input, output) = if kind == OperationKind::Undo {
                (&pair.after, &pair.before)
            } else {
                (&pair.before, &pair.after)
            };
            if path != recovery_path
                || resolved.is_empty()
                || candidate.is_empty()
                || retained.is_empty()
                || resolved == candidate
                || resolved == retained
                || candidate == retained
                || before != input
                || after != output
            {
                return Err(HistoryError::LoadError(
                    "Invalid tag recovery descriptor".into(),
                ));
            }
        },
        (StoredAction::EditTagValues { .. }, _, _) => {
            return Err(HistoryError::LoadError(
                "Tag edit requires patches and switch descriptor".into(),
            ));
        },
        (_, _, Some(_)) | (_, RecoveryDescriptor::FileSwitch { .. }, None) => {
            return Err(HistoryError::LoadError(
                "Unexpected patches on filesystem action".into(),
            ));
        },
        (
            StoredAction::MoveFile { source, target },
            RecoveryDescriptor::Move {
                source: actual_source,
                target: actual_target,
                ..
            },
            None,
        )
        | (
            StoredAction::CopyFile { source, target },
            RecoveryDescriptor::Copy {
                source: actual_source,
                target: actual_target,
                ..
            },
            None,
        ) => {
            let (source, target) = if kind == OperationKind::Undo {
                (target, source)
            } else {
                (source, target)
            };
            if source != actual_source || target != actual_target {
                return Err(HistoryError::LoadError(
                    "Recovery move/copy paths differ from action".into(),
                ));
            }
        },
        (
            StoredAction::RemoveFile { path },
            RecoveryDescriptor::Remove { path: actual_path, .. },
            None,
        ) if kind != OperationKind::Undo && path == actual_path => {},
        (StoredAction::RemoveFile { .. }, RecoveryDescriptor::Noop, None)
            if kind == OperationKind::Undo => {},
        (
            StoredAction::MakeDir { path } | StoredAction::RemoveDir { path },
            RecoveryDescriptor::Directory {
                path: actual_path,
                before_exists,
                after_exists,
            },
            None,
        ) if path == actual_path => {
            validate_directory_direction(
                &entry.action,
                kind,
                *before_exists,
                *after_exists,
            )?;
        },
        _ => {
            return Err(HistoryError::LoadError(
                "Recovery descriptor differs from action".into(),
            ));
        },
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Validate the complete journal snapshot together.
fn read_pending(
    c: &Connection,
    records: Option<&[database::DatabaseRecord]>,
) -> Result<Vec<PendingOperation>> {
    let mut s = c.prepare(
        "SELECT id,record_id,kind,finalized,plan FROM operations ORDER BY id",
    )?;
    let mut rows = s.query([])?;
    let mut result = vec![];
    while let Some(r) = rows.next()? {
        let id = OperationId(r.get(0)?);
        let record_id = index(r.get(1)?)?;
        let kind = OperationKind::parse(&r.get::<_, String>(2)?)?;
        let finalized = r.get::<_, i64>(3)? == 1;
        let plan: Option<Vec<StoredAction>> = decode(&r.get::<_, String>(4)?)?;
        let loaded;
        let record = if let Some(records) = records {
            &records
                .iter()
                .find(|r| r.record.id() == Some(record_id))
                .ok_or_else(|| {
                    HistoryError::LoadError("Pending record missing".into())
                })?
                .record
        } else {
            loaded = database::read_record(c, record_id)?;
            &loaded.record
        };
        if let Some(plan) = &plan {
            for action in plan {
                database::validate_action(action)?;
            }
            let expected: Vec<_> = if kind == OperationKind::Undo {
                record.iter().rev().cloned().collect()
            } else {
                record.iter().cloned().collect()
            };
            if plan != &expected {
                return Err(HistoryError::LoadError(
                    "Pending plan differs from recorded actions".into(),
                ));
            }
        }
        let mut progress=c.prepare("SELECT position,action_position,recovery,completed,cleaned FROM progress WHERE operation_id=?1 ORDER BY position")?;
        let mut progress_rows = progress.query([id.0])?;
        let mut entries = vec![];
        while let Some(p) = progress_rows.next()? {
            let position = index(p.get(0)?)?;
            let action_position = index(p.get(1)?)?;
            let plan = plan.as_ref().ok_or_else(|| {
                HistoryError::LoadError(
                    "Progress without an operation plan".into(),
                )
            })?;
            let expected_position = if kind == OperationKind::Undo {
                plan.len().checked_sub(position + 1)
            } else {
                Some(position)
            };
            if position != entries.len()
                || Some(action_position) != expected_position
            {
                return Err(HistoryError::LoadError(
                    "Invalid operation progress ordering".into(),
                ));
            }
            let action = plan.get(position).cloned().ok_or_else(|| {
                HistoryError::LoadError(
                    "Progress outside operation plan".into(),
                )
            })?;
            let prepared = PreparedAction {
                action,
                recovery: decode(&p.get::<_, String>(2)?)?,
                patches: read_patch(c, record_id, action_position)?,
            };
            validate_prepared(&prepared, kind)?;
            let completed = p.get::<_, i64>(3)? == 1;
            let cleaned = p.get::<_, i64>(4)? == 1;
            if cleaned && (!completed || !finalized) {
                return Err(HistoryError::LoadError(
                    "Cleanup without finalization".into(),
                ));
            }
            entries.push(PendingEntry {
                position,
                action_position,
                prepared,
                completed,
                cleaned,
            });
        }
        if entries
            .iter()
            .take(entries.len().saturating_sub(1))
            .any(|e| !e.completed)
            || (finalized
                && (plan.as_ref().is_none_or(|p| p.len() != entries.len())
                    || entries.iter().any(|e| !e.completed)))
        {
            return Err(HistoryError::LoadError(
                "Invalid finalized progress".into(),
            ));
        }
        result.push(PendingOperation {
            id,
            kind,
            record_id,
            plan,
            entries,
            finalized,
        });
    }
    Ok(result)
}

pub(super) fn validate_patches(
    c: &Connection,
    records: &[database::DatabaseRecord],
) -> Result<()> {
    for stored in records {
        let record = &stored.record;
        for (position, action) in record.iter().enumerate() {
            let pair = read_patch(c, record.id().unwrap(), position)?;
            if matches!(action, StoredAction::EditTagValues { .. }) {
                if stored.finalized && pair.is_none() {
                    return Err(HistoryError::LoadError(
                        "Finalized tag action has no binary patches".into(),
                    ));
                }
            } else if pair.is_some() {
                return Err(HistoryError::LoadError(
                    "Unexpected patch on filesystem action".into(),
                ));
            }
        }
    }
    read_pending(c, Some(records))?;
    Ok(())
}
