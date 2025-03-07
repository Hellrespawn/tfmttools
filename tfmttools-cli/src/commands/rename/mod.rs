mod apply;
mod cleanup;
mod context;
mod setup;

use camino::Utf8Path;
use color_eyre::Result;
pub use context::RenameContext;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_fs::ActionHandler;

use crate::history::load_history;

pub fn rename(context: &RenameContext) -> Result<()> {
    let (mut history, load_history_result) =
        load_history(&context.app_options().history_file_path())?;

    let (rename_actions, metadata) =
        setup::create_actions(context, &mut history, load_history_result)?;

    let applied_actions = apply::apply_actions(context, rename_actions)?;

    if let Some(applied_actions) = applied_actions {
        cleanup::clean_up(context, &mut history, applied_actions, metadata)?;
    }

    Ok(())
}

fn move_files_iter(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> impl Iterator<Item = Result<Action>> {
    let initial_actions = RenameAction::create_actions(rename_actions);

    initial_actions
        .into_iter()
        .map(|action| {
            let handler = ActionHandler::new(context.fs_handler(), false);

            Ok(handler.apply(action, context.rename_options().move_mode())?)
        })
        .flatten_ok()
}

fn strip_path_prefix(path: &Utf8Path, prefix: &Utf8Path) -> String {
    let path = path.strip_prefix(prefix).unwrap_or(path);

    if path.is_relative() {
        format!(".{}{path}", std::path::MAIN_SEPARATOR)
    } else {
        format!("{path}")
    }
}
