use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;

const TEST_CASE_DIR_NAME: &str = "cases";
const FILES_DIR_NAME: &str = "files";
const TEMPLATE_DIR_NAME: &str = "template";

pub struct TestContext {
    work_dir: WorkDir,
    source_dirs: SourceDirs,
}

impl TestContext {
    pub fn new(source_dirs: SourceDirs) -> Result<Self> {
        Ok(Self { source_dirs, work_dir: WorkDir::new()? })
    }

    pub fn work_dir(&self) -> &WorkDir {
        &self.work_dir
    }

    pub fn source_dirs(&self) -> &SourceDirs {
        &self.source_dirs
    }
}

#[derive(Clone)]
pub struct SourceDirs {
    path: Utf8PathBuf,
}

impl SourceDirs {
    pub fn new<P: AsRef<Utf8Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    pub fn test_case_dir(&self) -> Utf8PathBuf {
        self.path.join(TEST_CASE_DIR_NAME)
    }

    pub fn files_dir(&self) -> Utf8PathBuf {
        self.path.join(FILES_DIR_NAME)
    }

    pub fn template_dir(&self) -> Utf8PathBuf {
        self.path.join(TEMPLATE_DIR_NAME)
    }
}

const INPUT_DIR_NAME: &str = "input";
const CONFIG_DIR_NAME: &str = "config";

pub struct WorkDir {
    inner: TempDir,
}

impl WorkDir {
    pub fn new() -> Result<Self> {
        Ok(Self { inner: TempDir::new()? })
    }

    pub fn path(&self) -> Utf8PathBuf {
        self.inner.to_path_buf().try_into().expect("tempdir should be UTF-8")
    }

    pub fn input_dir(&self) -> Utf8PathBuf {
        self.path().join(INPUT_DIR_NAME)
    }

    pub fn config_dir(&self) -> Utf8PathBuf {
        self.path().join(CONFIG_DIR_NAME)
    }
}
