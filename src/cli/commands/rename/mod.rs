mod validate;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use indicatif::ProgressIterator;
use validate::validate_move_actions;

use crate::action::Move;
use crate::audiofile::AudioFile;
use crate::cli::ui::table::Table;
use crate::cli::ui::{self, create_spinner};
use crate::config::{Config, DRY_RUN_PREFIX};
use crate::fs::{self, PathIterator};
use crate::template::Template;

pub(crate) fn rename(
    config: &Config,
    name: &str,
    arguments: Vec<String>,
) -> Result<()> {
    config.create_dir(config.directory())?;

    let template = config.get_template(name)?.with_arguments(arguments);

    let files = gather_files(config)?;

    let move_actions = create_move_actions(config, &template, &files)?;
    let move_actions = filter_unchanged_destinations(move_actions);

    if move_actions.is_empty() {
        println!("There are no audio files to rename.");
        Ok(())
    } else {
        validate_move_actions(config, &move_actions)?;

        perform_move_actions(config, move_actions)
    }
}
fn gather_files(config: &Config) -> Result<Vec<AudioFile>> {
    let spinner = create_spinner(
        "audio files",
        "total files",
        "Gathering files...",
        "Gathered files.",
    )?;

    let paths: Vec<Utf8PathBuf> =
        PathIterator::new(config.current_dir(), Some(config.recursion_depth()))
            .flatten()
            .filter(|path| AudioFile::path_predicate(path))
            .progress_with(spinner)
            .collect();

    paths.iter().map(|path| AudioFile::new(path)).collect()
}

fn create_move_actions(
    config: &Config,
    template: &Template,
    files: &[AudioFile],
) -> Result<Vec<Move>> {
    let bar = ui::create_progressbar(
        files.len() as u64,
        "Determining output paths...",
        "Determined output paths",
        config.dry_run(),
    )?;

    let move_actions: Result<Vec<Move>> = files
        .iter()
        .map(|audiofile| {
            Ok(Move::new(
                audiofile.path().to_owned(),
                audiofile.create_target_path(template, config.current_dir())?,
            ))
        })
        .progress_with(bar)
        .collect();

    println!();
    println!();

    move_actions
}

fn get_common_path(paths: &[&Utf8Path]) -> Utf8PathBuf {
    debug_assert!(!paths.is_empty());

    let mut iter = paths.iter();

    // We have already returned if no files were found, so this unwrap
    // should be safe.
    let mut common_path = iter.next().unwrap().to_path_buf();

    for path in iter {
        let mut new_common_path = Utf8PathBuf::new();

        for (left, right) in path.components().zip(common_path.components()) {
            if left == right {
                new_common_path.push(left);
            } else {
                break;
            }
        }
        common_path = new_common_path;
    }

    common_path
}

// TODO? Refactor this into the `create_changes` progress bar?
fn filter_unchanged_destinations(move_actions: Vec<Move>) -> Vec<Move> {
    move_actions.into_iter().filter(Move::source_differs_from_target).collect()
}

fn perform_move_actions(
    config: &Config,
    move_actions: Vec<Move>,
) -> Result<()> {
    print_move_actions_preview(config, &move_actions);

    let common_path = get_common_path(
        &move_actions.iter().map(Move::source).collect::<Vec<_>>(),
    );

    move_files(config.dry_run(), move_actions)?;

    fs::remove_empty_subdirectories(
        config.dry_run(),
        &common_path,
        config.recursion_depth(),
    )?;

    let prefix: &str = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

    println!("{prefix}Removed leftover folders.");

    Ok(())
}

fn move_files(dry_run: bool, move_actions: Vec<Move>) -> Result<()> {
    let bar = ui::create_progressbar(
        move_actions.len() as u64,
        "Moving files...",
        "Moved files",
        dry_run,
    )?;

    for move_action in move_actions.into_iter().progress_with(bar) {
        let target = move_action.target();

        // Actions target are all files, and always have a parent.
        debug_assert!(target.parent().is_some());

        create_dir(dry_run, target.parent().unwrap())?;

        if !dry_run {
            fs::copy_or_move_file(move_action.source(), move_action.target())?;
        }

        #[cfg(debug_assertions)]
        crate::debug::delay();
    }

    Ok(())
}

fn create_dir(dry_run: bool, path: &Utf8Path) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        create_dir(dry_run, parent)?;
    }

    if !dry_run {
        fs::create_dir(path)?;
    }

    Ok(())
}

pub(crate) fn print_move_actions_preview(
    config: &Config,
    move_actions: &[Move],
) {
    let length = move_actions.len();

    let step = std::cmp::max(move_actions.len() / config.preview_amount(), 1);

    let slice = move_actions
        .iter()
        .step_by(step)
        .map(Move::target)
        .map(|path| path.strip_prefix(config.current_dir()).unwrap_or(path))
        .collect::<Vec<_>>();

    let mut table = Table::new();

    table.set_heading(if slice.len() <= config.preview_amount() {
        format!("Previewing {} files", slice.len())
    } else {
        format!("Previewing {} of {} files", slice.len(), length)
    });

    for path in slice {
        table.push_path(path);
    }

    println!("{table}");
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use color_eyre::Result;
    use fs_err as fs;

    #[test]
    fn test_remove_dir_error_codes() -> Result<()> {
        let tempdir = TempDir::new()?;

        let test_folder = tempdir.path().join("test_folder");
        let test_file = test_folder.join("test.file");

        #[cfg(windows)]
        let expected_code = 145;

        #[cfg(unix)]
        let expected_code = 39;

        fs::create_dir(&test_folder)?;
        fs::write(test_file, "")?;

        if let Err(err) = fs::remove_dir(test_folder) {
            if let Some(error_code) =
                std::io::Error::last_os_error().raw_os_error()
            {
                assert_eq!(
                    error_code, expected_code,
                    "Expected code {expected_code}, got {error_code}",
                );
                Ok(())
            } else {
                panic!("Received unexpected error:\n{err}");
            }
        } else {
            Ok(())
        }
    }
}
