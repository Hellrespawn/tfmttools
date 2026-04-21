use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_history::{History, LoadHistoryResult};
use tracing::info;

use super::{
    ExecutionResult, RenameContext, RenamePlan, apply, cleanup, setup,
};
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
        let plan = self.plan_actions()?;

        if !plan.actions.is_empty() {
            self.preview(&plan)?;
        }

        let execution = self.execute(plan)?;
        self.cleanup(execution)
    }

    fn plan_actions(&self) -> Result<RenamePlan> {
        setup::create_plan(self.context, &self.history, self.load_result)
    }

    fn preview(&self, plan: &RenamePlan) -> Result<()> {
        apply::preview(self.context, plan)
    }

    fn execute(&self, plan: RenamePlan) -> Result<ExecutionResult> {
        apply::execute(self.context, plan)
    }

    fn cleanup(&mut self, execution: ExecutionResult) -> Result<()> {
        match execution {
            ExecutionResult::Applied { actions, unchanged_files, metadata } => {
                let actions = cleanup::handle_remaining_files(
                    self.context,
                    actions,
                    &unchanged_files,
                )?;
                self.save_history(actions, metadata)?;
            },
            ExecutionResult::NothingToRename(_unchanged_paths) => {
                let msg = "There are no audio files to rename.";
                println!("{msg}");
                info!("{msg}");
            },
            ExecutionResult::Aborted => {
                println!("Aborting!");
            },
        }

        Ok(())
    }

    fn save_history(
        &mut self,
        actions: Vec<Action>,
        metadata: ActionRecordMetadata,
    ) -> Result<()> {
        cleanup::store_history(
            self.context,
            &mut self.history,
            actions,
            metadata,
        )
    }
}
