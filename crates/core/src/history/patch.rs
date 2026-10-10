use rusqlite::{Connection, OptionalExtension, ToSql, params_from_iter};
use serde::{Deserialize, Serialize};

use super::{AttemptId, HistoryError, Record, Result, StoredAction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteIdentity {
    pub length: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryPatchPair {
    pub format: String,
    pub before: ByteIdentity,
    pub after: ByteIdentity,
    pub forward: Vec<u8>,
    pub reverse: Vec<u8>,
}
fn number(value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|e| HistoryError::SaveError(e.to_string()))
}

fn contextualize<T>(result: Result<T>, context: &str) -> Result<T> {
    result.map_err(|error| {
        match error {
            HistoryError::LoadError(message) => {
                HistoryError::LoadError(format!("{context}: {message}"))
            },
            other => other,
        }
    })
}

pub(super) fn validate_action_pair(
    action: &StoredAction,
    pair: Option<&BinaryPatchPair>,
) -> Result<()> {
    if matches!(action, StoredAction::EditTagValues { .. }) != pair.is_some() {
        return Err(HistoryError::MutError(
            "Tag actions require binary patches only".into(),
        ));
    }
    if let Some(pair) = pair {
        validate_pair(pair)?;
    }
    Ok(())
}

struct EncodedPair<'a> {
    pair: &'a BinaryPatchPair,
    before: i64,
    after: i64,
}
impl<'a> EncodedPair<'a> {
    fn new(pair: &'a BinaryPatchPair) -> Result<Self> {
        validate_pair(pair)?;
        Ok(Self {
            pair,
            before: i64::try_from(pair.before.length)
                .map_err(|e| HistoryError::SaveError(e.to_string()))?,
            after: i64::try_from(pair.after.length)
                .map_err(|e| HistoryError::SaveError(e.to_string()))?,
        })
    }

    fn values(&self) -> [&dyn ToSql; 7] {
        [
            &self.pair.format,
            &self.before,
            &self.after,
            &self.pair.before.sha256,
            &self.pair.after.sha256,
            &self.pair.forward,
            &self.pair.reverse,
        ]
    }
}

pub(super) fn insert_attempt(
    c: &Connection,
    id: AttemptId,
    pair: &BinaryPatchPair,
) -> Result<()> {
    let encoded = EncodedPair::new(pair)?;
    c.execute("INSERT INTO attempt_patches(attempt_id,format,before_length,after_length,before_hash,after_hash,forward,reverse) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params_from_iter([&id.0 as &dyn ToSql].into_iter().chain(encoded.values())))?;
    Ok(())
}

pub(super) fn insert(
    c: &Connection,
    record: usize,
    position: usize,
    pair: &BinaryPatchPair,
) -> Result<()> {
    let encoded = EncodedPair::new(pair)?;
    let record = number(record)?;
    let position = number(position)?;
    c.execute("INSERT INTO patches(record_id,action_position,format,before_length,after_length,before_hash,after_hash,forward,reverse) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params_from_iter([&record as &dyn ToSql, &position].into_iter().chain(encoded.values())))?;
    Ok(())
}

pub(super) fn validate_pair(p: &BinaryPatchPair) -> Result<()> {
    if p.format != "bsdiff40-v1"
        || !p.forward.starts_with(b"BSDIFF40")
        || !p.reverse.starts_with(b"BSDIFF40")
        || p.forward.len() < 32
        || p.reverse.len() < 32
    {
        return Err(HistoryError::MutError(
            "Invalid patch format or header".into(),
        ));
    }
    Ok(())
}
pub(super) fn read_attempt(
    c: &Connection,
    id: AttemptId,
) -> Result<Option<BinaryPatchPair>> {
    // Reuse patch decoding through a parameterized query with the same column order.
    contextualize(
        read_patch_query(
            c,
            "SELECT format,before_length,after_length,before_hash,after_hash,forward,reverse FROM attempt_patches WHERE attempt_id=?1",
            &[&id.0],
        ),
        &format!("Attempt {}", id.0),
    )
}
pub(super) fn read(
    c: &Connection,
    record: usize,
    position: usize,
) -> Result<Option<BinaryPatchPair>> {
    contextualize(
        read_patch_query(
            c,
            "SELECT format,before_length,after_length,before_hash,after_hash,forward,reverse FROM patches WHERE record_id=?1 AND action_position=?2",
            &[&number(record)?, &number(position)?],
        ),
        &format!("Record {record}, action {position}"),
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
        validate_pair(&pair)
            .map_err(|error| HistoryError::LoadError(error.to_string()))?;
        Ok(pair)
    })
    .transpose()
}

pub(super) fn validate_records(
    c: &Connection,
    records: &[Record],
) -> Result<()> {
    for record in records {
        let id = record.id().expect("Stored record has an ID");
        for (position, action) in record.iter().enumerate() {
            let pair = read(c, id, position)?;
            contextualize(
                validate_action_pair(action, pair.as_ref())
                    .map_err(|e| HistoryError::LoadError(e.to_string())),
                &format!("Record {id}, action {position}"),
            )?;
        }
    }
    Ok(())
}
