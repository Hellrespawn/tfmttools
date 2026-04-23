use camino::Utf8PathBuf;
use color_eyre::Result;

const TEST_CASE_DIR_NAME: &str = "cases";
const AUDIO_DIR_NAME: &str = "audio";
const EXTRA_DIR_NAME: &str = "extra";
const TEMPLATE_DIR_NAME: &str = "template";
const TEST_REPORT_DIR: &str = "report";

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

    pub fn report_output_dir(&self) -> Utf8PathBuf {
        self.root.join(TEST_REPORT_DIR)
    }
}

pub fn copy_files(
    source_dir: Utf8PathBuf,
    target_dir: &camino::Utf8Path,
) -> Result<()> {
    let paths = fs_err::read_dir(source_dir)?
        .flat_map(|result| {
            result.map(|entry| Utf8PathBuf::from_path_buf(entry.path()))
        })
        .flatten()
        .collect::<Vec<_>>();

    fs_err::create_dir(target_dir)?;

    for path in &paths {
        let file_name =
            path.file_name().expect("fixture path should include a file name");

        fs_err::copy(path, target_dir.join(file_name))?;
    }

    Ok(())
}
