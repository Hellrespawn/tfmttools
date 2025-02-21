use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecord, ActionRecordMetadata};
use tfmttools_fs::{ActionHandler, FsHandler};
use tfmttools_history::{History, HistoryMode, LoadHistoryResult, RecordState};

use crate::config::paths::AppPaths;
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList};

#[derive(Debug)]
pub struct UndoRedoCommand {
    yes: bool,

    amount: usize,
    mode: HistoryMode,
    formatter: HistoryFormatter,
}

impl UndoRedoCommand {
    pub fn new(yes: bool, amount: usize, mode: HistoryMode) -> Self {
        Self {
            yes,
            amount,
            mode,
            formatter: HistoryFormatter::new()
                .with_prefix(HistoryPrefix::Ordered(')')),
        }
    }

    pub fn run(
        &self,
        app_paths: &AppPaths,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        let verb = match self.mode {
            HistoryMode::Undo => "undo",
            HistoryMode::Redo => "redo",
        };

        let (mut history, load_history_result) =
            load_history(&app_paths.history_file())?;

        match load_history_result {
            LoadHistoryResult::New => {
                eprintln!("There is no history to {verb}.");
                Ok(())
            },
            LoadHistoryResult::Loaded => {
                let records = self.get_records(&mut history)?;

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
                        self.yes || self.confirm_undo_redo(&records)?;

                    if confirmation {
                        self.perform_undo_redo_actions(records, fs_handler)?;

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
        history: &'h mut impl History<Action, ActionRecordMetadata>,
    ) -> Result<Vec<&'h mut ActionRecord>> {
        let records = match self.mode {
            HistoryMode::Undo => {
                history.get_n_records_to_undo_mut(self.amount)?
            },
            HistoryMode::Redo => {
                history.get_n_records_to_redo_mut(self.amount)?
            },
        };

        Ok(records)
    }

    fn confirm_undo_redo(&self, records: &[&mut ActionRecord]) -> Result<bool> {
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

    fn preview_undo_redo(&self, records: &[&mut ActionRecord]) -> Result<()> {
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
        records: Vec<&mut ActionRecord>,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        let action_handler = ActionHandler::new(fs_handler, false);

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

            match self.mode {
                HistoryMode::Undo => record.set_state(RecordState::Undone),
                HistoryMode::Redo => record.set_state(RecordState::Redone),
            }

            println!("Done.");
        }

        Ok(())
    }
}
