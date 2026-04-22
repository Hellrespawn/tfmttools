use tfmttools_core::action::{CaseInsensitivePathSet, RenameAction};

pub(super) struct RenameCycleDetector<'a> {
    rename_actions: &'a [RenameAction],
}

impl<'a> RenameCycleDetector<'a> {
    pub(super) fn new(rename_actions: &'a [RenameAction]) -> Self {
        Self { rename_actions }
    }

    pub(super) fn needs_temporary_staging(&self) -> bool {
        let mut source_paths = CaseInsensitivePathSet::new();

        for action in self.rename_actions {
            source_paths.insert(action.source());
        }

        self.rename_actions
            .iter()
            .any(|action| source_paths.contains(action.target()))
    }
}
