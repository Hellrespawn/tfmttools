use std::fmt::Write;
use std::io::Read;

use camino::Utf8Path;
use sha2::{Digest, Sha256};

const MAX_BYTES_TO_READ: u64 = 60 * 1024 * 1024;

pub fn get_path_checksum(path: &Utf8Path) -> std::io::Result<String> {
    // shortcut: compare only the first 60 MiB, use full hashes if fixtures need it.
    let mut file = fs_err::File::open(path)?.take(MAX_BYTES_TO_READ);
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => hash.update(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {},
            Err(error) => return Err(error),
        }
    }
    let mut checksum = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(checksum, "{byte:02x}").unwrap();
    }
    Ok(checksum)
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom, Write};

    use super::*;

    #[test]
    fn hashes_file_bytes_as_sha256() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"abc").unwrap();
        let path = Utf8Path::from_path(file.path()).unwrap();
        assert_eq!(
            get_path_checksum(path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashes_only_the_first_sixty_mib() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().set_len(MAX_BYTES_TO_READ).unwrap();
        let path = Utf8Path::from_path(file.path()).unwrap().to_owned();
        let original = get_path_checksum(&path).unwrap();
        file.seek(SeekFrom::End(0)).unwrap();
        file.write_all(b"ignored suffix").unwrap();
        assert_eq!(get_path_checksum(&path).unwrap(), original);
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(b"changed prefix").unwrap();
        assert_ne!(get_path_checksum(&path).unwrap(), original);
    }
}
