use color_eyre::Result;

use crate::config::Config;
use crate::history::{History, LoadHistoryResult};

// TODO Report difference in entered amount and actual amount
// Summarize actions undone/redone

#[derive(Copy, Clone)]
pub(crate) enum HistoryMode {
    Undo(usize),
    Redo(usize),
}

pub(crate) fn undo_redo(config: &Config, mode: HistoryMode) -> Result<()> {
    let verb = match mode {
        HistoryMode::Undo(_) => "undo",
        HistoryMode::Redo(_) => "redo",
    };

    let result = History::load(config.history_file())?;

    match result {
        LoadHistoryResult::New(_) => {
            eprintln!("There is no history to {verb}.");
            Ok(())
        },
        LoadHistoryResult::Loaded(mut history) => {
            let records = match mode {
                HistoryMode::Undo(n) => history.get_records_to_undo(n),
                HistoryMode::Redo(n) => history.get_records_to_redo(n),
            };

            if let Some(records) = records {
                match mode {
                    HistoryMode::Undo(_) => {
                        for record in records.iter().rev() {
                            for action in record.iter().rev() {
                                action.undo(config.dry_run())?;
                            }
                        }
                    },
                    HistoryMode::Redo(_) => {
                        for record in records {
                            for action in record.iter() {
                                action.redo(config.dry_run())?;
                            }
                        }
                    },
                }

                history.save()?;

                Ok(())
            } else {
                eprintln!("There is no history to {verb}.");
                Ok(())
            }
        },
    }
}
