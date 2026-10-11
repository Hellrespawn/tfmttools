use color_eyre::Result;
use tfmttools_core::history::{History, LoadHistoryResult};
use tfmttools_core::util::Utf8PathExt;
use tfmttools_fs::PathIteratorOptions;
use tracing::info;

use super::{RenameDecision, cleanup, confirmation, planning, preview};
use crate::cli::{RenameArgs, RenameOptions, TFMTOptions};
use crate::history::load_history_for_mode;

pub struct RenameSession<'a> {
    app_options: &'a TFMTOptions,
    rename_options: RenameOptions,
    history: History,
    load_result: LoadHistoryResult,
}

impl<'a> RenameSession<'a> {
    pub fn from_args(
        app_options: &'a TFMTOptions,
        rename_args: RenameArgs,
    ) -> Result<Self> {
        let rename_options =
            RenameOptions::try_from((rename_args, app_options))?;

        let (mut history, load_result) = load_history_for_mode(
            &app_options.history_file_path()?,
            app_options.fs_mode(),
        )?;

        crate::history::execution::admit_command(
            &mut history,
            app_options.fs_mode(),
        )?;

        Ok(Self { app_options, rename_options, history, load_result })
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

        let execution = confirmation::confirm_and_plan(&self, plan)?;
        self.finish(execution)
    }

    fn finish(&mut self, execution: RenameDecision) -> Result<()> {
        match execution {
            RenameDecision::Confirmed {
                actions,
                unchanged_files,
                metadata,
            } => {
                let actions = cleanup::append_cleanup_actions(
                    self,
                    actions,
                    &unchanged_files,
                )?;
                crate::history::execution::execute_recorded(
                    &mut self.history,
                    self.app_options.fs_mode(),
                    actions,
                    metadata,
                )?;
                if !matches!(
                    self.app_options.fs_mode(),
                    tfmttools_core::util::FSMode::DryRun
                ) {
                    println!("Run completed.");
                    println!(
                        "Saved run #{} to history.",
                        self.app_options.run_id()
                    );
                }
            },
            RenameDecision::NothingToRename => {
                let msg = "There are no audio files to rename.";
                println!("{msg}");
                info!("{msg}");
            },
            RenameDecision::Aborted => {
                println!("Aborting!");
            },
        }

        Ok(())
    }
}
