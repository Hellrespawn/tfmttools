use std::io::Read;

use camino::Utf8Path;
use tfmttools_core::error::TFMTResult;

// 60 megabytes
// Accounts for lossless audio
const MAX_BYTES_TO_READ: usize = 60 * 1024 * 1024;

pub fn get_file_checksum(path: &Utf8Path) -> TFMTResult<String> {
    let length =
        std::cmp::min(MAX_BYTES_TO_READ, path.metadata()?.len() as usize);

    let mut buf = vec![0u8; length];

    let mut file = fs_err::File::open(path)?;

    while file.read(&mut buf)? > 0 {
        // keep reading
    }

    Ok(format!("{:X}", adler::adler32_slice(&buf)))
}
