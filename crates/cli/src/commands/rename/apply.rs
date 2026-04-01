use color_eyre::Result;
use color_eyre::eyre::eyre;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction, validate_rename_actions};
use tfmttools_core::util::{Utf8File, Utf8PathExt};
use tracing::trace;

use super::{RenameContext, RenameResult};
use crate::options::ConfirmMode;
use crate::term::current_dir_utf8;
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList, ProgressBar};

pub fn apply_actions(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<RenameResult> {
    let (rename_actions, unchanged_files) =
        RenameAction::separate_unchanged_destinations(rename_actions);

    // Can't apply compiler attribute to macro invocation directly.
    #[allow(unstable_name_collisions)]
    {
        trace!(
            "Unchanged paths:\n{}",
            unchanged_files
                .iter()
                .map(Utf8File::to_string)
                .intersperse("\n".to_owned())
                .collect::<String>()
        );
    }

    if rename_actions.is_empty() {
        Ok(RenameResult::NothingToRename(unchanged_files))
    } else {
        validate_rename_action_errors(&rename_actions)?;

        preview_rename_actions(context, &rename_actions, &unchanged_files)?;

        let confirmation = matches!(
            context.app_options().confirm_mode(),
            ConfirmMode::NoConfirm
        ) || ConfirmationPrompt::new("Move files?")
            .prompt()?;

        if confirmation {
            match move_files(context, rename_actions) {
                Ok(applied_actions) => {
                    Ok(RenameResult::Ok { applied_actions, unchanged_files })
                },
                Err((err, applied_actions)) => {
                    let _ = handle_error_during_move(applied_actions);
                    Err(err)
                },
            }
        } else {
            Ok(RenameResult::Aborted)
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
    unchanged_files: &[Utf8File],
) -> Result<()> {
    let working_directory = current_dir_utf8()?;

    let iter = rename_actions.iter().map(|rename_action| {
        super::strip_path_prefix(
            rename_action.target().as_path(),
            working_directory.as_path(),
        )
    });

    let preview_list =
        PreviewList::new(iter, context.app_options().preview_list_size())
            .with_item_name(ItemName::simple("destination"));

    if !unchanged_files.is_empty() {
        println!("There are {} unchanged files.\n", unchanged_files.len());
    }

    preview_list.print()?;

    Ok(())
}

fn move_files(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>, (color_eyre::Report, Vec<Action>)> {
    let bar = ProgressBar::bar(
        context.app_options().display_mode(),
        "Moving files:",
        "Moved files.",
        rename_actions.len() as u64,
        true,
    );

    let mut applied_actions = Vec::new();

    let iter =
        super::move_files_iter(context, rename_actions).inspect(|result| {
            if let Ok(action) = result {
                if action.is_rename_action() {
                    bar.inc_found();

                    #[cfg(feature = "debug")]
                    crate::debug::delay();
                }
            }
        });

    for action in iter {
        match action {
            Ok(applied_action) => applied_actions.push(applied_action),
            Err(err) => {
                bar.finish();
                return Err((err, applied_actions));
            },
        }
    }

    bar.finish();
    Ok(applied_actions)
}

#[allow(clippy::unnecessary_wraps)]
fn handle_error_during_move(_applied_actions: Vec<Action>) -> Result<()> {
    Ok(())
}
