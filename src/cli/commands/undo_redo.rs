use color_eyre::Result;
use history::{History, LoadHistoryResult, Record};
use once_cell::sync::Lazy;
use time::format_description::{self, FormatItem};

use super::super::config::Config;
use super::Command;
use crate::action::Action;
use crate::cli::preview::{preview, PreviewData};
use crate::cli::HistoryMode;

static DATE_FORMAT: Lazy<Vec<FormatItem>> = Lazy::new(|| {
    format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("Unable to parse date format.")
});

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
            // FIXME Show records in the correct order in preview.
            for record in records.iter().rev() {
                print!("Undoing {}... ", format_record(record));

                for action in record.iter().rev() {
                    action.undo(dry_run)?;
                }

                println!("Done.");
            }
        },
        HistoryMode::Redo => {
            for record in records {
                print!("Redoing {}... ", format_record(record));

                for action in record.iter() {
                    action.redo(dry_run)?;
                }

                println!("Done.");
            }
        },
    }

    Ok(())
}

pub fn format_record(record: &Record<Action>) -> String {
    let no_of_moves = record.items().iter().filter(|a| a.is_move()).count();

    let no_of_mk_dirs = record.items().iter().filter(|a| a.is_mk_dir()).count();

    let no_of_rm_dirs = record.items().iter().filter(|a| a.is_rm_dir()).count();

    let mut string = format!("[{}] ", record.len());

    if no_of_mk_dirs > 1 {
        string += &format!("{no_of_mk_dirs} created directories, ");
    }

    if no_of_moves > 1 {
        string += &format!("{no_of_moves} moved files, ");
    }

    if no_of_rm_dirs > 1 {
        string += &format!("{no_of_rm_dirs} removed directories, ");
    }

    string = string[0..string.len() - 2].to_owned();

    if let Some(timestamp) = record.timestamp() {
        string += &format!(
            " ({})",
            timestamp
                .format(&DATE_FORMAT)
                .expect("Unable to format timestamp.")
        );
    }

    string
}
