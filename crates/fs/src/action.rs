mod executor;
mod handler;
mod rename_cycles;
mod rename_planner;
mod rename_staging;

pub use executor::ActionExecutor;
pub use handler::ActionHandler;
use tfmttools_core::action::{Action, RenameAction};

enum PlannedAction {
    Action(Action),
    Rename(RenameAction),
}
