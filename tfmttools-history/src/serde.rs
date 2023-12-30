use color_eyre::eyre::eyre;
use color_eyre::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::trace;

use super::record::Record;
use super::stack::RefStack;

pub struct HistorySerde;

impl HistorySerde {
    pub fn serialize<T, M>(stack: &RefStack<Record<T, M>>) -> Result<Vec<u8>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
        M: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        #[cfg(feature = "debug")]
        let bytes = serde_json::to_vec_pretty(stack)?;

        #[cfg(not(feature = "debug"))]
        let bytes = serde_json::to_vec(stack)?;

        Ok(bytes)
    }

    pub fn deserialize<T, M>(bytes: &[u8]) -> Result<RefStack<Record<T, M>>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
        M: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        let stack = serde_json::from_slice(bytes)
            .map_err(|err| eyre!("Unable to deserialize history: {}", err,))?;

        trace!("Deserialized history:\n{:#?}", stack);

        Ok(stack)
    }
}
