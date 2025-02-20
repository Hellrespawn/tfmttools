use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::trace;

use super::record::Record;
use super::stack::RefStack;
use crate::{HistoryError, Result};

pub struct HistorySerde;

impl HistorySerde {
    pub fn serialize<T, M>(stack: &RefStack<Record<T, M>>) -> Result<Vec<u8>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
        M: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        let result = serde_json::to_vec_pretty(stack);

        let bytes =
            result.map_err(|source| HistoryError::Serialize { source })?;

        Ok(bytes)
    }

    pub fn deserialize<T, M>(bytes: &[u8]) -> Result<RefStack<Record<T, M>>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
        M: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        let stack = serde_json::from_slice(bytes)
            .map_err(|source| HistoryError::Deserialize { source })?;

        trace!("Deserialized history:\n{:#?}", stack);

        Ok(stack)
    }
}
