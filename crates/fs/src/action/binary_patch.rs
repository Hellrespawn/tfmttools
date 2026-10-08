use std::io::{self, Write};

use qbsdiff::{Bsdiff, Bspatch};
use sha2::{Digest, Sha256};
use tfmttools_core::history::{BinaryPatchPair, ByteIdentity, HistoryMode};

use crate::error::{FsError, FsResult};

pub fn byte_identity(bytes: &[u8]) -> ByteIdentity {
    ByteIdentity {
        length: bytes.len() as u64,
        sha256: Sha256::digest(bytes).into(),
    }
}

pub fn create_patch_pair(
    before: &[u8],
    after: &[u8],
) -> FsResult<BinaryPatchPair> {
    let mut forward = Vec::new();
    let mut reverse = Vec::new();
    Bsdiff::new(before, after).compare(&mut forward)?;
    Bsdiff::new(after, before).compare(&mut reverse)?;
    let pair = BinaryPatchPair {
        format: "bsdiff40-v1".into(),
        before: byte_identity(before),
        after: byte_identity(after),
        forward,
        reverse,
    };
    if apply_patch(before, &pair, HistoryMode::Redo)? != after
        || apply_patch(after, &pair, HistoryMode::Undo)? != before
    {
        return Err(FsError::Recovery(
            "Binary patches failed exact round-trip verification".into(),
        ));
    }
    Ok(pair)
}

pub fn apply_patch(
    bytes: &[u8],
    pair: &BinaryPatchPair,
    direction: HistoryMode,
) -> FsResult<Vec<u8>> {
    if pair.format != "bsdiff40-v1" {
        return Err(FsError::Recovery(
            "Unsupported binary patch format".into(),
        ));
    }
    let (expected, output, patch) = match direction {
        HistoryMode::Undo => (&pair.after, &pair.before, &pair.reverse),
        HistoryMode::Redo => (&pair.before, &pair.after, &pair.forward),
    };
    if &byte_identity(bytes) != expected {
        return Err(FsError::Recovery(
            "File changed since the recorded operation; refusing replay".into(),
        ));
    }
    let patcher = Bspatch::new(patch)?;
    if patcher.hint_target_size() != output.length {
        return Err(FsError::Recovery(
            "Patch target length differs from recorded length".into(),
        ));
    }
    let mut writer = BoundedOutput { bytes: Vec::new(), limit: output.length };
    patcher.apply(bytes, &mut writer)?;
    if &byte_identity(&writer.bytes) != output {
        return Err(FsError::Recovery(
            "Patched bytes differ from recorded result".into(),
        ));
    }
    Ok(writer.bytes)
}

struct BoundedOutput {
    bytes: Vec<u8>,
    limit: u64,
}
impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64
            > self.limit.saturating_sub(self.bytes.len() as u64)
        {
            return Err(io::Error::other(
                "Patch exceeds recorded target length",
            ));
        }
        self.bytes.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
