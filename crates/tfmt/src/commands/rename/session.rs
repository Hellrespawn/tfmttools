use color_eyre::Result;
use tfmttools_core::history::{History, LoadHistoryResult};
use tfmttools_core::util::Utf8PathExt;
use tfmttools_fs::{FsHandler, PathIteratorOptions};
use tracing::info;

use super::{RenameExecutionResult, apply, finish, planning, preview};
use crate::cli::{RenameArgs, RenameOptions, TFMTOptions};
use crate::history::load_history_for_mode;

pub struct RenameSession<'a> {
    fs_handler: &'a FsHandler,
    app_options: &'a TFMTOptions,
    rename_options: RenameOptions,
    history: History,
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

        let (mut history, load_result) = load_history_for_mode(
            &app_options.history_file_path()?,
            app_options.fs_mode(),
        )?;

        crate::history::execution::recover_pending(&mut history, fs_handler)?;

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
        let plan =
            planning::create_plan(&self, &self.history, self.load_result)?;

        if !plan.actions.is_empty() {
            preview::preview(&self, &plan)?;
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
                crate::history::execution::execute_recorded(
                    &mut self.history,
                    self.fs_handler,
                    actions,
                    metadata,
                )?;
                if !matches!(
                    self.app_options.fs_mode(),
                    tfmttools_core::util::FSMode::DryRun
                ) {
                    println!(
                        "Saved run #{} to history.",
                        self.app_options.run_id()
                    );
                }
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
