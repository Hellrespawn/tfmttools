use color_eyre::Result;
use itertools::Itertools;
use tfmttools_core::util::Utf8File;
use tfmttools_fs::ActionExecutor;
use tracing::trace;

use super::{RenameExecutionResult, RenamePlan, RenameSession};

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
            let executor = ActionExecutor::new(session.fs_handler())
                .move_mode(session.rename_options().move_mode());
            let actions = executor.plan_actions(plan.actions)?;
            Ok(RenameExecutionResult::Applied {
                actions,
                unchanged_files: plan.unchanged_files,
                metadata: plan.metadata,
            })
        } else {
            Ok(RenameExecutionResult::Aborted)
        }
    }
}
