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

struct MoveFilesIter<'ah> {
    into_iter: <Vec<Action> as IntoIterator>::IntoIter,
    action_handler: ActionHandler<'ah>,
}

impl<'ah> MoveFilesIter<'ah> {
    fn new(
        into_iter: <Vec<Action> as IntoIterator>::IntoIter,
        action_handler: ActionHandler<'ah>,
    ) -> Self {
        Self { into_iter, action_handler }
    }
}

impl Iterator for MoveFilesIter<'_> {
    type Item = Result<Vec<Action>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.into_iter
            .next()
            .map(|action| Ok(self.action_handler.apply(action)?))
    }
}

fn move_files_iter(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> impl Iterator<Item = Result<Action>> {
    let initial_actions = RenameAction::create_actions(rename_actions);

    let handler = ActionHandler::new(context.fs_handler())
        .move_mode(context.rename_options().move_mode());

    MoveFilesIter::new(initial_actions.into_iter(), handler).flatten_ok()
}

fn strip_path_prefix(path: &Utf8Path, prefix: &Utf8Path) -> String {
    let path = path.strip_prefix(prefix).unwrap_or(path);

    if path.is_relative() {
        format!(".{}{path}", std::path::MAIN_SEPARATOR)
    } else {
        format!("{path}")
    }
}
