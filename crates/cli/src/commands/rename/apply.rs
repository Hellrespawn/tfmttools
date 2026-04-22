use color_eyre::Result;
use color_eyre::eyre::eyre;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction, validate_rename_actions};
use tfmttools_core::util::{Utf8File, Utf8PathExt};
use tfmttools_fs::ActionExecutor;
use tracing::trace;

use super::{RenameExecutionResult, RenameContext, RenamePlan};
use crate::ui::{ItemName, PreviewList, ProgressBar, current_dir_utf8};

pub fn preview(context: &RenameContext, plan: &RenamePlan) -> Result<()> {
    validate_rename_action_errors(&plan.actions)?;
    preview_rename_actions(context, &plan.actions, &plan.unchanged_files)
}

pub fn execute(
    context: &RenameContext,
    plan: RenamePlan,
) -> Result<RenameExecutionResult> {
    // Can't apply compiler attribute to macro invocation directly.
    #[allow(unstable_name_collisions)]
    {
        trace!(
            "Unchanged paths:\n{}",
            plan.unchanged_files
                .iter()
                .map(Utf8File::to_string)
                .intersperse("\n".to_owned())
                .collect::<String>()
        );
    }

    if plan.actions.is_empty() {
        Ok(RenameExecutionResult::NothingToRename(plan.unchanged_files))
    } else {
        let confirmation = super::shared::confirm(context, "Move files?")?;

        if confirmation {
            match move_files(context, plan.actions) {
                Ok(actions) => {
                    Ok(RenameExecutionResult::Applied {
                        actions,
                        unchanged_files: plan.unchanged_files,
                        metadata: plan.metadata,
                    })
                },
                Err((err, applied_actions)) => {
                    let _ = handle_error_during_move(applied_actions);
                    Err(err)
                },
            }
        } else {
            Ok(RenameExecutionResult::Aborted)
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
        super::shared::strip_path_prefix(
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

    let executor = ActionExecutor::new(context.fs_handler())
        .move_mode(context.rename_options().move_mode());

    let iter =
        executor.apply_rename_actions(rename_actions).inspect(|result| {
            if let Ok(action) = result
                && action.is_rename_action()
            {
                bar.inc_found();

                #[cfg(feature = "debug")]
                crate::debug::delay();
            }
        });

    for action in iter {
        match action {
            Ok(applied_action) => applied_actions.push(applied_action),
            Err(err) => {
                bar.finish();
                return Err((err.into(), applied_actions));
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
