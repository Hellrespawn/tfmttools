use camino::Utf8PathBuf;
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::util::Utf8File;

use crate::error::{FsError, FsResult};

pub fn read_audio_file(path: Utf8PathBuf) -> FsResult<AudioFile> {
    let tagged_file = match lofty::read_from_path(&path) {
        Ok(tagged_file) => tagged_file,
        Err(err) => return Err(FsError::Lofty(path, err)),
    };

    Ok(AudioFile::from_tagged_file(Utf8File::new(&path), &tagged_file)?)
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;

    use super::*;

    #[test]
    fn read_audio_file_errors_on_nonexistent_path() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = Utf8PathBuf::try_from(temp_dir.path().join("missing.mp3"))?;

        let error = read_audio_file(path).unwrap_err();

        assert!(matches!(error, FsError::Lofty(_, _)));

        Ok(())
    }
}
