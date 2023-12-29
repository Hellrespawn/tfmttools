use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::util::format_record;
use tfmttools_history::{History, HistoryMode, LoadHistoryResult, Record};

use super::super::config::Config;
use super::Command;
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList};

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
            let records = get_records(&mut history, mode, amount);

            let actual = records.len();

            if actual < amount {
                println!(
                    "Tried to {verb} {amount} runs, but only {actual} can be {verb}ne.",
                    verb = mode.verb()
                );
            }

            if records.is_empty() {
                println!("There are no runs to {verb}.");
            } else {
                let confirmation =
                    force || confirm_undo_redo(&records, actual, mode)?;

                if confirmation {
                    perform_undo_redo_actions(
                        &records,
                        config.dry_run(),
                        mode,
                    )?;

                    history.save()?;
                } else {
                    println!("Aborting!");
                }
            }

            Ok(())
        },
    }
}

fn get_records(
    history: &mut History<Action>,
    mode: HistoryMode,
    amount: usize,
) -> Vec<&Record<Action>> {
    match mode {
        HistoryMode::Undo => history.get_records_to_undo(amount).collect(),
        HistoryMode::Redo => history.get_records_to_redo(amount).collect(),
    }
}

fn confirm_undo_redo(
    records: &[&Record<Action>],
    amount: usize,
    mode: HistoryMode,
) -> Result<bool> {
    preview_undo_redo(records);

    let item_name = ItemName::simple("record");

    let prompt_message = format!(
        "{} {} {}?",
        mode.verb_capitalized(),
        amount,
        item_name.by_amount(amount)
    );

    let confirmation_prompt = ConfirmationPrompt::new(&prompt_message);

    confirmation_prompt.prompt()
}

fn preview_undo_redo(records: &[&Record<Action>]) {
    const LEADING_LINES: usize = 3;
    const TRAILING_LINES: usize = 3;

    let iter = records.iter().map(|r| format_record(r));

    let preview_list = PreviewList::new(iter)
        .leading(LEADING_LINES)
        .trailing(TRAILING_LINES)
        .item_name(ItemName::simple("record"));

    preview_list.print();
}

fn perform_undo_redo_actions(
    records: &[&Record<Action>],
    dry_run: bool,
    mode: HistoryMode,
) -> Result<()> {
    match mode {
        HistoryMode::Undo => {
            for record in records {
                println!("Undoing {}...", format_record(record));

                for action in record.iter().rev() {
                    action.undo(dry_run)?;
                }
            }

            println!("Done.");
        },
        HistoryMode::Redo => {
            for record in records {
                println!("Redoing {}...", format_record(record));

                for action in record.iter() {
                    action.redo(dry_run)?;
                }
            }

            println!("Done.");
        },
    }

    Ok(())
}
