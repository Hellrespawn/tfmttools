mod apply;
mod cleanup;
mod context;
mod setup;

use camino::Utf8Path;
use color_eyre::Result;
pub use context::RenameContext;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::util::Utf8File;
use tfmttools_fs::ActionHandler;
use tracing::info;

use crate::history::load_history;

pub enum RenameResult {
    Ok { applied_actions: Vec<Action>, unchanged_files: Vec<Utf8File> },
    NothingToRename(Vec<Utf8File>),
    Aborted,
}

pub fn rename(context: &RenameContext) -> Result<()> {
    let (mut history, load_history_result) =
        load_history(&context.app_options().history_file_path()?)?;

    let (rename_actions, metadata) =
        setup::create_actions(context, &mut history, load_history_result)?;

    let applied_actions = apply::apply_actions(context, rename_actions)?;

    match applied_actions {
        RenameResult::Ok { applied_actions, unchanged_files } => {
            cleanup::clean_up(
                context,
                &mut history,
                applied_actions,
                &unchanged_files,
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
    let make_dir_actions = RenameAction::get_make_dir_actions(&rename_actions);

    let make_dir_iter = make_dir_actions.into_iter().map(|action| {
        let handler = ActionHandler::new(context.fs_handler())
            .move_mode(context.rename_options().move_mode());

        handler.apply(&action)?;

        Ok(action)
    });

    let move_files_iter = rename_actions
        .into_iter()
        .map(|rename_action| {
            let handler = ActionHandler::new(context.fs_handler())
                .move_mode(context.rename_options().move_mode());

            let applied_actions = handler.rename(&rename_action)?;

            Ok(applied_actions)
        })
        .flatten_ok();

    make_dir_iter.chain(move_files_iter)
}

fn strip_path_prefix(path: &Utf8Path, prefix: &Utf8Path) -> String {
    let path = path.strip_prefix(prefix).unwrap_or(path);

    if path.is_relative() {
        format!(".{}{path}", std::path::MAIN_SEPARATOR)
    } else {
        format!("{path}")
    }
}
