mod apply;
mod discovery;
mod finish;
mod planning;
mod preview;
mod session;
mod shared;
mod template_resolution;

use color_eyre::Result;
pub(crate) use session::RenameSession;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::Utf8File;
use tfmttools_core::warning::Warning;
use tfmttools_fs::FsHandler;

use crate::cli::{RenameArgs, TFMTOptions};

pub(crate) struct RenamePlan {
    pub(crate) actions: Vec<RenameAction>,
    pub(crate) unchanged_files: Vec<Utf8File>,
    pub(crate) metadata: ActionRecordMetadata,
    pub(crate) warnings: Vec<Warning>,
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
