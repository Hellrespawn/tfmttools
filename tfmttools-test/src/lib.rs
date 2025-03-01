mod file_tree;
mod predicates;
mod runner;
mod test_case;
mod test_case_data;
mod test_failure;

pub use runner::test_runner;

pub const TEST_DATA_DIRECTORY: &str = "../testdata";

pub const TEST_CASE_DIR_NAME: &str = "cases";
pub const TEST_AUDIO_FILE_DIR_NAME: &str = "files";
pub const TEST_TEMPLATE_DIR_NAME: &str = "template";
pub const TEST_CONFIG_DIR_NAME: &str = "config";

pub const TEST_RUN_ID: &str = "run_id";
