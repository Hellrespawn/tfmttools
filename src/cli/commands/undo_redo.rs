use clap::Args;
use color_eyre::Result;
use history::{History, LoadHistoryResult, Record};

use super::super::config::Config;
use super::Command;
use crate::action::Action;
use crate::cli::preview::{preview, PreviewData};

// TODO Summarize actions undone/redone
// TODO Add interactive preview

#[derive(Copy, Clone)]
pub enum HistoryMode {
    Undo,
    Redo,
}

impl HistoryMode {
    pub fn verb(&self) -> &str {
        match self {
            HistoryMode::Undo => "undo",
            HistoryMode::Redo => "redo",
        }
    }

    pub fn verb_capitalized(&self) -> &str {
        match self {
            HistoryMode::Undo => "Undo",
            HistoryMode::Redo => "Redo",
        }
    }
}

#[derive(Args, Debug)]
pub struct Undo {
    #[arg(short, long)]
    dry_run: bool,

    #[arg(short, long)]
    force: bool,

    #[arg(default_value_t = 1)]
    amount: usize,
}

#[derive(Args, Debug)]
pub struct Redo {
    #[arg(short, long)]
    dry_run: bool,

    #[arg(short, long)]
    force: bool,

    #[arg(default_value_t = 1)]
    amount: usize,
}

impl Command for Undo {
    fn run(&self, config: &Config) -> Result<()> {
        undo_redo(
            config,
            self.dry_run,
            self.force,
            self.amount,
            HistoryMode::Undo,
        )
    }
}

impl Command for Redo {
    fn run(&self, config: &Config) -> Result<()> {
        undo_redo(
            config,
            self.dry_run,
            self.force,
            self.amount,
            HistoryMode::Redo,
        )
    }
}

fn undo_redo(
    config: &Config,
    dry_run: bool,
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

            if let Some(records) = records {
                let confirmation =
                    force || preview_undo_redo(records, amount, mode)?;

                if confirmation {
                    perform_undo_redo_actions(records, dry_run, mode)?;

                    history.save()?;
                } else {
                    println!("Aborting!");
                }
            } else {
                eprintln!("There are no runs to {verb}.");
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
