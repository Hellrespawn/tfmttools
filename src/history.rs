use crate::action::Action;
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub(crate) struct History {
    records: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Record {
    actions: Vec<Action>,
    applied: bool,

    #[cfg(feature = "time")]
    timestamp: Option<OffsetDateTime>,

    #[cfg(not(feature = "time"))]
    timestamp: Option<String>,
}

impl Record {
    #[cfg(feature = "time")]
    pub fn new(actions: Vec<Action>) -> Result<Self> {
        Ok(Self {
            actions,
            applied: true,
            timestamp: Some(OffsetDateTime::now_local()?),
        })
    }

    #[cfg(not(feature = "time"))]
    pub fn new(actions: Vec<Action>) -> Result<Self> {
        Ok(Self { actions, applied: true, timestamp: None })
    }
}
