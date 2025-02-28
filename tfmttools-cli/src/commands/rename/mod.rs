mod apply;
mod cleanup;
mod context;
mod setup;

use color_eyre::Result;
pub use context::{RenameContext, RenameMiscOptions, RenameTemplateOptions};

use crate::history::load_history;

pub fn rename(context: &RenameContext) -> Result<()> {
    let (mut history, load_history_result) =
        load_history(&context.app_paths.history_file())?;

    let (rename_actions, metadata) =
        setup::create_actions(context, &mut history, load_history_result)?;

    let applied_actions = apply::apply_actions(context, rename_actions)?;

    if let Some(applied_actions) = applied_actions {
        cleanup::clean_up(context, &mut history, applied_actions, metadata)?;
    }

    Ok(())
}
