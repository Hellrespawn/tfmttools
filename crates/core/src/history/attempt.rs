use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

use super::{
    ActionRecordMetadata, BinaryPatchPair, History, HistoryError, HistoryMode,
    Record, Result, StoredAction, database, patch,
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
impl From<HistoryMode> for OperationKind {
    fn from(direction: HistoryMode) -> Self {
        match direction {
            HistoryMode::Undo => Self::Undo,
            HistoryMode::Redo => Self::Redo,
        }
    }
}

#[derive(Debug)]
pub enum RunRequest {
    Apply { metadata: ActionRecordMetadata },
    Replay { record_id: usize, direction: HistoryMode },
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
    serde_json::to_string(v).map_err(|e| HistoryError::SaveError(e.to_string()))
}
fn decode<T: serde::de::DeserializeOwned>(v: &str) -> Result<T> {
    serde_json::from_str(v).map_err(|e| HistoryError::LoadError(e.to_string()))
}

impl History {
    pub fn begin_run(&mut self, request: RunRequest) -> Result<RunId> {
        let kind = match &request {
            RunRequest::Apply { .. } => OperationKind::Apply,
            RunRequest::Replay { direction, .. } => (*direction).into(),
        };
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
        let record_id = match request {
            RunRequest::Apply { metadata } => {
                let id: i64 = tx.query_row(
                    "SELECT coalesce(max(id)+1,0) FROM records",
                    [],
                    |r| r.get(0),
                )?;
                tx.execute("INSERT INTO records(id,state,timestamp,metadata,complete,applied_count,redo_allowed) VALUES(?1,'applied',?2,?3,0,0,1)", params![id,chrono::Local::now().fixed_offset().to_rfc3339(),encode(&metadata)?])?;
                index(id)?
            },
            RunRequest::Replay { record_id: id, .. } => {
                let record = database::read_record(&tx, id)?;
                let eligible = record.next_action_position(kind).is_some();
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
        let changed = if kind == OperationKind::Apply {
            vec![database::read_record(&tx, record_id)?]
        } else {
            Vec::new()
        };
        commit(tx, &mut self.cache_usable)?;
        self.publish_records(changed);
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
        let tx = self.connection.as_mut().unwrap().transaction()?;
        let (rid, kind) = run_info(&tx, run)?;
        let prospective = CurrentAttempt {
            id: AttemptId(0),
            run_id: run,
            record_id: rid,
            kind,
            action_position,
            action: action.clone(),
            details: details.clone(),
            patches: patches.cloned(),
        };
        validate_attempt(&tx, &prospective)?;
        tx.execute("INSERT INTO attempts(run_id,action_position,payload,details) VALUES(?1,?2,?3,?4)", params![run.0,number(action_position)?,encode(action)?,encode(details)?])?;
        let id = AttemptId(tx.last_insert_rowid());
        if let Some(pair) = patches {
            patch::insert_attempt(&tx, id, pair)?;
        }
        commit(tx, &mut self.cache_usable)?;
        Ok(id)
    }

    pub fn current_attempt(&self) -> Result<Option<CurrentAttempt>> {
        self.ensure_usable()?;
        self.connection.as_ref().map_or(Ok(None), read_current_attempt)
    }

    pub fn confirm_attempt(&mut self, id: AttemptId) -> Result<()> {
        self.ensure_connection()?;
        let a = self
            .current_attempt()?
            .filter(|a| a.id == id)
            .ok_or_else(|| invalid("Unknown attempt"))?;
        let tx = self.connection.as_mut().unwrap().transaction()?;
        let affected = confirm(&tx, &a)?;
        let changed = read_affected(&tx, affected)?;
        commit(tx, &mut self.cache_usable)?;
        self.publish_records(changed);
        Ok(())
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
        let record = database::read_record(&tx, rid)?;
        commit(tx, &mut self.cache_usable)?;
        self.publish_records(vec![record.clone()]);
        Ok(record)
    }

    /// Stored run identity for read-only interruption reporting.
    pub fn open_run(&self) -> Result<Option<(RunId, usize, OperationKind)>> {
        self.ensure_usable()?;
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
        let affected = match outcome {
            AttemptOutcome::Applied => confirm(&tx, &a)?,
            AttemptOutcome::NotApplied => {
                tx.execute("DELETE FROM attempts WHERE id=?1", [id.0])?;
                vec![a.record_id]
            },
        };
        tx.execute("UPDATE records SET redo_allowed=0,complete=CASE WHEN ?2='apply' THEN 0 ELSE complete END WHERE id=?1",params![number(a.record_id)?,a.kind.name()])?;
        tx.execute("DELETE FROM runs WHERE id=?1", [a.run_id.0])?;
        let changed = read_affected(&tx, affected)?;
        let record = changed
            .iter()
            .find(|r| r.id() == Some(a.record_id))
            .expect("Affected records include attempt record")
            .clone();
        commit(tx, &mut self.cache_usable)?;
        self.publish_records(changed);
        Ok(record)
    }

    pub fn patches(
        &self,
        record_id: usize,
        position: usize,
    ) -> Result<Option<BinaryPatchPair>> {
        self.ensure_usable()?;
        self.connection
            .as_ref()
            .map_or(Ok(None), |c| patch::read(c, record_id, position))
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
    let details: AttemptDetails = decode(&r.get::<_, String>(6)?)?;
    let patches = patch::read_attempt(c, id)?;
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
    validate_attempt(c, &attempt).map_err(|error| {
        match error {
            HistoryError::MutError(message) => {
                HistoryError::LoadError(format!("Attempt {}: {message}", id.0))
            },
            other => other,
        }
    })?;
    Ok(Some(attempt))
}

impl AttemptDetails {
    fn validate(&self) -> Result<()> {
        if self.paths.is_empty()
            || self.paths.iter().any(String::is_empty)
            || self.instructions.is_empty()
        {
            return Err(invalid(
                "Attempt requires paths and inspection instructions",
            ));
        }
        Ok(())
    }
}

fn validate_attempt(c: &Connection, a: &CurrentAttempt) -> Result<()> {
    database::validate_action(&a.action).map_err(|error| {
        match error {
            HistoryError::LoadError(message) => HistoryError::MutError(message),
            other => other,
        }
    })?;
    a.details.validate()?;
    let record = database::read_record(c, a.record_id)?;
    if record.next_action_position(a.kind) != Some(a.action_position) {
        return Err(invalid("Attempt position differs from replay cursor"));
    }
    patch::validate_action_pair(&a.action, a.patches.as_ref())?;
    if a.kind != OperationKind::Apply
        && (record.actions().get(a.action_position) != Some(&a.action)
            || a.patches != patch::read(c, a.record_id, a.action_position)?)
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
fn confirm(c: &Connection, a: &CurrentAttempt) -> Result<Vec<usize>> {
    let rid = number(a.record_id)?;
    let mut affected = vec![a.record_id];
    match a.kind {
        OperationKind::Apply => {
            c.execute("INSERT INTO actions(record_id,position,payload) VALUES(?1,?2,?3)", params![
                rid,
                number(a.action_position)?,
                encode(&a.action)?
            ])?;
            if let Some(pair) = &a.patches {
                patch::insert(c, a.record_id, a.action_position, pair)?;
            }
            // Preserve an applied prefix of a partially undone record while invalidating its redo suffix.
            if a.action_position == 0 {
                const PREDICATE: &str = "applied_count < (SELECT count(*) FROM actions WHERE record_id=records.id) AND id<>?1 AND (redo_allowed<>0 OR (applied_count=0 AND state<>'superseded'))";
                let mut statement = c.prepare(&format!(
                    "SELECT id FROM records WHERE {PREDICATE}"
                ))?;
                for id in statement.query_map([rid], |r| r.get::<_, i64>(0))? {
                    affected.push(index(id?)?);
                }
                c.execute(&format!("UPDATE records SET redo_allowed=0,state=CASE WHEN applied_count=0 THEN 'superseded' ELSE state END WHERE {PREDICATE}"), [rid])?;
            }
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
    Ok(affected)
}

fn read_affected(c: &Connection, ids: Vec<usize>) -> Result<Vec<Record>> {
    ids.into_iter().map(|id| database::read_record(c, id)).collect()
}

fn commit(tx: Transaction<'_>, cache_usable: &mut bool) -> Result<()> {
    // A commit error can be ambiguous. Never publish the staged cache in that case.
    *cache_usable = false;
    tx.commit()?;
    #[cfg(test)]
    if tests::FAIL_AFTER_COMMIT.replace(false) {
        return Err(HistoryError::SaveError(
            "Injected error after successful commit".into(),
        ));
    }
    *cache_usable = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        pub(super) static FAIL_AFTER_COMMIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    fn setup() -> (tempfile::TempDir, History) {
        let dir = tempfile::tempdir().unwrap();
        let history = History::new(
            camino::Utf8PathBuf::try_from(dir.path().join("history")).unwrap(),
        );
        (dir, history)
    }
    fn metadata() -> ActionRecordMetadata {
        ActionRecordMetadata::new(
            super::super::TemplateMetadata::Validation { value: "test".into() },
            vec![],
            "test".into(),
        )
    }
    fn attempt(h: &mut History, run: RunId, position: usize) -> AttemptId {
        h.begin_attempt(
            run,
            position,
            &StoredAction::MakeDir { path: "dir".into() },
            &AttemptDetails {
                paths: vec!["dir".into()],
                instructions: "Inspect".into(),
            },
            None,
        )
        .unwrap()
    }
    #[test]
    fn confirmation_decodes_only_affected_records() {
        let (_dir, mut h) = setup();
        for _ in 0..8 {
            let run = h
                .begin_run(RunRequest::Apply { metadata: metadata() })
                .unwrap();
            let id = attempt(&mut h, run, 0);
            h.confirm_attempt(id).unwrap();
            h.close_run(run, true).unwrap();
        }
        let undo = h
            .begin_run(RunRequest::Replay {
                record_id: 0,
                direction: HistoryMode::Undo,
            })
            .unwrap();
        let id = attempt(&mut h, undo, 0);
        h.confirm_attempt(id).unwrap();
        h.close_run(undo, true).unwrap();
        let run =
            h.begin_run(RunRequest::Apply { metadata: metadata() }).unwrap();
        for position in 0..3 {
            let id = attempt(&mut h, run, position);
            database::DECODED_IDS.with_borrow_mut(|ids| *ids = Some(vec![]));
            h.confirm_attempt(id).unwrap();
            let ids =
                database::DECODED_IDS.with_borrow_mut(Option::take).unwrap();
            assert!(
                ids.iter().all(|id| *id == 8 || (position == 0 && *id == 0)),
                "unrelated records decoded: {ids:?}"
            );
            assert!(ids.contains(&8));
            assert_eq!(ids.contains(&0), position == 0);
        }
    }
    #[test]
    fn failed_commit_poisoning_requires_a_validated_reload() {
        let (dir, mut h) = setup();
        let run =
            h.begin_run(RunRequest::Apply { metadata: metadata() }).unwrap();
        let id = attempt(&mut h, run, 0);
        let c = h.connection.as_ref().unwrap();
        c.execute_batch("CREATE TABLE deferred_failure(record_id INTEGER REFERENCES records(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_commit AFTER DELETE ON attempts BEGIN INSERT INTO deferred_failure VALUES(999); END;").unwrap();
        assert!(h.confirm_attempt(id).is_err());
        assert!(h.ensure_usable().is_err());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                h.records()
            }))
            .is_err()
        );
        assert!(
            h.begin_run(RunRequest::Apply { metadata: metadata() }).is_err()
        );
        // The injected trigger is outside the schema contract: reload must fail.
        assert!(h.load().is_err());
        assert!(h.ensure_usable().is_err());
        Connection::open(dir.path().join("history"))
            .unwrap()
            .execute_batch(
                "DROP TRIGGER fail_commit; DROP TABLE deferred_failure",
            )
            .unwrap();
        h.load().unwrap();
        assert!(h.ensure_usable().is_ok());
        assert!(h.records()[0].is_empty());
        assert_eq!(h.current_attempt().unwrap().unwrap().id, id);
        h.confirm_attempt(id).unwrap();
        assert_eq!(h.records()[0].applied_count(), 1);
    }
    #[test]
    fn ambiguous_successful_commit_publishes_only_after_reload() {
        let (_dir, mut h) = setup();
        let run =
            h.begin_run(RunRequest::Apply { metadata: metadata() }).unwrap();
        let id = attempt(&mut h, run, 0);
        FAIL_AFTER_COMMIT.set(true);
        assert!(h.confirm_attempt(id).is_err());
        assert!(h.ensure_usable().is_err());
        h.load().unwrap();
        assert!(h.current_attempt().unwrap().is_none());
        assert_eq!(h.records()[0].applied_count(), 1);
        h.close_run(run, false).unwrap();
        assert!(!h.records()[0].redo_allowed());
    }
}
