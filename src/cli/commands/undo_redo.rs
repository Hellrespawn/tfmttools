use clap::Args;
use color_eyre::Result;

use super::super::config::Config;
use super::Command;
use crate::history::{History, LoadHistoryResult};

// TODO Summarize actions undone/redone
// TODO Add interactive preview

#[derive(Copy, Clone)]
pub enum HistoryMode {
    Undo,
    Redo,
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

    fn override_dry_run(&mut self, dry_run: bool) {
        if dry_run {
            self.dry_run = true;
        }
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

    fn override_dry_run(&mut self, dry_run: bool) {
        if dry_run {
            self.dry_run = true;
        }
    }
}

fn undo_redo(
    config: &Config,
    dry_run: bool,
    _force: bool,
    amount: usize,
    mode: HistoryMode,
) -> Result<()> {
    let verb = match mode {
        HistoryMode::Undo => "undo",
        HistoryMode::Redo => "redo",
    };

    let result = History::load(&config.history_file())?;

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
                let delta = amount - records.len();

                if delta > 0 {
                    println!("Tried to {verb} {amount} runs, but only {delta} can be {verb}ne.");
                }

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

                history.save()?;

                Ok(())
            } else {
                eprintln!("There are no runs to {verb}.");
                Ok(())
            }
        },
    }
}
