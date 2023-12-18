use color_eyre::eyre::eyre;
use color_eyre::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::trace;

use super::record::Record;
use super::stack::RefStack;

pub struct HistorySerde;

impl HistorySerde {
    pub fn serialize<T>(stack: &RefStack<Record<T>>) -> Result<Vec<u8>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        Ok(bincode::serialize(stack)?)
    }

    pub fn deserialize<T>(bytes: &[u8]) -> Result<RefStack<Record<T>>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        let stack = bincode::deserialize(bytes)
            .map_err(|err| eyre!("Unable to deserialize history: {}", err,))?;

        trace!("Deserialized history:\n{:#?}", stack);

        Ok(stack)
    }
}
