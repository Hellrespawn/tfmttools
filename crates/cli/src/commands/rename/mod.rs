mod apply;
mod finish;
mod session;
mod setup;
mod shared;

use color_eyre::Result;
pub(crate) use session::RenameSession;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::Utf8File;
use tfmttools_fs::FsHandler;

use crate::cli::{RenameArgs, TFMTOptions};

pub(crate) struct RenamePlan {
    actions: Vec<RenameAction>,
    unchanged_files: Vec<Utf8File>,
    metadata: ActionRecordMetadata,
}

pub(crate) enum RenameExecutionResult {
    Applied {
        actions: Vec<Action>,
        unchanged_files: Vec<Utf8File>,
        metadata: ActionRecordMetadata,
    },
    NothingToRename(Vec<Utf8File>),
    Aborted,
}

pub fn rename(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    rename_args: RenameArgs,
) -> Result<()> {
    RenameSession::from_args(fs_handler, app_options, rename_args)?.run()
}
