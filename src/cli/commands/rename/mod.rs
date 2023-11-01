mod validate;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use file_history::{Change, History, HistoryError};
use indicatif::ProgressIterator;
use validate::validate_changes;

use crate::cli::ui::{self, create_spinner};
use crate::config::{Config, DRY_RUN_PREFIX, HISTORY_NAME};
use crate::audiofile::AudioFile;
use crate::fs::PathIterator;
use crate::template::Template;

pub(crate) fn rename(
    config: &Config,
    name: &str,
    arguments: Vec<String>,
) -> Result<()> {
    config.create_dir(config.directory())?;

    let mut history = History::load(config.directory(), HISTORY_NAME)?;

    let template = config.get_template(name)?.with_arguments(arguments);

    let files = gather_files(config)?;

    let changes = create_changes(config, &template, &files)?;
    let changes = filter_unchanged_destinations(changes);

    if changes.is_empty() {
        println!("There are no audio files to rename.");
        Ok(())
    } else {
        validate_changes(config, &changes)?;

        perform_changes(config, &mut history, changes)
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
        PathIterator::recursive(config.current_dir(), config.recursion_depth())
            .flatten()
            .filter(|path| AudioFile::path_predicate(path))
            .progress_with(spinner)
            .collect();

    paths.iter().map(|path| AudioFile::new(path)).collect()
}

fn create_changes(
    config: &Config,
    template: &Template,
    files: &[AudioFile],
) -> Result<Vec<Change>> {
    let bar = ui::create_progressbar(
        files.len() as u64,
        "Determining output paths...",
        "Determined output paths",
        config.dry_run(),
    )?;

    let changes: Result<Vec<Change>> = files
        .iter()
        .progress_with(bar)
        .map(|audiofile| change_from_file(config, template, audiofile))
        .collect();

    println!();
    println!();

    changes
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

fn change_from_file(
    config: &Config,
    template: &Template,
    audiofile: &AudioFile,
) -> Result<Change> {
    let string = template.render(audiofile)?;

    let string = normalize_separators(&string);

    let target =
        create_target_path_from_string(config, &string, audiofile.extension());

    let change = Change::mv(audiofile.path(), target);

    #[cfg(debug_assertions)]
    crate::debug::delay();

    Ok(change)
}

fn create_target_path_from_string(
    config: &Config,
    string: &str,
    extension: &str,
) -> Utf8PathBuf {
    let target_path = Utf8PathBuf::from(format!("{string}.{extension}"));

    // If target_path has an absolute path, join will clobber the current_dir,
    // so this is always safe.
    config.current_dir().join(target_path)
}

// TODO? Refactor this into the `create_changes` progress bar?
fn filter_unchanged_destinations(changes: Vec<Change>) -> Vec<Change> {
    changes
        .into_iter()
        .filter(|change| {
            let source =
                change.source().expect("Can only validate collisions on move.");

            source != change.target()
        })
        .collect()
}

fn perform_changes(
    config: &Config,
    history: &mut History,
    changes: Vec<Change>,
) -> Result<()> {
    ui::print_changes_preview(config, &changes, &std::env::current_dir()?);

    let common_path = get_common_path(
        &changes
            .iter()
            .map(|change| {
                change.source().expect("Can only validate collisions on move.")
            })
            .collect::<Vec<_>>(),
    );

    move_files(config.dry_run(), history, changes)?;

    clean_up_source_dirs(config, history, &common_path)?;

    history.save()?;

    Ok(())
}

fn move_files(
    dry_run: bool,
    history: &mut History,
    changes: Vec<Change>,
) -> Result<()> {
    let bar = ui::create_progressbar(
        changes.len() as u64,
        "Moving files...",
        "Moved files",
        dry_run,
    )?;

    for change in changes.into_iter().progress_with(bar) {
        let target = change.target();

        // Actions target are all files, and always have a parent.
        debug_assert!(target.parent().is_some());

        create_dir(dry_run, history, target.parent().unwrap())?;
        if !dry_run {
            history.apply(change)?;
        }

        #[cfg(debug_assertions)]
        crate::debug::delay();
    }

    Ok(())
}

fn create_dir(
    dry_run: bool,
    history: &mut History,
    path: &Utf8Path,
) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        create_dir(dry_run, history, parent)?;
    }

    let change = Change::mkdir(path);

    if !dry_run {
        history.apply(change)?;
    }

    Ok(())
}

fn clean_up_source_dirs(
    config: &Config,
    history: &mut History,
    common_path: &Utf8Path,
) -> Result<()> {
    let dirs = gather_dirs(common_path, config.recursion_depth());

    let changes: Vec<Change> = dirs.into_iter().map(Change::rmdir).collect();

    if !config.dry_run() {
        for change in changes {
            remove_dir(history, change)?;
        }
    }

    let prefix: &str = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

    println!("{prefix}Removed leftover folders.");

    Ok(())
}

fn gather_dirs(path: &Utf8Path, depth: usize) -> Vec<Utf8PathBuf> {
    let mut dirs = Vec::new();

    if depth == 0 {
        return dirs;
    }

    for entry in path.read_dir_utf8().into_iter().flatten().flatten() {
        let dir = entry.path();
        if dir.is_dir() {
            dirs.extend(gather_dirs(dir, depth - 1));
            dirs.push(dir.to_owned());
        }
    }

    dirs
}

fn remove_dir(history: &mut History, change: Change) -> Result<()> {
    let result = history.apply(change);

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
