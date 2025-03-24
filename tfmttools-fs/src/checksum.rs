use std::hash::Hash;
use std::io::Read;

use adler::Adler32;
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

    let mut adler = Adler32::new();

    buf.hash(&mut adler);

    Ok(format!("{:X}", adler.checksum()))
}
