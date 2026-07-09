use camino::Utf8Path;
use tfmttools_core::util::{Utf8Directory, Utf8File};

use crate::error::{FsError, FsResult};

pub fn verify_file(path: impl AsRef<Utf8Path>) -> FsResult<Utf8File> {
    let path = path.as_ref();

    if path.is_file() || !path.exists() {
        Ok(Utf8File::new(path))
    } else {
        Err(FsError::NotAFile(path.to_owned()))
    }
}

pub fn verify_directory(
    path: impl AsRef<Utf8Path>,
) -> FsResult<Utf8Directory> {
    let path = path.as_ref();

    if path.is_dir() || !path.exists() {
        Ok(Utf8Directory::new(path))
    } else {
        Err(FsError::NotADirectory(path.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;

    use super::*;

    #[test]
    fn verify_file_accepts_missing_path() -> Result<()> {
        let path = Utf8PathBuf::from("/does/not/exist.mp3");

        assert!(verify_file(&path).is_ok());

        Ok(())
    }

    #[test]
    fn verify_file_rejects_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = Utf8PathBuf::try_from(temp_dir.path().to_owned())?;

        let error = verify_file(&path).unwrap_err();

        assert!(matches!(error, FsError::NotAFile(_)));

        Ok(())
    }

    #[test]
    fn verify_directory_rejects_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = Utf8PathBuf::try_from(temp_dir.path().join("f.mp3"))?;
        fs_err::write(&path, "x")?;

        let error = verify_directory(&path).unwrap_err();

        assert!(matches!(error, FsError::NotADirectory(_)));

        Ok(())
    }
}
