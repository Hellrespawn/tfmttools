use tfmttools_core::action::RenameAction;

use super::PlannedAction;
use super::rename_cycles::RenameCycleDetector;
use super::rename_staging::StagedRenamePlanner;

pub(super) struct RenamePlanner {
    rename_actions: Vec<RenameAction>,
}

impl RenamePlanner {
    pub(super) fn new(rename_actions: Vec<RenameAction>) -> Self {
        Self { rename_actions }
    }

    pub(super) fn plan(self) -> Vec<PlannedAction> {
        let make_dir_actions =
            RenameAction::get_make_dir_actions(&self.rename_actions)
                .into_iter()
                .map(PlannedAction::Action);

        let move_actions = self.plan_move_actions();

        make_dir_actions.chain(move_actions).collect()
    }

    fn plan_move_actions(self) -> Vec<PlannedAction> {
        if RenameCycleDetector::new(&self.rename_actions)
            .needs_temporary_staging()
        {
            StagedRenamePlanner::new(&self.rename_actions)
                .plan(self.rename_actions)
        } else {
            self.rename_actions.into_iter().map(PlannedAction::Rename).collect()
        }
    }
}
