use color_eyre::Result;
use tfmttools_core::history::ActionHistory;
use tfmttools_history::LoadHistoryResult;

use super::Command;
use crate::config::Config;
use crate::history::{RecordFormat, RecordFormatter};

#[derive(Debug)]
pub struct ShowHistory {
    formatter: RecordFormatter,
}

impl ShowHistory {
    pub fn new(verbose: bool) -> Self {
        Self {
            formatter: RecordFormatter::new(if verbose {
                RecordFormat::Verbose
            } else {
                RecordFormat::Normal
            }),
        }
    }
}

impl Command for ShowHistory {
    fn run(&self, config: &Config) -> Result<()> {
        let load_history_result = ActionHistory::load(&config.history_file())?;

        if let LoadHistoryResult::New(_) = &load_history_result {
            print_no_history();
        } else {
            let history = load_history_result.unwrap();

            let undo = history.get_records_to_undo().collect::<Vec<_>>();

            let redo = history.get_records_to_redo().collect::<Vec<_>>();

            if undo.is_empty() && redo.is_empty() {
                print_no_history();
            } else {
                if undo.is_empty() {
                    println!("There is nothing to undo.");
                } else {
                    println!("Undo history:");
                }

                for record in undo {
                    println!("{}", self.formatter.format_record(record));
                }

                println!();

                if redo.is_empty() {
                    println!("There is nothing to redo.");
                } else {
                    println!("Redo history:");
                }

                for record in redo {
                    println!("{}", self.formatter.format_record(record));
                }
            }
        }

        Ok(())
    }
}

fn print_no_history() {
    println!("There is no history to display.");
}
