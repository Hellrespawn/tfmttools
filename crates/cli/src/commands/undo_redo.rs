use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecord, ActionRecordMetadata};
use tfmttools_fs::{ActionHandler, FsHandler};
use tfmttools_history::{History, HistoryMode, LoadHistoryResult, RecordState};

use crate::cli::TFMTOptions;
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList, PreviewListSize};

#[derive(Debug)]
pub struct UndoRedoCommand {
    yes: bool,

    amount: usize,
    mode: HistoryMode,
    formatter: HistoryFormatter,
    preview_list_size: PreviewListSize,
}

impl UndoRedoCommand {
    pub fn new(
        yes: bool,
        amount: usize,
        mode: HistoryMode,
        preview_list_size: PreviewListSize,
    ) -> Self {
        Self {
            yes,
            amount,
            mode,
            preview_list_size,
            formatter: HistoryFormatter::new()
                .with_prefix(HistoryPrefix::Ordered(')')),
        }
    }

    pub fn run(
        &self,
        app_options: &TFMTOptions,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        let verb = match self.mode {
            HistoryMode::Undo => "undo",
            HistoryMode::Redo => "redo",
        };

        let (mut history, load_history_result) =
            load_history(&app_options.history_file_path()?)?;

        match load_history_result {
            LoadHistoryResult::New => {
                eprintln!("There is no history to {verb}.");
                Ok(())
            },
            LoadHistoryResult::Loaded => {
                let records = self.get_records(&history)?;

                let amount = self.amount;
                let actual = records.len();

                if records.is_empty() {
                    println!("There are no runs to {verb}.");
                } else {
                    if actual < amount {
                        println!(
                            "Tried to {verb} {amount} runs, but only {actual} can be {verb}ne.",
                            verb = self.mode.verb()
                        );
                    }

                    let confirmation =
                        self.yes || self.confirm_undo_redo(&records)?;

                    if confirmation {
                        self.perform_undo_redo_actions(
                            &mut history,
                            records,
                            fs_handler,
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
        &self,
        history: &History<Action, ActionRecordMetadata>,
    ) -> Result<Vec<ActionRecord>> {
        Ok(match self.mode {
            HistoryMode::Undo => history.get_n_records_to_undo(self.amount)?,
            HistoryMode::Redo => history.get_n_records_to_redo(self.amount)?,
        })
    }

    fn confirm_undo_redo(&self, records: &[ActionRecord]) -> Result<bool> {
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

    fn preview_undo_redo(&self, records: &[ActionRecord]) -> Result<()> {
        let iter =
            records.iter().map(|record| self.formatter.format_record(record));

        let preview_list = PreviewList::new(iter, self.preview_list_size)
            .with_item_name(ItemName::simple("record"));

        preview_list.print()?;

        Ok(())
    }

    fn perform_undo_redo_actions(
        &self,
        history: &mut History<Action, ActionRecordMetadata>,
        records: Vec<ActionRecord>,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        let action_handler = ActionHandler::new(fs_handler);

        for record in records {
            println!(
                "{}ing {}...",
                self.mode.verb_capitalized(),
                self.formatter.format_record(&record)
            );

            match self.mode {
                HistoryMode::Undo => {
                    for action in record.iter().rev() {
                        action_handler.undo(action)?;
                    }
                },
                HistoryMode::Redo => {
                    for action in record.iter() {
                        action_handler.redo(action)?;
                    }
                },
            }

            match self.mode {
                HistoryMode::Undo => {
                    history.set_record_state(record, RecordState::Undone)?;
                },
                HistoryMode::Redo => {
                    history.set_record_state(record, RecordState::Redone)?;
                },
            }

            println!("Done.");
        }

        Ok(())
    }
}
