use color_eyre::Result;
use tfmttools_core::history::LoadActionHistoryResult;

use super::Command;
use crate::config::Config;
use crate::history::{load_history, HistoryFormat, HistoryFormatter};

#[derive(Debug)]
pub struct ShowHistory {
    formatter: HistoryFormatter,
}

impl ShowHistory {
    pub fn new(verbose: bool) -> Self {
        Self {
            formatter: HistoryFormatter::new(if verbose {
                HistoryFormat::Verbose
            } else {
                HistoryFormat::Normal
            }),
        }
    }
}

impl Command for ShowHistory {
    fn run(&self, config: &Config) -> Result<()> {
        let load_history_result = load_history(config)?;

        match load_history_result {
            LoadActionHistoryResult::Loaded(history) => {
                println!("{}", self.formatter.format(&history));
            },
            LoadActionHistoryResult::New(_) => {
                println!("There is no history.");
            },
        }

        Ok(())
    }
}
