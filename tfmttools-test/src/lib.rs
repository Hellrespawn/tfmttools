mod file_tree;
mod predicates;
mod runner;
mod test_case;
mod test_case_data;
mod test_failure;

pub use runner::test_runner;

pub static TEST_DATA_DIRECTORY: &str = "../testdata";

pub static TEST_CASE_DIR_NAME: &str = "cases";
pub static TEST_AUDIO_FILE_DIR_NAME: &str = "files";
pub static TEST_TEMPLATE_DIR_NAME: &str = "template";
pub static TEST_CONFIG_DIR_NAME: &str = "config";
