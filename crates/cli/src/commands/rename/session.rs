use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_history::{History, LoadHistoryResult};
use tracing::info;

use super::{RenameExecutionResult, RenameContext, apply, finish, setup};
use crate::history::load_history;

pub struct RenameSession<'a> {
    context: &'a RenameContext<'a>,
    history: History<Action, ActionRecordMetadata>,
    load_result: LoadHistoryResult,
}

impl<'a> RenameSession<'a> {
    pub fn load(context: &'a RenameContext<'a>) -> Result<Self> {
        let (history, load_result) =
            load_history(&context.app_options().history_file_path()?)?;

        Ok(Self { context, history, load_result })
    }

    pub fn run(mut self) -> Result<()> {
        let plan =
            setup::create_plan(self.context, &self.history, self.load_result)?;

        if !plan.actions.is_empty() {
            apply::preview(self.context, &plan)?;
        }

        let execution = apply::execute(self.context, plan)?;
        self.finish(execution)
    }

    fn finish(&mut self, execution: RenameExecutionResult) -> Result<()> {
        match execution {
            RenameExecutionResult::Applied { actions, unchanged_files, metadata } => {
                let actions = finish::handle_remaining_files(
                    self.context,
                    actions,
                    &unchanged_files,
                )?;
                finish::store_history(
                    self.context,
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
