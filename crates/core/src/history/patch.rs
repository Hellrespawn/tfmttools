use serde::{Deserialize, Serialize};

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
