mod binary_patch;
mod executor;
mod file_switch;
mod handler;
mod recorded_execution;
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
pub use file_switch::{
    cleanup_completed_artifacts, cleanup_prepared, discard_prepared,
    install_prepared, prepare_tag_edit, prepare_tag_replay, recover_prepared,
};
pub use recorded_execution::prepare_action;
pub use tag_edit::write_tag_candidate;
