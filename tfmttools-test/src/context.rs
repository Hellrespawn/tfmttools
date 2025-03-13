use assert_fs::TempDir;
use camino::Utf8PathBuf;
use color_eyre::Result;

const TEST_DATA_DIRECTORY: &str = "../tfmttools-test/testdata";
const TEST_CASE_DIR_NAME: &str = "cases";
const AUDIO_DIR_NAME: &str = "audio";
const EXTRA_DIR_NAME: &str = "extra";
const TEMPLATE_DIR_NAME: &str = "template";

const INPUT_AUDIO_DIR_NAME: &str = "input";
const INPUT_EXTRA_DIR_NAME: &str = "extra";
const CONFIG_DIR_NAME: &str = "config";

const TEST_REPORT_TEMPLATE_NAME: &str = "test-template.html";
const TEST_REPORT_DIR: &str = "report";

pub struct TestContext {
    temp_dir: TempDir,
}

impl TestContext {
    pub fn new() -> Result<Self> {
        Ok(Self { temp_dir: TempDir::new()? })
    }

    pub fn work_dir_path(&self) -> Utf8PathBuf {
        self.temp_dir.to_path_buf().try_into().expect("tempdir should be UTF-8")
    }

    pub fn input_audio_dir(&self) -> Utf8PathBuf {
        self.work_dir_path().join(INPUT_AUDIO_DIR_NAME)
    }

    pub fn input_extra_dir(&self) -> Utf8PathBuf {
        self.input_audio_dir().join(INPUT_EXTRA_DIR_NAME)
    }

    pub fn config_work_dir(&self) -> Utf8PathBuf {
        self.work_dir_path().join(CONFIG_DIR_NAME)
    }

    pub fn persist_work_dir_if(self, bool: bool) {
        let _ = self.temp_dir.into_persistent_if(bool);
    }
}

pub struct SourceDirs;

impl SourceDirs {
    pub fn test_data_dir() -> Utf8PathBuf {
        Utf8PathBuf::from(TEST_DATA_DIRECTORY)
    }

    pub fn test_case_dir() -> Utf8PathBuf {
        Self::test_data_dir().join(TEST_CASE_DIR_NAME)
    }

    pub fn audio_dir() -> Utf8PathBuf {
        Self::test_data_dir().join(AUDIO_DIR_NAME)
    }

    pub fn extra_dir() -> Utf8PathBuf {
        Self::test_data_dir().join(EXTRA_DIR_NAME)
    }

    pub fn template_dir() -> Utf8PathBuf {
        Self::test_data_dir().join(TEMPLATE_DIR_NAME)
    }

    pub fn test_report_template_path() -> Utf8PathBuf {
        Self::test_data_dir().join(TEST_REPORT_TEMPLATE_NAME)
    }

    pub fn test_report_output_dir() -> Utf8PathBuf {
        Self::test_data_dir().join(TEST_REPORT_DIR)
    }
}
