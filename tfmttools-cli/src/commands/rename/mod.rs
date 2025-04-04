mod apply;
mod cleanup;
mod context;
mod setup;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
pub use context::RenameContext;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_fs::ActionHandler;
use tracing::info;

use crate::history::load_history;

pub enum RenameResult {
    Ok { applied_actions: Vec<Action>, unchanged_paths: Vec<Utf8PathBuf> },
    NothingToRename(Vec<Utf8PathBuf>),
    Aborted,
}

pub fn rename(context: &RenameContext) -> Result<()> {
    let (mut history, load_history_result) =
        load_history(&context.app_options().history_file_path()?)?;

    let (rename_actions, metadata) =
        setup::create_actions(context, &mut history, load_history_result)?;

    let applied_actions = apply::apply_actions(context, rename_actions)?;

    match applied_actions {
        RenameResult::Ok { applied_actions, unchanged_paths } => {
            cleanup::clean_up(
                context,
                &mut history,
                applied_actions,
                &unchanged_paths,
                metadata,
            )?;
        },
        RenameResult::NothingToRename(_unchanged_paths) => {
            let msg = "There are no audio files to rename.";
            println!("{msg}");
            info!("{msg}");
        },
        RenameResult::Aborted => {
            println!("Aborting!");
        },
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
