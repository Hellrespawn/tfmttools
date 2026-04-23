use assert_fs::TempDir;
use camino::Utf8PathBuf;
use color_eyre::Result;
use tfmttools_test_harness::{FixtureDirs, copy_files};

const INPUT_AUDIO_DIR_NAME: &str = "input";
const INPUT_EXTRA_DIR_NAME: &str = "extra";
const CONFIG_DIR_NAME: &str = "config";

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

    pub fn persist_work_dir_if(self, persist: bool) {
        let _ = self.temp_dir.into_persistent_if(persist);
    }
}

pub fn populate_files(
    fixture_dirs: &FixtureDirs,
    context: &TestContext,
) -> Result<()> {
    copy_files(fixture_dirs.template_dir(), &context.config_work_dir())?;
    copy_files(fixture_dirs.audio_dir(), &context.input_audio_dir())?;
    copy_files(fixture_dirs.extra_dir(), &context.input_extra_dir())?;

    Ok(())
}
