use camino::Utf8PathBuf;
use color_eyre::Result;

const TEST_CASE_DIR_NAME: &str = "cases";
const AUDIO_DIR_NAME: &str = "audio";
const EXTRA_DIR_NAME: &str = "extra";
const TEMPLATE_DIR_NAME: &str = "template";
const REPORTS_ROOT: &str = "../../tests/reports";

pub struct FixtureDirs {
    root: Utf8PathBuf,
}

impl FixtureDirs {
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Utf8PathBuf {
        &self.root
    }

    pub fn test_case_dir() -> Utf8PathBuf {
        Self::cli().case_dir()
    }

    pub fn cli() -> Self {
        Self::new("../../tests/fixtures/cli")
    }

    pub fn case_dir(&self) -> Utf8PathBuf {
        self.root.join(TEST_CASE_DIR_NAME)
    }

    pub fn audio_dir(&self) -> Utf8PathBuf {
        self.root.join(AUDIO_DIR_NAME)
    }

    pub fn extra_dir(&self) -> Utf8PathBuf {
        self.root.join(EXTRA_DIR_NAME)
    }

    pub fn template_dir(&self) -> Utf8PathBuf {
        self.root.join(TEMPLATE_DIR_NAME)
    }

    pub fn reports_dir() -> Utf8PathBuf {
        Utf8PathBuf::from(REPORTS_ROOT)
    }
}

pub fn copy_files(
    source_dir: Utf8PathBuf,
    target_dir: &camino::Utf8Path,
) -> Result<()> {
    let paths = fs_err::read_dir(&source_dir)?
        .map(|entry| {
            let path = entry?.path();
            Ok(Utf8PathBuf::try_from(path)?)
        })
        .collect::<Result<Vec<_>>>()?;

    fs_err::create_dir(target_dir)?;

    for path in &paths {
        let file_name =
            path.file_name().expect("fixture path should include a file name");

        fs_err::copy(path, target_dir.join(file_name))?;
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    use super::*;

    #[test]
    fn fixture_copy_rejects_incomplete_scan_before_creating_destination() {
        let root = std::env::temp_dir()
            .join(format!("tfmt-fixture-copy-{}", std::process::id()));
        let source = root.join("source");
        let target = Utf8PathBuf::try_from(root.join("target")).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("valid"), b"valid").unwrap();
        std::fs::write(
            source.join(OsString::from_vec(b"invalid-\xff".to_vec())),
            b"invalid",
        )
        .unwrap();
        let result =
            copy_files(Utf8PathBuf::try_from(source).unwrap(), &target);
        assert!(result.is_err());
        assert!(!target.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
