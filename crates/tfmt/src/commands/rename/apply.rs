use color_eyre::Result;
use itertools::Itertools;
use tfmttools_core::action::Action;
use tfmttools_core::util::Utf8File;
use tfmttools_fs::ActionExecutor;
use tracing::trace;

use super::{RenameExecutionResult, RenamePlan, RenameSession};
use crate::ui::ProgressBar;

pub fn execute(
    session: &RenameSession,
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
        let confirmation = super::shared::confirm(session, "Move files?")?;

        if confirmation {
            match move_files(session, plan.actions) {
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

fn move_files(
    session: &RenameSession,
    rename_actions: Vec<tfmttools_core::action::RenameAction>,
) -> Result<Vec<Action>, (color_eyre::Report, Vec<Action>)> {
    let bar = ProgressBar::bar(
        session.app_options().display_mode(),
        "Moving files:",
        "Moved files.",
        rename_actions.len() as u64,
        true,
    );

    let mut applied_actions = Vec::new();

    let executor = ActionExecutor::new(session.fs_handler())
        .move_mode(session.rename_options().move_mode());

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
