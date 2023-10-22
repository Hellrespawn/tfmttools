mod validate;

use crate::cli::config::{DRY_RUN_PREFIX, HISTORY_NAME};
use crate::cli::{ui, Config};
use crate::file::AudioFile;
use crate::template::Template;
use color_eyre::Result;
use file_history::{Action, History, HistoryError};
use indicatif::ProgressIterator;
use std::fs;
use std::path::{Path, PathBuf};
use validate::validate_actions;

pub(crate) fn rename(
    config: &Config,
    name: &str,
    arguments: Vec<String>,
) -> Result<()> {
    config.create_dir(config.config_dir())?;

    let mut history = History::load(config.config_dir(), HISTORY_NAME)?;

    let template = config.get_template(name)?.with_arguments(arguments);

    let files = gather_files(config)?;

    let actions = create_actions(config, &template, &files)?;
    let actions = filter_unchanged_destinations(actions);

    if actions.is_empty() {
        println!("There are no audio files to rename.");
        Ok(())
    } else {
        validate_actions(config, &actions)?;

        perform_actions(config, &mut history, actions)
    }
}
fn gather_files(config: &Config) -> Result<Vec<AudioFile>> {
    let spinner = ui::AudioFileSpinner::new(
        "audio files",
        "total files",
        "Gathering files...",
    )?;

    let paths = Config::search_path(
        config.current_dir(),
        config.recursion_depth(),
        &|p| {
            p.extension().map_or(false, |extension| {
                for supported_extension in AudioFile::SUPPORTED_EXTENSIONS {
                    if extension == supported_extension {
                        return true;
                    }
                }

                false
            })
        },
        Some(&spinner),
    );

    spinner.finish("Gathered files.");

    paths.iter().map(|p| AudioFile::new(p)).collect()
}

fn create_actions(
    config: &Config,
    template: &Template,
    files: &[AudioFile],
) -> Result<Vec<Action>> {
    let bar = ui::create_progressbar(
        files.len() as u64,
        "Determining output paths...",
        "Determined output paths",
        false,
    )?;

    let actions: Result<Vec<Action>> = files
        .iter()
        .progress_with(bar)
        .map(|audiofile| action_from_file(config, template, audiofile))
        .collect();

    println!();
    println!();

    actions
}

fn get_common_path(paths: &[&Path]) -> PathBuf {
    debug_assert!(!paths.is_empty());

    let mut iter = paths.iter();

    // We have already returned if no files were found, so this unwrap
    // should be safe.
    let mut common_path = iter.next().unwrap().to_path_buf();

    for path in iter {
        let mut new_common_path = PathBuf::new();

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

fn action_from_file(
    config: &Config,
    template: &Template,
    audiofile: &AudioFile,
) -> Result<Action> {
    let string = template.render(audiofile)?;

    let string = normalize_separators(&string);

    let target =
        create_target_path_from_string(config, &string, audiofile.extension());

    let action = Action::mv(audiofile.path(), target);

    #[cfg(debug_assertions)]
    crate::debug::delay();

    Ok(action)
}

fn create_target_path_from_string(
    config: &Config,
    string: &str,
    extension: &str,
) -> PathBuf {
    let target_path = PathBuf::from(format!("{string}.{extension}"));

    // If target_path has an absolute path, join will clobber the current_dir,
    // so this is always safe.
    config.current_dir().join(target_path)
}

// TODO? Refactor this into the `create_actions` progress bar?
fn filter_unchanged_destinations(actions: Vec<Action>) -> Vec<Action> {
    actions
        .into_iter()
        .filter(|action| {
            let (source, target) = action.get_src_tgt_unchecked();
            source != target
        })
        .collect()
}

fn perform_actions(
    config: &Config,
    history: &mut History,
    actions: Vec<Action>,
) -> Result<()> {
    ui::print_actions_preview(config, &actions, &std::env::current_dir()?);

    let common_path = get_common_path(
        &actions
            .iter()
            .map(|a| a.get_src_tgt_unchecked().0)
            .collect::<Vec<_>>(),
    );

    move_files(config.dry_run(), history, actions)?;

    clean_up_source_dirs(config, history, &common_path)?;

    history.save()?;

    Ok(())
}

fn move_files(
    dry_run: bool,
    history: &mut History,
    actions: Vec<Action>,
) -> Result<()> {
    let bar = ui::create_progressbar(
        actions.len() as u64,
        "Moving files...",
        "Moved files",
        dry_run,
    )?;

    for action in actions.into_iter().progress_with(bar) {
        let (_, target) = action.get_src_tgt_unchecked();
        // Actions target are all files, and always have a parent.

        debug_assert!(target.parent().is_some());

        create_dir(dry_run, history, target.parent().unwrap())?;
        if !dry_run {
            history.apply(action)?;
        }

        #[cfg(debug_assertions)]
        crate::debug::delay();
    }

    Ok(())
}

fn create_dir(dry_run: bool, history: &mut History, path: &Path) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        create_dir(dry_run, history, parent)?;
    }

    let action = Action::mkdir(path);

    if !dry_run {
        history.apply(action)?;
    }

    Ok(())
}

fn clean_up_source_dirs(
    config: &Config,
    history: &mut History,
    common_path: &Path,
) -> Result<()> {
    let dirs = gather_dirs(common_path, config.recursion_depth());

    let actions: Vec<Action> = dirs.into_iter().map(Action::rmdir).collect();

    if !config.dry_run() {
        for action in actions {
            remove_dir(history, action)?;
        }
    }

    let prefix: &str = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

    println!("{prefix}Removed leftover folders.");

    Ok(())
}

fn gather_dirs(path: &Path, depth: usize) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if depth == 0 {
        return dirs;
    }

    for entry in fs::read_dir(path).into_iter().flatten().flatten() {
        let dir = entry.path();
        if dir.is_dir() {
            dirs.extend(gather_dirs(&dir, depth - 1));
            dirs.push(dir);
        }
    }

    dirs
}

fn remove_dir(history: &mut History, action: Action) -> Result<()> {
    let result = history.apply(action);

    if let Err(err) = result {
        let mut is_expected_error = false;

        if let HistoryError::IO(io_error) = &err {
            if let Some(error_code) = io_error.raw_os_error() {
                #[cfg(windows)]
                // https://docs.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
                // 145: Directory not empty
                let expected_code = 145;

                // https://nuetzlich.net/errno.html
                // 39: Directory not empty
                #[cfg(unix)]
                let expected_code = 39;

                if error_code == expected_code {
                    is_expected_error = true;
                }
            }
        }

        if !is_expected_error {
            return Err(err.into());
        }
    }

    Ok(())
}

fn normalize_separators(string: &str) -> String {
    string
        .split(['\\', '/'])
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use color_eyre::Result;

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
        let result = fs::remove_dir(test_folder);

        if let Err(err) = result {
            if let Some(error_code) = err.raw_os_error() {
                assert_eq!(error_code, expected_code);
                Ok(())
            } else {
                Err(err.into())
            }
        } else {
            Ok(())
        }
    }
}
