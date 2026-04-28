use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecord, ActionRecordMetadata};
use tfmttools_fs::{ActionHandler, FsHandler};
use tfmttools_history::{History, HistoryMode, LoadHistoryResult, RecordState};

use crate::cli::{ConfirmMode, TFMTOptions};
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList, PreviewListSize};

pub fn undo_redo(
    mode: HistoryMode,
    amount: usize,
    app_options: &TFMTOptions,
    fs_handler: &FsHandler,
) -> Result<()> {
    let verb = match mode {
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
            let records = get_records(&history, mode, amount)?;

            let actual = records.len();

            if records.is_empty() {
                println!("There are no runs to {verb}.");
            } else {
                if actual < amount {
                    println!(
                        "Tried to {verb} {amount} runs, but only {actual} can be {verb}ne.",
                        verb = mode.verb()
                    );
                }

                let formatter = HistoryFormatter::new()
                    .with_prefix(HistoryPrefix::Ordered(')'));
                let confirmation = matches!(
                    app_options.confirm_mode(),
                    ConfirmMode::NoConfirm
                ) || confirm_undo_redo(
                    &records,
                    mode,
                    &formatter,
                    app_options.preview_list_size(),
                )?;

                if confirmation {
                    perform_undo_redo_actions(
                        &mut history,
                        records,
                        fs_handler,
                        mode,
                        &formatter,
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
    history: &History<Action, ActionRecordMetadata>,
    mode: HistoryMode,
    amount: usize,
) -> Result<Vec<ActionRecord>> {
    Ok(match mode {
        HistoryMode::Undo => history.get_n_records_to_undo(amount)?,
        HistoryMode::Redo => history.get_n_records_to_redo(amount)?,
    })
}

fn confirm_undo_redo(
    records: &[ActionRecord],
    mode: HistoryMode,
    formatter: &HistoryFormatter,
    preview_list_size: PreviewListSize,
) -> Result<bool> {
    preview_undo_redo(records, formatter, preview_list_size)?;

    let item_name = ItemName::simple("record");

    let amount = records.len();

    let prompt_message = format!(
        "{} {} {}?",
        mode.verb_capitalized(),
        amount,
        item_name.by_amount(amount)
    );

    let confirmation_prompt = ConfirmationPrompt::new(&prompt_message);

    confirmation_prompt.prompt()
}

fn preview_undo_redo(
    records: &[ActionRecord],
    formatter: &HistoryFormatter,
    preview_list_size: PreviewListSize,
) -> Result<()> {
    let iter = records.iter().map(|record| formatter.format_record(record));

    let preview_list = PreviewList::new(iter, preview_list_size)
        .with_item_name(ItemName::simple("record"));

    preview_list.print()?;

    Ok(())
}

fn perform_undo_redo_actions(
    history: &mut History<Action, ActionRecordMetadata>,
    records: Vec<ActionRecord>,
    fs_handler: &FsHandler,
    mode: HistoryMode,
    formatter: &HistoryFormatter,
) -> Result<()> {
    let action_handler = ActionHandler::new(fs_handler);

    for record in records {
        println!(
            "{}ing {}...",
            mode.verb_capitalized(),
            formatter.format_record(&record)
        );

        match mode {
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

        match mode {
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
