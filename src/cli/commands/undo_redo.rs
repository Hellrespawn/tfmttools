use color_eyre::Result;

use crate::config::Config;
use crate::history::{History, LoadHistoryResult};

// TODO Summarize actions undone/redone

#[derive(Copy, Clone)]
pub(crate) enum HistoryMode {
    Undo(usize),
    Redo(usize),
}

impl HistoryMode {
    fn amount(&self) -> usize {
        match self {
            Self::Undo(n) | Self::Redo(n) => *n,
        }
    }
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
                let amount = mode.amount();
                let delta = amount - records.len();

                if delta > 0 {
                    println!("Tried to {verb} {amount} runs, but only {delta} can be {verb}ne.");
                }

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
                eprintln!("There are no runs to {verb}.");
                Ok(())
            }
        },
    }
}
