use camino::Utf8PathBuf;
use color_eyre::Result;

use super::Command;
use crate::config::Config;

#[derive(Debug)]
pub struct ShowHistory;

impl Command for ShowHistory {
    fn run(&self, config: &Config) -> Result<()> {
        let path = &config.history_file();

        eprintln!("Not yet implemented");

        Ok(())
    }
}
