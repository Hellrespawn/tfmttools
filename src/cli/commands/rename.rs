use camino::Utf8PathBuf;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use history::{History, SaveHistoryResult};

use super::super::config::{Config, DRY_RUN_PREFIX};
use super::Command;
use crate::action::{Action, Move};
use crate::audiofile::AudioFile;
use crate::cli::preview::{preview, PreviewData};
use crate::cli::ui::{ProgressBar, ProgressBarOptions};
use crate::cli::util::PathOrString;
use crate::fs::{self, PathIterator, RemoveDirResult};
use crate::template::{Template, Templates};

#[derive(Debug)]
pub struct Rename {
    input_directory: Utf8PathBuf,
    template_directory: Utf8PathBuf,

    force: bool,

    recursion_depth: usize,

    template: PathOrString,
    arguments: Vec<String>,
}

impl Rename {
    pub fn new(
        input_directory: Utf8PathBuf,
        template_directory: Utf8PathBuf,
        force: bool,
        recursion_depth: usize,
        template: PathOrString,
        arguments: Vec<String>,
    ) -> Self {
        Self {
            input_directory,
            template_directory,
            force,
            recursion_depth,
            template,
            arguments,
        }
    }
}

impl Command for Rename {
    fn run(&self, config: &Config) -> Result<()> {
        InnerRename { options: self, config }.rename()
    }
}

struct InnerRename<'ir> {
    options: &'ir Rename,
    config: &'ir Config,
}

impl<'a> InnerRename<'a> {
    pub fn rename(&self) -> Result<()> {
        let templates = match &self.options.template {
            PathOrString::Path(path, string) => {
                Templates::read_filename(path, string)
            },
            PathOrString::String(_) => {
                Templates::read_directory(&self.options.template_directory)
            },
        }?;

        let template_name = self.options.template.as_str();

        let template = templates
            .get_template(template_name, self.options.arguments.clone())
            .ok_or(eyre!("Unable to find template: {}", template_name))?;

        let files = self.gather_files()?;

        let move_actions = self.create_move_actions(&template, &files)?;
        let move_actions = Move::filter_unchanged_destinations(move_actions);

        if move_actions.is_empty() {
            println!("There are no audio files to rename.");
            Ok(())
        } else {
            Self::validate_move_actions(&move_actions)?;

            let cwd = self.config.working_directory()?;

            let app_data = PreviewData::rename(
                template.name(),
                &self.options.arguments,
                &move_actions,
                &cwd,
            );

            let confirmation = self.options.force || preview(&app_data)?;

            if confirmation {
                let actions = self.perform_move_actions(move_actions)?;

                self.store_history(actions)?;
            } else {
                println!("Aborting!");
            }

            Ok(())
        }
    }

    fn gather_files(&self) -> Result<Vec<AudioFile>> {
        let options = ProgressBarOptions::spinner(
            self.config.dry_run(),
            "audio",
            "total",
            "Gathering files...",
            "Gathered files.",
        )?;

        let spinner = ProgressBar::new(options);

        let file_paths = PathIterator::new(
            &self.options.input_directory,
            Some(self.options.recursion_depth),
        )
        .flatten()
        .inspect(|_| spinner.inc_total())
        .filter(|path| AudioFile::path_predicate(path))
        .inspect(|_| {
            spinner.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .map(|path| AudioFile::new(&path))
        .collect::<Result<Vec<_>>>();

        spinner.finish();

        file_paths
    }

    fn create_move_actions(
        &self,
        template: &Template,
        files: &[AudioFile],
    ) -> Result<Vec<Move>> {
        let options = ProgressBarOptions::bar(
            self.config.dry_run(),
            "Determining output paths:",
            "Determined output paths.",
        )?;

        let bar = ProgressBar::with_length(options, files.len() as u64);

        let move_actions: Result<Vec<Move>> = files
            .iter()
            .map(|audiofile| {
                Ok(Move::new(
                    audiofile.path().to_owned(),
                    audiofile.construct_target_path(
                        template,
                        &self.options.input_directory,
                    )?,
                ))
            })
            .inspect(|_| {
                bar.inc_found();

                #[cfg(feature = "debug")]
                crate::debug::delay();
            })
            .collect();

        bar.finish();

        println!();
        println!();

        move_actions
    }

    fn validate_move_actions(move_actions: &[Move]) -> Result<()> {
        let validation_errors =
            crate::action::validate_move_actions(move_actions);

        if validation_errors.is_empty() {
            Ok(())
        } else {
            Err(eyre!("Had validation errors:"))
        }
    }

    fn perform_move_actions(
        &self,
        move_actions: Vec<Move>,
    ) -> Result<Vec<Action>> {
        let common_prefix = fs::get_longest_common_prefix(
            &move_actions.iter().map(Move::source).collect::<Vec<_>>(),
        );

        let mut actions = self.move_files(move_actions)?;

        if self.config.dry_run() {
            print!("{DRY_RUN_PREFIX}");
        }

        if let Some(common_path) = common_prefix {
            let removed = fs::remove_empty_subdirectories(
                self.config.dry_run(),
                &common_path,
                self.options.recursion_depth,
            )?;

            actions.extend(
                removed
                    .iter()
                    .filter(|(_, r)| matches!(r, RemoveDirResult::Removed))
                    .map(|(p, _)| Action::RemoveDir(p.clone())),
            );

            println!("Removed leftover folders.");
        } else {
            println!("Unable to remove leftover folders.");
        }

        Ok(actions)
    }

    fn move_files(&self, move_actions: Vec<Move>) -> Result<Vec<Action>> {
        let options = ProgressBarOptions::bar(
            self.config.dry_run(),
            "Moving files:",
            "Moved files.",
        )?;

        let bar = ProgressBar::with_length(options, move_actions.len() as u64);

        let actions = Move::create_actions(move_actions);

        for action in &actions {
            action.apply(self.config.dry_run())?;

            if action.is_move() {
                bar.inc_found();

                #[cfg(feature = "debug")]
                crate::debug::delay();
            }
        }

        bar.finish();

        Ok(actions)
    }

    fn store_history(&self, actions: Vec<Action>) -> Result<()> {
        if !self.config.dry_run() {
            let mut history = History::load(&self.config.history_file())?;

            history.push(actions)?;

            if let SaveHistoryResult::Exists(tmp_file) = history.save()? {
                eprintln!(
                    "History file path exists, but is not a file: {}",
                    history.path()
                );
                eprintln!("Backed up history to: {tmp_file}");
            }
        }

        Ok(())
    }
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
