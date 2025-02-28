use camino::Utf8Path;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::{Action, RenameAction, validate_rename_actions};
use tfmttools_fs::ActionHandler;

use super::RenameContext;
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList, ProgressBar};

pub fn apply_actions(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Option<Vec<Action>>> {
    let rename_actions =
        RenameAction::filter_unchanged_destinations(rename_actions);

    if rename_actions.is_empty() {
        println!("There are no audio files to rename.");
        Ok(None)
    } else {
        validate_rename_action_errors(&rename_actions)?;

        let confirmation = context.misc_options.no_confirm
            || confirm_rename_actions(context, &rename_actions)?;

        if confirmation {
            Ok(Some(move_files(context, rename_actions)?))
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

fn confirm_rename_actions(
    context: &RenameContext,
    rename_actions: &[RenameAction],
) -> Result<bool> {
    let cwd = context.app_paths.working_directory()?;

    preview_rename_actions(rename_actions, &cwd)?;

    let confirmation_prompt = ConfirmationPrompt::new("Move files?");

    confirmation_prompt.prompt()
}

fn preview_rename_actions(
    rename_actions: &[RenameAction],
    working_directory: &Utf8Path,
) -> Result<()> {
    const LEADING_LINES: usize = 3;
    const TRAILING_LINES: usize = 3;

    let iter = rename_actions.iter().map(|rename_action| {
        let path = rename_action
            .target()
            .strip_prefix(working_directory)
            .unwrap_or(rename_action.target());

        if path.is_relative() {
            format!(".{}{path}", std::path::MAIN_SEPARATOR)
        } else {
            format!("{path}")
        }
    });

    let preview_list = PreviewList::new(iter)
        .leading(LEADING_LINES)
        .trailing(TRAILING_LINES)
        .item_name(ItemName::simple("file"));

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
    );

    let initial_actions = RenameAction::create_actions(rename_actions);

    let mut applied_actions = Vec::new();

    let handler = ActionHandler::new(
        context.fs_handler,
        context.misc_options.always_copy,
        false,
    );

    for action in initial_actions {
        let actions = handler.apply(action)?;

        let is_rename_action = actions
            .iter()
            .any(tfmttools_core::action::Action::is_rename_action);

        applied_actions.extend(actions);

        if is_rename_action {
            bar.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        }
    }

    bar.finish();

    Ok(applied_actions)
}
