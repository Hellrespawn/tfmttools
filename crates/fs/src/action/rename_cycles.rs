use tfmttools_core::action::{CaseInsensitivePathSet, RenameAction};

pub(super) fn needs_temporary_staging(actions: &[RenameAction]) -> bool {
    let mut source_paths = CaseInsensitivePathSet::new();
    for action in actions {
        source_paths.insert(action.source());
    }
    actions.iter().any(|action| source_paths.contains(action.target()))
}
