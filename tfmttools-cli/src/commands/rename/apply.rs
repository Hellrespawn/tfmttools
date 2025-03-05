use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::{Action, RenameAction, validate_rename_actions};
use tracing::info;

use super::RenameContext;
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList, ProgressBar};

pub fn apply_actions(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Option<Vec<Action>>> {
    let rename_actions =
        RenameAction::filter_unchanged_destinations(rename_actions);

    if rename_actions.is_empty() {
        let msg = "There are no audio files to rename.";
        println!("{msg}");
        info!("{msg}");
        Ok(None)
    } else {
        validate_rename_action_errors(&rename_actions)?;

        preview_rename_actions(context, &rename_actions)?;

        let confirmation = context.misc_options().no_confirm()
            || ConfirmationPrompt::new("Move files?").prompt()?;

        if confirmation {
            let applied_actions = move_files(context, rename_actions)?;

            Ok(Some(applied_actions))
        } else {
            println!("Aborting!");
            Ok(None)
        }
    }
}

fn validate_rename_action_errors(
    rename_actions: &[RenameAction],
) -> Result<()> {
    let validation_errors = validate_rename_actions(rename_actions);

    if validation_errors.is_empty() {
        Ok(())
    } else {
        let error_string = validation_errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        Err(eyre!("Had validation errors:\n{error_string}"))
    }
}

fn preview_rename_actions(
    context: &RenameContext,
    rename_actions: &[RenameAction],
) -> Result<()> {
    let working_directory = context.app_paths().working_directory()?;

    let iter = rename_actions.iter().map(|rename_action| {
        super::strip_path_prefix(rename_action.target(), &working_directory)
    });

    let preview_list =
        PreviewList::new(iter, context.misc_options().preview_list_size())
            .with_item_name(ItemName::simple("destination"));

    preview_list.print()?;

    Ok(())
}

fn move_files(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>> {
    let bar = ProgressBar::bar(
        "Moving files:",
        "Moved files.",
        rename_actions.len() as u64,
        true,
    );

    let applied_actions = super::move_files_iter(context, rename_actions)
        .inspect(|result| {
            if let Ok(action) = result {
                if action.is_rename_action() {
                    bar.inc_found();

                    #[cfg(feature = "debug")]
                    crate::debug::delay();
                }
            }
        })
        .collect();

    bar.finish();

    applied_actions
}
