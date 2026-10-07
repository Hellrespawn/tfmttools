use color_eyre::Result;
use tfmttools_core::action::RenameAction;
use tfmttools_core::history::{History, LoadHistoryResult};

use super::{RenamePlan, RenameSession, discovery, template_resolution};

pub fn create_plan(
    session: &RenameSession,
    history: &History,
    load_history_result: LoadHistoryResult,
) -> Result<RenamePlan> {
    let resolved = template_resolution::resolve_template(
        session,
        history,
        load_history_result,
    )?;

    let (actions, warnings) =
        discovery::create_actions_from_template(session, &resolved)?;

    let (actions, unchanged_files) =
        RenameAction::separate_unchanged_destinations(actions);

    Ok(RenamePlan {
        actions,
        unchanged_files,
        metadata: resolved.metadata,
        warnings,
    })
}
