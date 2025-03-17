use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::ActionMode;
use tfmttools_fs::{
    ActionHandler, PathIterator, PathIteratorOptions, get_file_checksum,
    get_longest_common_prefix,
};
use tfmttools_history_core::{History, HistoryError};
use tracing::trace;

use super::RenameContext;
use crate::options::ConfirmMode;
use crate::term::current_dir_utf8;
use crate::ui::{ConfirmationPrompt, PreviewList};

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
    let common_prefix = get_longest_common_prefix(
        &applied_actions
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

    if let Some(common_path) = common_prefix {
        let remaining_paths =
            get_remaining_files_and_directories(context, &common_path)?;

        let (files, folders): (Vec<_>, Vec<_>) =
            remaining_paths.into_iter().partition(|p| p.is_file());

        let has_unknown_files =
            files.iter().any(|path| !file_is_safe_to_delete(path));

        let run_id = context.app_options().run_id();

        if has_unknown_files {
            println!("Found {} remaining files.", files.len());

            preview_files_to_delete(context, &files)?;

            let rename_actions = files
                .into_iter()
                .map(|path| {
                    create_rename_action(
                        context,
                        &common_path,
                        path.clone(),
                        run_id,
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            let confirmation =
                matches!(
                    context.app_options().confirm_mode(),
                    ConfirmMode::NoConfirm
                ) || ConfirmationPrompt::new("Delete these remaining files?")
                    .prompt()?;

            if !confirmation {
                println!("Skipping clean-up.");
                return Ok(());
            }

            applied_actions.extend(move_files(context, rename_actions)?);

            println!("Deleted.");
        } else if !files.is_empty() {
            let rename_actions = files
                .iter()
                .map(|path| {
                    create_rename_action(
                        context,
                        &common_path,
                        path.clone(),
                        run_id,
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            println!("Deleted the following files:");
            preview_files_to_delete(context, &files)?;

            applied_actions.extend(move_files(context, rename_actions)?);
        }

        if !folders.is_empty() {
            applied_actions.extend(remove_directories(context, folders)?);
            println!("Removed empty folders.");
        }
    } else {
        println!(
            "Unable to determine remaining files and folders, skipping clean-up."
        );
    }
    Ok(())
}

fn get_remaining_files_and_directories(
    context: &RenameContext,
    common_path: &Utf8Path,
) -> Result<Vec<Utf8PathBuf>> {
    let options = PathIteratorOptions::with_depth(
        common_path,
        context.rename_options().recursion_depth(),
    );

    let remaining = PathIterator::new(&options)
        // .rev()
        .collect::<TFMTResult<Vec<_>>>()?;

    Ok(remaining)
}

fn file_is_safe_to_delete(path: &Utf8Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext))
}

fn create_rename_action(
    context: &RenameContext,
    common_path: &Utf8Path,
    path: Utf8PathBuf,
    run_id: &str,
) -> Result<RenameAction> {
    let checksum = get_file_checksum(&path)?;

    let relative_path = path.strip_prefix(common_path).expect(
        "common_path is based on this file too, should always have prefix.",
    );

    let path_concat = relative_path.components().join("_");
    let target_name = format!("{path_concat}_{checksum}");

    let rename_action = RenameAction::new(
        path,
        context.rename_options().bin_directory().join(run_id).join(target_name),
    );

    trace!("Created rename action: {rename_action:?}");

    Ok(rename_action)
}

fn preview_files_to_delete(
    context: &RenameContext,
    paths: &[Utf8PathBuf],
) -> Result<()> {
    let working_directory = current_dir_utf8()?;

    let items = PreviewList::new(
        paths
            .iter()
            .map(|path| super::strip_path_prefix(path, &working_directory)),
        context.app_options().preview_list_size(),
    )
    .into_string()?;

    println!("{items}");

    Ok(())
}

fn move_files(
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>> {
    super::move_files_iter(context, rename_actions).collect()
}

fn remove_directories(
    context: &RenameContext,
    directories: Vec<Utf8PathBuf>,
) -> Result<Vec<Action>> {
    let handler = ActionHandler::new(context.fs_handler(), false);

    directories
        .into_iter()
        .rev()
        .map(|path| {
            Ok(handler.apply(
                Action::RemoveDir(path),
                context.rename_options().move_mode(),
            )?)
        })
        .flatten_ok()
        .collect()
}

fn store_history(
    context: &RenameContext,
    history: &mut impl History<Action, ActionRecordMetadata>,
    actions: Vec<Action>,
    metadata: ActionRecordMetadata,
) -> Result<()> {
    if matches!(context.app_options().action_mode(), ActionMode::DryRun) {
        Ok(())
    } else {
        history.push(actions, metadata)?;

        let result = history.save();

        if matches!(result, Err(HistoryError::SaveErrorWithBackup { .. })) {
            eprintln!("{}", result.unwrap_err());
            Ok(())
        } else {
            result?;
            println!(
                "Saved run #{} to history.",
                context.app_options().run_id()
            );
            Ok(())
        }
    }
}
