use color_eyre::Result;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::warning::Warning;
use tfmttools_history::{History, LoadHistoryResult};

use super::{RenamePlan, RenameSession, discovery, template_resolution};

pub fn create_plan(
    session: &RenameSession,
    history: &History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<RenamePlan> {
    let resolved = template_resolution::resolve_template(
        session,
        history,
        load_history_result,
    )?;

    let mut resolved = resolved;
    let mut warnings: Vec<Warning> = std::mem::take(&mut resolved.warnings);

    let (actions, file_warnings) =
        discovery::create_actions_from_template(session, &resolved)?;

    warnings.extend(file_warnings);

    let (actions, unchanged_files) =
        RenameAction::separate_unchanged_destinations(actions);

    Ok(RenamePlan { actions, unchanged_files, metadata: resolved.metadata, warnings })
}
