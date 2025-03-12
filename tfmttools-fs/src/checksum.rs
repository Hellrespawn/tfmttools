use std::hash::Hash;

use adler::Adler32;
use camino::Utf8Path;
use tfmttools_core::error::TFMTResult;

pub fn get_filename_checksum(filename: &str) -> String {
    let mut adler = Adler32::new();

    filename.hash(&mut adler);

    format!("{:X}", adler.checksum())
}

pub fn get_file_checksum(path: &Utf8Path) -> TFMTResult<String> {
    let mut adler = Adler32::new();

    let file_body = fs_err::read(path)?;
    file_body.hash(&mut adler);

    Ok(format!("{:X}", adler.checksum()))
}
