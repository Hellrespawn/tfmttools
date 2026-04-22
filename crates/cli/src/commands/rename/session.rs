use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::Utf8PathExt;
use tfmttools_fs::{FsHandler, PathIteratorOptions};
use tfmttools_history::{History, LoadHistoryResult};
use tracing::info;

use super::{RenameExecutionResult, apply, finish, setup};
use crate::cli::{RenameArgs, RenameOptions, TFMTOptions};
use crate::history::load_history;

pub struct RenameSession<'a> {
    fs_handler: &'a FsHandler,
    app_options: &'a TFMTOptions,
    rename_options: RenameOptions,
    history: History<Action, ActionRecordMetadata>,
    load_result: LoadHistoryResult,
}

impl<'a> RenameSession<'a> {
    pub fn from_args(
        fs_handler: &'a FsHandler,
        app_options: &'a TFMTOptions,
        rename_args: RenameArgs,
    ) -> Result<Self> {
        let rename_options =
            RenameOptions::try_from((rename_args, app_options))?;

        let (history, load_result) =
            load_history(&app_options.history_file_path()?)?;

        Ok(Self {
            fs_handler,
            app_options,
            rename_options,
            history,
            load_result,
        })
    }

    pub fn fs_handler(&self) -> &FsHandler {
        self.fs_handler
    }

    pub fn path_iterator_options(&self) -> PathIteratorOptions<'_> {
        PathIteratorOptions::with_depth(
            self.rename_options.input_directory().as_path(),
            self.rename_options.recursion_depth(),
        )
    }

    pub fn app_options(&self) -> &TFMTOptions {
        self.app_options
    }

    pub fn rename_options(&self) -> &RenameOptions {
        &self.rename_options
    }

    pub fn run(mut self) -> Result<()> {
        let plan = setup::create_plan(&self, &self.history, self.load_result)?;

        if !plan.actions.is_empty() {
            apply::preview(&self, &plan)?;
        }

        let execution = apply::execute(&self, plan)?;
        self.finish(execution)
    }

    fn finish(&mut self, execution: RenameExecutionResult) -> Result<()> {
        match execution {
            RenameExecutionResult::Applied {
                actions,
                unchanged_files,
                metadata,
            } => {
                let actions = finish::handle_remaining_files(
                    self,
                    actions,
                    &unchanged_files,
                )?;
                finish::store_history(
                    self.app_options,
                    &mut self.history,
                    actions,
                    metadata,
                )?;
            },
            RenameExecutionResult::NothingToRename(_unchanged_paths) => {
                let msg = "There are no audio files to rename.";
                println!("{msg}");
                info!("{msg}");
            },
            RenameExecutionResult::Aborted => {
                println!("Aborting!");
            },
        }

        Ok(())
    }
}
