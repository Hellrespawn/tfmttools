use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionHistory, ActionRecord};
use tfmttools_fs::{ActionHandler, FsHandler};
use tfmttools_history::{HistoryMode, LoadHistoryResult};

use super::super::config::Config;
use super::Command;
use crate::history::{load_history, HistoryFormatter, HistoryPrefix};
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList};

#[derive(Debug)]
pub struct UndoRedoCommand {
    force: bool,

    amount: usize,
    mode: HistoryMode,
    formatter: HistoryFormatter,
}

impl UndoRedoCommand {
    pub fn new(force: bool, amount: usize, mode: HistoryMode) -> Self {
        Self {
            force,
            amount,
            mode,
            formatter: HistoryFormatter::new()
                .with_prefix(HistoryPrefix::Ordered(')')),
        }
    }

    fn undo_redo(&self, config: &Config) -> Result<()> {
        let verb = match self.mode {
            HistoryMode::Undo => "undo",
            HistoryMode::Redo => "redo",
        };

        let load_history_result = load_history(config)?;

        match load_history_result {
            LoadHistoryResult::New(_) => {
                eprintln!("There is no history to {verb}.");
                Ok(())
            },
            LoadHistoryResult::Loaded(mut history) => {
                let records = self.get_records(&mut history);

                let amount = self.amount;
                let actual = records.len();

                if actual < amount {
                    println!(
                    "Tried to {verb} {amount} runs, but only {actual} can be {verb}ne.",
                    verb = self.mode.verb()
                );
                }

                if records.is_empty() {
                    println!("There are no runs to {verb}.");
                } else {
                    let confirmation =
                        self.force || self.confirm_undo_redo(&records)?;

                    if confirmation {
                        self.perform_undo_redo_actions(
                            &records,
                            config.fs_handler(),
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

    fn get_records<'h>(
        &self,
        history: &'h mut ActionHistory,
    ) -> Vec<&'h ActionRecord> {
        match self.mode {
            HistoryMode::Undo => {
                history.pop_records_to_undo(self.amount).collect()
            },
            HistoryMode::Redo => {
                history.unpop_records_to_redo(self.amount).collect()
            },
        }
    }

    fn confirm_undo_redo(&self, records: &[&ActionRecord]) -> Result<bool> {
        self.preview_undo_redo(records)?;

        let item_name = ItemName::simple("record");

        let amount = records.len();

        let prompt_message = format!(
            "{} {} {}?",
            self.mode.verb_capitalized(),
            amount,
            item_name.by_amount(amount)
        );

        let confirmation_prompt = ConfirmationPrompt::new(&prompt_message);

        confirmation_prompt.prompt()
    }

    fn preview_undo_redo(&self, records: &[&ActionRecord]) -> Result<()> {
        const LEADING_LINES: usize = 3;
        const TRAILING_LINES: usize = 3;

        let iter =
            records.iter().map(|record| self.formatter.format_record(record));

        let preview_list = PreviewList::new(iter)
            .leading(LEADING_LINES)
            .trailing(TRAILING_LINES)
            .item_name(ItemName::simple("record"));

        preview_list.print()?;

        Ok(())
    }

    fn perform_undo_redo_actions(
        &self,
        records: &[&ActionRecord],
        fs_handler: &FsHandler,
    ) -> Result<()> {
        let action_handler = ActionHandler::new(fs_handler);

        for record in records {
            println!(
                "{}ing {}...",
                self.mode.verb_capitalized(),
                self.formatter.format_record(record)
            );

            let actions: Vec<&Action> = match self.mode {
                HistoryMode::Undo => record.iter().rev().collect(),
                HistoryMode::Redo => record.iter().collect(),
            };

            for action in actions {
                match self.mode {
                    HistoryMode::Undo => action_handler.undo(action)?,
                    HistoryMode::Redo => action_handler.redo(action)?,
                }
            }

            println!("Done.");
        }

        Ok(())
    }
}

impl Command for UndoRedoCommand {
    fn run(&self, config: &Config) -> Result<()> {
        self.undo_redo(config)
    }
}
