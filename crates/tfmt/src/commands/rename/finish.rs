use std::collections::HashSet;

use camino::Utf8Path;
use color_eyre::Result;
use itertools::Itertools;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::{FSMode, Utf8Directory, Utf8File, Utf8PathExt};
use tfmttools_fs::{
    ActionExecutor, PathIterator, PathIteratorOptions, get_file_checksum,
    get_longest_common_prefix,
};
use tfmttools_history::{History, HistoryError};
use tracing::{debug, info, trace};

use super::RenameSession;
use crate::cli::TFMTOptions;
use crate::ui::{PreviewList, current_dir_utf8};

const AUTO_DELETE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "gif", "bmp"];

pub(crate) fn handle_remaining_files(
    session: &RenameSession,
    mut applied_actions: Vec<Action>,
    unchanged_files: &[Utf8File],
) -> Result<Vec<Action>> {
    let common_prefix = get_longest_common_prefix(
        applied_actions.iter().filter_map(Action::source),
    );

    if let Some(common_prefix) = common_prefix {
        debug!("Common prefix of renamed files: {}", common_prefix);

        let protected_paths = protected_cleanup_paths(
            applied_actions.as_slice(),
            unchanged_files,
        );
        let remaining_items = discover_remaining_items(
            session,
            &common_prefix,
            &protected_paths,
        )?;

        // Can't apply compiler attribute to macro invocation directly.
        #[allow(unstable_name_collisions)]
        {
            trace!(
                "Remaining files:\n{}",
                remaining_items
                    .files
                    .iter()
                    .map(Utf8File::to_string)
                    .intersperse("\n".to_owned())
                    .collect::<String>()
            );

            trace!(
                "Remaining folders:\n{}",
                remaining_items
                    .directories
                    .iter()
                    .map(Utf8Directory::to_string)
                    .intersperse("\n".to_owned())
                    .collect::<String>()
            );
        }

        let cleanup_plan = plan_cleanup(remaining_items);

        if let Some(cleanup_plan) = cleanup_plan {
            if !prepare_cleanup(session, &cleanup_plan)? {
                return Ok(applied_actions);
            }

            let confirmed_delete = cleanup_plan.requires_confirmation
                && !cleanup_plan.files_to_delete.is_empty();
            let removed_directories =
                !cleanup_plan.directories_to_remove.is_empty();
            let cleanup_actions =
                execute_cleanup(session, &common_prefix, cleanup_plan)?;

            if confirmed_delete {
                println!("Deleted.");
            }

            if removed_directories {
                println!("Removed empty folders.");
            }

            applied_actions.extend(cleanup_actions);
        }
    } else {
        println!(
            "Unable to determine remaining files and folders, skipping clean-up."
        );
    }
    Ok(applied_actions)
}

struct CleanupPlan {
    files_to_delete: Vec<Utf8File>,
    directories_to_remove: Vec<Utf8Directory>,
    requires_confirmation: bool,
}

struct RemainingItems {
    files: Vec<Utf8File>,
    directories: Vec<Utf8Directory>,
}

fn discover_remaining_items(
    session: &RenameSession,
    common_prefix: &Utf8Path,
    protected_paths: &HashSet<camino::Utf8PathBuf>,
) -> Result<RemainingItems> {
    let options = PathIteratorOptions::with_depth(
        common_prefix,
        session.rename_options().recursion_depth(),
    );

    let remaining = PathIterator::new(&options)
        .filter_ok(|path| !protected_paths.contains(path))
        .collect::<TFMTResult<Vec<_>>>()?;

    let (files, folders): (Vec<_>, Vec<_>) =
        remaining.into_iter().partition(|p| p.is_file());

    Ok(RemainingItems {
        files: files.into_iter().map(Utf8File::new_unchecked).collect(),
        directories: folders
            .into_iter()
            .map(Utf8Directory::new_unchecked)
            .collect(),
    })
}

fn protected_cleanup_paths(
    applied_actions: &[Action],
    unchanged_files: &[Utf8File],
) -> HashSet<camino::Utf8PathBuf> {
    unchanged_files
        .iter()
        .map(|f| f.to_owned().into_path_buf())
        .chain(
            applied_actions
                .iter()
                .filter(|action| action.is_rename_action())
                .map(|action| action.target().to_owned()),
        )
        .collect()
}

fn plan_cleanup(remaining_items: RemainingItems) -> Option<CleanupPlan> {
    if remaining_items.files.is_empty()
        && remaining_items.directories.is_empty()
    {
        return None;
    }

    let requires_confirmation =
        remaining_items.files.iter().any(|path| !file_is_safe_to_delete(path));

    Some(CleanupPlan {
        files_to_delete: remaining_items.files,
        directories_to_remove: remaining_items.directories,
        requires_confirmation,
    })
}

fn file_is_safe_to_delete(path: &Utf8File) -> bool {
    path.extension().is_some_and(|ext| AUTO_DELETE_EXTENSIONS.contains(&ext))
}

fn prepare_cleanup(
    session: &RenameSession,
    cleanup_plan: &CleanupPlan,
) -> Result<bool> {
    if cleanup_plan.requires_confirmation {
        info!("Has files that are not safe to delete.");

        println!(
            "Found {} remaining files.",
            cleanup_plan.files_to_delete.len()
        );

        preview_files_to_delete(session, &cleanup_plan.files_to_delete)?;

        if !confirm_cleanup(session)? {
            println!("Skipping clean-up.");
            return Ok(false);
        }
    } else if !cleanup_plan.files_to_delete.is_empty() {
        info!("Has only files that are safe to delete.");

        println!("Deleted the following files:");
        preview_files_to_delete(session, &cleanup_plan.files_to_delete)?;
    }

    Ok(true)
}

fn create_rename_action(
    session: &RenameSession,
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
        session
            .rename_options()
            .bin_directory()
            .join(run_id)?
            .join_file(target_name)?,
    );

    trace!("Created rename action: {rename_action:?}");

    Ok(rename_action)
}

fn preview_files_to_delete(
    session: &RenameSession,
    files: &[Utf8File],
) -> Result<()> {
    let working_directory = current_dir_utf8()?;

    let items = PreviewList::new(
        files.iter().map(|file| {
            super::shared::strip_path_prefix(
                file.as_path(),
                working_directory.as_path(),
            )
        }),
        session.app_options().preview_list_size(),
    )
    .into_string()?;

    println!("{items}");

    Ok(())
}

fn confirm_cleanup(session: &RenameSession) -> Result<bool> {
    super::shared::confirm(session, "Delete these remaining files?")
}

fn execute_cleanup(
    session: &RenameSession,
    common_prefix: &Utf8Path,
    cleanup_plan: CleanupPlan,
) -> Result<Vec<Action>> {
    let rename_actions = create_rename_actions(
        session,
        common_prefix,
        cleanup_plan.files_to_delete,
    )?;
    let mut actions = move_files(session, rename_actions)?;

    actions.extend(remove_directories(
        session,
        cleanup_plan.directories_to_remove,
    )?);

    Ok(actions)
}

fn create_rename_actions(
    session: &RenameSession,
    common_prefix: &Utf8Path,
    files: Vec<Utf8File>,
) -> Result<Vec<RenameAction>> {
    files
        .into_iter()
        .map(|file| {
            create_rename_action(
                session,
                common_prefix,
                file,
                session.app_options().run_id(),
            )
        })
        .collect()
}

fn move_files(
    session: &RenameSession,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>> {
    let executor = ActionExecutor::new(session.fs_handler())
        .move_mode(session.rename_options().move_mode());

    Ok(executor
        .apply_rename_actions(rename_actions)
        .collect::<TFMTResult<_>>()?)
}

fn remove_directories(
    session: &RenameSession,
    directories: Vec<Utf8Directory>,
) -> Result<Vec<Action>> {
    Ok(ActionExecutor::new(session.fs_handler())
        .remove_directories(directories)?)
}

pub(crate) fn store_history(
    app_options: &TFMTOptions,
    history: &mut History<Action, ActionRecordMetadata>,
    actions: Vec<Action>,
    metadata: ActionRecordMetadata,
) -> Result<()> {
    if matches!(app_options.fs_mode(), FSMode::DryRun) {
        Ok(())
    } else {
        history.push(actions, metadata)?;

        match history.save() {
            Err(err @ HistoryError::SaveErrorWithBackup { .. }) => {
                eprintln!("{err}");
            },
            result => {
                result?;
                println!("Saved run #{} to history.", app_options.run_id());
            },
        }

        Ok(())
    }
}
