use color_eyre::Result;
use tfmttools_core::history::LoadActionHistoryResult;
use tfmttools_fs::FsHandler;

use super::Command;
use crate::config::paths::AppPaths;
use crate::history::{
    load_history, HistoryFormat, HistoryFormatter, HistoryPrefix,
};

#[derive(Debug)]
pub struct ShowHistoryCommand {
    formatter: HistoryFormatter,
}

impl ShowHistoryCommand {
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

impl Command for ShowHistoryCommand {
    fn run(&self, app_paths: &AppPaths, _fs_handler: &FsHandler) -> Result<()> {
        let load_history_result = load_history(&app_paths.history_file())?;

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
