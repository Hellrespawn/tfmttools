use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::util::Utf8PathExt;

use super::rename_cycles::needs_temporary_staging;
use super::rename_staging::StagedRenamePlanner;

pub(super) struct RenamePlanner {
    rename_actions: Vec<RenameAction>,
}

impl RenamePlanner {
    pub(super) fn new(rename_actions: Vec<RenameAction>) -> Self {
        Self { rename_actions }
    }

    pub(super) fn plan(self) -> Vec<Action> {
        let make_dir_actions =
            RenameAction::intermediate_directories(&self.rename_actions)
                .into_iter()
                .filter(|dir| !dir.as_path().exists())
                .map(|dir| Action::MakeDir(dir.into_path_buf()));

        let move_actions = self.plan_move_actions();

        make_dir_actions.chain(move_actions).collect()
    }

    fn plan_move_actions(self) -> Vec<Action> {
        if needs_temporary_staging(&self.rename_actions) {
            StagedRenamePlanner::new(&self.rename_actions)
                .plan(self.rename_actions)
        } else {
            self.rename_actions
                .iter()
                .map(Action::move_from_rename_action)
                .collect()
        }
    }
}
