mod cleanup;
mod confirmation;
mod discovery;
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

use crate::cli::{RenameArgs, TFMTOptions};

pub(crate) struct RenamePlan {
    pub(crate) actions: Vec<RenameAction>,
    pub(crate) unchanged_files: Vec<Utf8File>,
    pub(crate) metadata: ActionRecordMetadata,
    pub(crate) warnings: Vec<Warning>,
}

pub(crate) enum RenameDecision {
    Confirmed {
        actions: Vec<Action>,
        unchanged_files: Vec<Utf8File>,
        metadata: ActionRecordMetadata,
    },
    NothingToRename,
    Aborted,
}

pub fn rename(
    app_options: &TFMTOptions,
    rename_args: RenameArgs,
) -> Result<()> {
    RenameSession::from_args(app_options, rename_args)?.run()
}
