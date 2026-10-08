mod binary_patch;
mod executor;
mod handler;
mod rename_cycles;
mod rename_planner;
mod rename_staging;
mod tag_edit;

pub use executor::ActionExecutor;
pub use handler::ActionHandler;
use tfmttools_core::action::{Action, RenameAction};

enum PlannedAction {
    Action(Action),
    Rename(RenameAction),
}

pub use binary_patch::{apply_patch, byte_identity, create_patch_pair};
pub use tag_edit::write_tag_candidate;
