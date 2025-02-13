pub mod clear_history;
pub mod copy_tags;
pub mod fix;
pub mod list_templates;
pub mod rename;
pub mod show_history;
pub mod undo_redo;

use color_eyre::Result;
use tfmttools_fs::FsHandler;

use crate::config::paths::AppPaths;

pub trait Command: std::fmt::Debug {
    fn run(&self, app_paths: &AppPaths, fs_handler: &FsHandler) -> Result<()>;
}
