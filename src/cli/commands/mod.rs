pub mod clear_history;
pub mod list_templates;
pub mod rename;
pub mod seed;
pub mod undo_redo;

use color_eyre::Result;

use super::config::Config;

pub trait Command {
    fn run(&self, config: &Config) -> Result<()>;

    fn override_dry_run(&mut self, dry_run: bool);
}
