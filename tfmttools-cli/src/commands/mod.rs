pub mod clear_history;
pub mod list_templates;
pub mod rename;
pub mod show_history;
pub mod undo_redo;

use color_eyre::Result;

use super::config::Config;

pub trait Command: std::fmt::Debug {
    fn run(&self, config: &Config) -> Result<()>;
}
