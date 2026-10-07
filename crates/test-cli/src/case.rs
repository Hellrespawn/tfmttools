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

pub fn remap_initial_files(
    dirs: &FixtureDirs,
    context: &TestContext,
    data: &tfmttools_test_harness::TestCaseData,
) -> Result<()> {
    for (destination, source) in data.initial_sources() {
        let target = context.input_audio_dir().join(destination);
        fs_err::create_dir_all(
            target.parent().expect("Fixture destination has a parent"),
        )?;
        fs_err::copy(dirs.audio_dir().join(source), target)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;
    use tfmttools_test_harness::TestCaseData;

    use super::*;

    #[test]
    fn initial_sources_copy_from_original_files_during_swap() {
        let fixtures = assert_fs::TempDir::new().unwrap();
        fixtures.child("audio").create_dir_all().unwrap();
        fixtures.child("extra").create_dir_all().unwrap();
        fixtures.child("template").create_dir_all().unwrap();
        fixtures.child("audio/one.mp3").write_str("one").unwrap();
        fixtures.child("audio/two.mp3").write_str("two").unwrap();
        fixtures
            .child("case.json")
            .write_str(
                r#"{
            "description": "swap", "expectations": {}, "tests": {},
            "initial-sources": { "one.mp3": "two.mp3", "two.mp3": "one.mp3", "selected/one.mp3": "one.mp3" }
        }"#,
            )
            .unwrap();
        let root =
            Utf8PathBuf::from_path_buf(fixtures.path().to_owned()).unwrap();
        let data = TestCaseData::from_file(&root.join("case.json")).unwrap();
        let fixtures = FixtureDirs::new(root);
        let context = TestContext::new().unwrap();
        populate_files(&fixtures, &context).unwrap();
        remap_initial_files(&fixtures, &context, &data).unwrap();
        assert_eq!(
            fs_err::read_to_string(context.input_audio_dir().join("one.mp3"))
                .unwrap(),
            "two"
        );
        assert_eq!(
            fs_err::read_to_string(context.input_audio_dir().join("two.mp3"))
                .unwrap(),
            "one"
        );
    }
}
