use color_eyre::eyre::eyre;
use color_eyre::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::record::Record;
use super::stack::RefStack;

pub struct HistorySerde;

impl HistorySerde {
    #[cfg(feature = "debug")]
    pub fn serialize<T>(stack: &RefStack<Record<T>>) -> Result<Vec<u8>>
    where
        T: std::fmt::Debug + Serialize + DeserializeOwned,
    {
        Ok(serde_json::to_vec(stack)?)
    }

    #[cfg(not(feature = "debug"))]
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
        let bincode_result = bincode::deserialize(bytes);

        let json_result = serde_json::from_slice(bytes);

        if let Ok(stack) = bincode_result {
            Ok(stack)
        } else if let Ok(stack) = json_result {
            Ok(stack)
        } else {
            Err(eyre!(
                "Unable to deserialize history:\nbincode: {}\njson: {}",
                bincode_result.unwrap_err(),
                json_result.unwrap_err()
            ))
        }
    }
}
