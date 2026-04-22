use camino::Utf8PathBuf;
use tfmttools_core::action::{Action, CaseInsensitivePathSet, RenameAction};
use tfmttools_core::util::{Utf8File, Utf8PathExt};

use super::PlannedAction;

pub(super) struct StagedRenamePlanner {
    reserved_paths: CaseInsensitivePathSet,
}

impl StagedRenamePlanner {
    pub(super) fn new(rename_actions: &[RenameAction]) -> Self {
        let mut reserved_paths = CaseInsensitivePathSet::new();

        for action in rename_actions {
            reserved_paths.insert(action.source());
            reserved_paths.insert(action.target());
        }

        Self { reserved_paths }
    }

    pub(super) fn plan(
        mut self,
        rename_actions: Vec<RenameAction>,
    ) -> Vec<PlannedAction> {
        let staged_actions = rename_actions
            .into_iter()
            .enumerate()
            .map(|(index, action)| {
                let temporary_path =
                    self.temporary_path_for(action.source(), index);

                (action, temporary_path)
            })
            .collect::<Vec<_>>();

        let stage_sources = staged_actions
            .iter()
            .map(|(action, temporary_path)| {
                PlannedAction::Action(Action::MoveFile {
                    source: action.source().to_owned().into_path_buf(),
                    target: temporary_path.clone(),
                })
            })
            .collect::<Vec<_>>();

        let move_to_targets =
            staged_actions.into_iter().map(|(action, temporary_path)| {
                PlannedAction::Action(Action::MoveFile {
                    source: temporary_path,
                    target: action.target().to_owned().into_path_buf(),
                })
            });

        stage_sources.into_iter().chain(move_to_targets).collect()
    }

    fn temporary_path_for(
        &mut self,
        source: &Utf8File,
        index: usize,
    ) -> Utf8PathBuf {
        let parent = source.parent();
        let process_id = std::process::id();

        for attempt in 0.. {
            let candidate = parent
                .as_path()
                .join(format!(".tfmt-{process_id}-{index}-{attempt}"));

            if !candidate.exists() && self.reserved_paths.insert(&candidate) {
                return candidate;
            }
        }

        unreachable!("unbounded temporary path generation should return");
    }
}
