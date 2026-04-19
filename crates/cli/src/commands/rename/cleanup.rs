use camino::Utf8Path;
use color_eyre::Result;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::{FSMode, Utf8Directory, Utf8File, Utf8PathExt};
use tfmttools_fs::{
    ActionHandler, PathIterator, PathIteratorOptions, get_file_checksum,
    get_longest_common_prefix,
};
use tfmttools_history::{History, HistoryError};
use tracing::{debug, info, trace};

use super::RenameContext;
use crate::cli::ConfirmMode;
use crate::ui::{ConfirmationPrompt, PreviewList, current_dir_utf8};

const AUTO_DELETE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "gif", "bmp"];

pub(crate) fn clean_up(
    context: &RenameContext,
    history: &mut History<Action, ActionRecordMetadata>,
    applied_actions: Vec<Action>,
    unchanged_files: &[Utf8File],
    metadata: ActionRecordMetadata,
) -> Result<()> {
    let applied_actions =
        handle_remaining_files(context, applied_actions, unchanged_files)?;

    store_history(context, history, applied_actions, metadata)?;

    Ok(())
}

fn handle_remaining_files(
    context: &RenameContext,
    mut applied_actions: Vec<Action>,
    unchanged_files: &[Utf8File],
) -> Result<Vec<Action>> {
    let common_prefix = get_longest_common_prefix(
        &applied_actions
            .iter()
            .filter_map(|action| {
                if let Action::MoveFile { source, .. }
                | Action::CopyFile { source, .. } = action
                {
                    Some(source.as_path())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>(),
    );

    if let Some(common_prefix) = common_prefix {
        debug!("Common prefix of renamed files: {}", common_prefix);

        let (files, folders) = get_remaining_files_and_directories(
            context,
            &common_prefix,
            unchanged_files,
        )?;

        // Can't apply compiler attribute to macro invocation directly.
        #[allow(unstable_name_collisions)]
        {
            trace!(
                "Remaining files:\n{}",
                files
                    .iter()
                    .map(Utf8File::to_string)
                    .intersperse("\n".to_owned())
                    .collect::<String>()
            );

            trace!(
                "Remaining folders:\n{}",
                folders
                    .iter()
                    .map(Utf8Directory::to_string)
                    .intersperse("\n".to_owned())
                    .collect::<String>()
            );
        }

        let has_unknown_files =
            files.iter().any(|path| !file_is_safe_to_delete(path));

        let run_id = context.app_options().run_id();

        if has_unknown_files {
            info!("Has files that are not safe to delete.");

            println!("Found {} remaining files.", files.len());

            preview_files_to_delete(context, &files)?;

            let rename_actions = files
                .into_iter()
                .map(|path| {
                    create_rename_action(context, &common_prefix, path, run_id)
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
                return Ok(applied_actions);
            }

            applied_actions.extend(move_files(context, rename_actions)?);

            println!("Deleted.");
        } else if !files.is_empty() {
            info!("Has only files that are safe to delete.");

            println!("Deleted the following files:");
            preview_files_to_delete(context, &files)?;

            let rename_actions = files
                .into_iter()
                .map(|file| {
                    create_rename_action(context, &common_prefix, file, run_id)
                })
                .collect::<Result<Vec<_>>>()?;

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
    Ok(applied_actions)
}

fn get_remaining_files_and_directories(
    context: &RenameContext,
    common_prefix: &Utf8Path,
    unchanged_files: &[Utf8File],
) -> Result<(Vec<Utf8File>, Vec<Utf8Directory>)> {
    let options = PathIteratorOptions::with_depth(
        common_prefix,
        context.rename_options().recursion_depth(),
    );

    let unchanged_paths = unchanged_files
        .iter()
        .map(|f| f.to_owned().into_path_buf())
        .collect::<Vec<_>>();

    let remaining = PathIterator::new(&options)
        .filter_ok(|path| !unchanged_paths.contains(path))
        .collect::<TFMTResult<Vec<_>>>()?;

    let (files, folders): (Vec<_>, Vec<_>) =
        remaining.into_iter().partition(|p| p.is_file());

    Ok((
        files.into_iter().map(Utf8File::new_unchecked).collect(),
        folders.into_iter().map(Utf8Directory::new_unchecked).collect(),
    ))
}

fn file_is_safe_to_delete(path: &Utf8File) -> bool {
    path.extension().is_some_and(|ext| AUTO_DELETE_EXTENSIONS.contains(&ext))
}

fn create_rename_action(
    context: &RenameContext,
    common_prefix: &Utf8Path,
    file: Utf8File,
    run_id: &str,
) -> Result<RenameAction> {
    let checksum = get_file_checksum(&file)?;

    let relative_path = file.as_path().strip_prefix(common_prefix).expect(
        "common_prefix is based on this file too, should always have prefix.",
    );

    let path_concat = relative_path.components().join("_");
    let target_name = format!("{path_concat}_{checksum}");

    let rename_action = RenameAction::new(
        file,
        context
            .rename_options()
            .bin_directory()
            .join(run_id)?
            .join_file(target_name)?,
    );

    trace!("Created rename action: {rename_action:?}");

    Ok(rename_action)
}

fn preview_files_to_delete(
    context: &RenameContext,
    paths: &[Utf8File],
) -> Result<()> {
    let working_directory = current_dir_utf8()?;

    let items = PreviewList::new(
        paths.iter().map(|path| {
            super::strip_path_prefix(
                path.as_path(),
                working_directory.as_path(),
            )
        }),
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
    directories: Vec<Utf8Directory>,
) -> Result<Vec<Action>> {
    let handler = ActionHandler::new(context.fs_handler());

    directories
        .into_iter()
        .rev()
        .map(|dir| {
            let action = Action::RemoveDir(dir.into_path_buf());
            handler.apply(&action)?;
            Ok(action)
        })
        .collect()
}

fn store_history(
    context: &RenameContext,
    history: &mut History<Action, ActionRecordMetadata>,
    actions: Vec<Action>,
    metadata: ActionRecordMetadata,
) -> Result<()> {
    if matches!(context.app_options().fs_mode(), FSMode::DryRun) {
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
