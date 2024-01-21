use color_eyre::Result;
use tfmttools_core::history::LoadActionHistoryResult;

use super::Command;
use crate::config::Config;
use crate::history::{
    load_history, HistoryFormat, HistoryFormatter, HistoryPrefix,
};

#[derive(Debug)]
pub struct ShowHistory {
    formatter: HistoryFormatter,
}

impl ShowHistory {
    pub fn new(verbosity: u8) -> Self {
        let formatter =
            HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));

        Self {
            formatter: if verbosity > 0 {
                formatter.with_format(HistoryFormat::Verbose)
            } else {
                formatter
            },
        }
    }
}

impl Command for ShowHistory {
    fn run(&self, config: &Config) -> Result<()> {
        let load_history_result = load_history(config)?;

        match load_history_result {
            LoadActionHistoryResult::Loaded(history) => {
                println!("{}", self.formatter.format_history(&history));
            },
            LoadActionHistoryResult::New(_) => {
                println!("There is no history.");
            },
        }

        Ok(())
    }
}
