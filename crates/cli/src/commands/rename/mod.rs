mod apply;
mod cleanup;
mod context;
mod session;
mod setup;
mod shared;

use color_eyre::Result;
pub use context::RenameContext;
use session::RenameSession;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::Utf8File;

pub(crate) struct RenamePlan {
    actions: Vec<RenameAction>,
    unchanged_files: Vec<Utf8File>,
    metadata: ActionRecordMetadata,
}

pub(crate) enum ExecutionResult {
    Applied {
        actions: Vec<Action>,
        unchanged_files: Vec<Utf8File>,
        metadata: ActionRecordMetadata,
    },
    NothingToRename(Vec<Utf8File>),
    Aborted,
}

pub fn rename(context: &RenameContext) -> Result<()> {
    RenameSession::load(context)?.run()
}
