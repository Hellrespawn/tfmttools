use std::hash::Hash;
use std::sync::LazyLock;

use adler::Adler32;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_fs::{
    ActionHandler, PathIterator, PathIteratorOptions, get_longest_common_prefix,
};
use tfmttools_history_core::{History, HistoryError};
use tracing::debug;

use super::RenameContext;
use crate::ui::{ConfirmationPrompt, ItemName, PreviewList};

const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "gif", "bmp"];

pub(crate) fn clean_up(
    context: &RenameContext,
    history: &mut impl History<Action, ActionRecordMetadata>,
    mut applied_actions: Vec<Action>,
    metadata: ActionRecordMetadata,
) -> Result<()> {
    handle_remaining_files(context, &mut applied_actions)?;

    store_history(context, history, applied_actions, metadata)?;

    Ok(())
}

fn handle_remaining_files(
    context: &RenameContext,
    applied_actions: &mut Vec<Action>,
) -> Result<()> {
    let remaining_paths = get_remaining_files_and_directories(applied_actions)?;

    if let Some(remaining_paths) = remaining_paths {
        let (files, folders): (Vec<_>, Vec<_>) =
            remaining_paths.into_iter().partition(|p| p.is_file());

        let (known_files, other_files): (Vec<_>, Vec<_>) =
            files.into_iter().partition(|path| file_is_safe_to_delete(path));

        let run_id = "abcdef";

        if !other_files.is_empty() {
            preview_files_to_delete(
                &other_files,
                &context.app_paths.working_directory()?,
            )?;

            let rename_actions = other_files
                .into_iter()
                .map(|path| create_rename_action(context, path, run_id))
                .collect::<Result<Vec<_>>>()?;

            let confirmation =
                ConfirmationPrompt::new("Delete these remaining files?")
                    .prompt()?;

            if !confirmation {
                println!("Skipping clean-up.");
                return Ok(());
            }

            applied_actions.extend(move_files(context, rename_actions)?);
        }

        if !known_files.is_empty() {
            println!("Deleting the following files:");
            preview_files_to_delete(
                &known_files,
                &context.app_paths.working_directory()?,
            )?;

            let rename_actions = known_files
                .into_iter()
                .map(|path| create_rename_action(context, path, run_id))
                .collect::<Result<Vec<_>>>()?;

            applied_actions.extend(move_files(context, rename_actions)?);
        }

        if !folders.is_empty() {
            println!("Removing empty folders");
            applied_actions.extend(remove_directories(context, folders)?);
        }
    } else {
        println!(
            "Unable to determine remaining files and folders, skipping clean-up."
        );
    }
    Ok(())
}

fn get_remaining_files_and_directories(
    actions: &[Action],
) -> Result<Option<Vec<Utf8PathBuf>>> {
    let common_prefix = get_longest_common_prefix(
        &actions
            .iter()
            .filter_map(|action| {
                if let Action::MoveFile(rename_action)
                | Action::CopyFile(rename_action) = action
                {
                    Some(rename_action.source())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>(),
    );

    debug!("Common prefix of path: {:?}", common_prefix);
    if let Some(common_path) = common_prefix {
        let options = PathIteratorOptions::new(&common_path);

        let remaining = PathIterator::new(&options)
            // .rev()
            .collect::<TFMTResult<Vec<_>>>()?;

        Ok(Some(remaining))
    } else {
        Ok(None)
    }
}

fn file_is_safe_to_delete(path: &Utf8Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext))
}

fn create_rename_action(
    context: &RenameContext,
    path: Utf8PathBuf,
    run_id: &str,
) -> Result<RenameAction> {
    let filename =
        path.file_name().ok_or(eyre!("source should have a filename"))?;

    let checksum = get_checksum(&path);

    let target_name = format!("{filename}_{checksum}");

    Ok(RenameAction::new(
        path,
        context.app_paths.bin_directory().join(run_id).join(target_name),
    ))
}

fn get_checksum(path: &Utf8Path) -> String {
    let mut adler = Adler32::new();

    path.hash(&mut adler);

    format!("{:X}", adler.checksum())
}

fn preview_files_to_delete(
    paths: &[Utf8PathBuf],
    working_directory: &Utf8Path,
) -> Result<()> {
    todo!()
}

fn move_files(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>> {
    todo!()
}

fn remove_directories(
    context: &RenameContext,
    directories: Vec<Utf8PathBuf>,
) -> Result<Vec<Action>> {
    todo!()
}

fn store_history(
    context: &RenameContext,
    history: &mut impl History<Action, ActionRecordMetadata>,
    actions: Vec<Action>,
    metadata: ActionRecordMetadata,
) -> Result<()> {
    if context.misc_options.dry_run {
        Ok(())
    } else {
        history.push(actions, metadata)?;

        let result = history.save();

        if matches!(result, Err(HistoryError::SaveErrorWithBackup { .. })) {
            eprintln!("{}", result.unwrap_err());
            Ok(())
        } else {
            result?;
            Ok(())
        }
    }
}
