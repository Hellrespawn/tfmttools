use color_eyre::Result;
use history::{History, LoadHistoryResult, Record};

use super::super::config::Config;
use super::Command;
use crate::action::Action;
use crate::cli::preview::{preview, PreviewData};
use crate::cli::HistoryMode;

// TODO Summarize actions undone/redone

#[derive(Debug)]
pub struct UndoRedo {
    force: bool,

    amount: usize,
    mode: HistoryMode,
}

impl UndoRedo {
    pub fn new(force: bool, amount: usize, mode: HistoryMode) -> Self {
        Self { force, amount, mode }
    }
}

impl Command for UndoRedo {
    fn run(&self, config: &Config) -> Result<()> {
        undo_redo(config, self.force, self.amount, self.mode)
    }
}

fn undo_redo(
    config: &Config,
    force: bool,
    amount: usize,
    mode: HistoryMode,
) -> Result<()> {
    let verb = match mode {
        HistoryMode::Undo => "undo",
        HistoryMode::Redo => "redo",
    };

    let result = History::<Action>::load(&config.history_file())?;

    match result {
        LoadHistoryResult::New(_) => {
            eprintln!("There is no history to {verb}.");
            Ok(())
        },
        LoadHistoryResult::Loaded(mut history) => {
            let records = match mode {
                HistoryMode::Undo => history.get_records_to_undo(amount),
                HistoryMode::Redo => history.get_records_to_redo(amount),
            };

            if records.is_empty() {
                println!("There are no runs to {verb}.");
            } else {
                let confirmation =
                    force || preview_undo_redo(records, amount, mode)?;

                if confirmation {
                    perform_undo_redo_actions(records, config.dry_run(), mode)?;

                    history.save()?;
                } else {
                    println!("Aborting!");
                }
            }

            Ok(())
        },
    }
}

fn preview_undo_redo(
    records: &[Record<Action>],
    amount: usize,
    mode: HistoryMode,
) -> Result<bool> {
    let data = match mode {
        HistoryMode::Undo => PreviewData::undo(records, amount),
        HistoryMode::Redo => PreviewData::redo(records, amount),
    };

    preview(&data)
}

fn perform_undo_redo_actions(
    records: &[Record<Action>],
    dry_run: bool,
    mode: HistoryMode,
) -> Result<()> {
    match mode {
        HistoryMode::Undo => {
            for record in records.iter().rev() {
                for action in record.iter().rev() {
                    action.undo(dry_run)?;
                }
            }
        },
        HistoryMode::Redo => {
            for record in records {
                for action in record.iter() {
                    action.redo(dry_run)?;
                }
            }
        },
    }

    Ok(())
}
