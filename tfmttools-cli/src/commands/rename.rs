use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use tfmttools_core::action::{validate_rename_actions, Action, RenameAction};
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::templates::Template;
use tfmttools_fs::{
    get_longest_common_prefix, ActionHandler, PathIterator, RemoveDirResult,
    TemplateLoader,
};
use tfmttools_history::{Record, SaveHistoryResult};
use tracing::debug;

use super::super::config::Config;
use super::Command;
use crate::history::load_history;
use crate::ui::{
    ConfirmationPrompt, ItemName, PreviewList, ProgressBar, ProgressBarOptions,
};
use crate::util::FileOrName;

const DEFAULT_RECURSION_DEPTH: usize = 4;

#[derive(Debug)]
pub struct RenameCommand {
    input_directory: Utf8PathBuf,
    template_directory: Utf8PathBuf,

    yes: bool,

    recursion_depth: usize,

    template: FileOrName,
    arguments: Vec<String>,
}

impl RenameCommand {
    pub fn new(
        input_directory: Utf8PathBuf,
        template_directory: Utf8PathBuf,
        yes: bool,
        recursion_depth: Option<usize>,
        template: FileOrName,
        arguments: Vec<String>,
    ) -> Self {
        Self {
            input_directory,
            template_directory,
            yes,
            recursion_depth: recursion_depth.unwrap_or(DEFAULT_RECURSION_DEPTH),
            template,
            arguments,
        }
    }
}

impl Command for RenameCommand {
    fn run(&self, config: &Config) -> Result<()> {
        InnerRename { options: self, config }.rename()
    }
}

struct InnerRename<'ir> {
    options: &'ir RenameCommand,
    config: &'ir Config,
}

impl<'a> InnerRename<'a> {
    pub fn rename(&self) -> Result<()> {
        let loader = match &self.options.template {
            FileOrName::File(path, string) => {
                TemplateLoader::read_filename(path, string)
            },
            FileOrName::Name(_) => {
                TemplateLoader::read_directory(&self.options.template_directory)
            },
        }?;

        let template_name = self.options.template.as_str();

        let template = loader
            .get_template(template_name, self.options.arguments.clone())
            .ok_or(eyre!("Unable to find template: {}", template_name))?;

        let files = self.gather_files()?;

        let rename_actions = self.create_rename_actions(&template, &files)?;
        let rename_actions =
            RenameAction::filter_unchanged_destinations(rename_actions);

        if rename_actions.is_empty() {
            println!("There are no audio files to rename.");
            Ok(())
        } else {
            Self::validate_rename_actions(&rename_actions)?;

            let confirmation = self.options.yes
                || self.confirm_rename_actions(&rename_actions)?;

            if confirmation {
                let actions = self.perform_rename_actions(rename_actions)?;

                self.store_history(actions)?;
            } else {
                println!("Aborting!");
            }

            Ok(())
        }
    }

    fn gather_files(&self) -> Result<Vec<AudioFile>> {
        let options = ProgressBarOptions::spinner(
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

    fn create_rename_actions(
        &self,
        template: &Template,
        files: &[AudioFile],
    ) -> Result<Vec<RenameAction>> {
        let cwd = self.config.working_directory()?;

        let options = ProgressBarOptions::bar(
            "Determining output paths:",
            "Determined output paths.",
        )?;

        let bar = ProgressBar::with_length(options, files.len() as u64);

        let rename_actions: Result<Vec<RenameAction>> = files
            .iter()
            .map(|audiofile| {
                Ok(RenameAction::new(
                    audiofile.path().to_owned(),
                    audiofile.construct_target_path(template, &cwd)?,
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

        rename_actions
    }

    fn validate_rename_actions(rename_actions: &[RenameAction]) -> Result<()> {
        let validation_errors = validate_rename_actions(rename_actions);

        if validation_errors.is_empty() {
            Ok(())
        } else {
            Err(eyre!("Had validation errors:"))
        }
    }

    fn confirm_rename_actions(
        &self,
        rename_actions: &[RenameAction],
    ) -> Result<bool> {
        let cwd = self.config.working_directory()?;

        Self::preview_rename_actions(rename_actions, &cwd)?;

        let confirmation_prompt = ConfirmationPrompt::new("Move files?");

        confirmation_prompt.prompt()
    }

    fn preview_rename_actions(
        rename_actions: &[RenameAction],
        working_directory: &Utf8Path,
    ) -> Result<()> {
        const LEADING_LINES: usize = 3;
        const TRAILING_LINES: usize = 3;

        let iter = rename_actions.iter().map(|rename_action| {
            let path = rename_action
                .target()
                .strip_prefix(working_directory)
                .unwrap_or(rename_action.target());

            if path.is_relative() {
                format!(".{}{path}", std::path::MAIN_SEPARATOR)
            } else {
                format!("{path}")
            }
        });

        let preview_list = PreviewList::new(iter)
            .leading(LEADING_LINES)
            .trailing(TRAILING_LINES)
            .item_name(ItemName::simple("file"));

        preview_list.print()?;

        Ok(())
    }

    fn perform_rename_actions(
        &self,
        rename_actions: Vec<RenameAction>,
    ) -> Result<Vec<Action>> {
        let common_prefix = get_longest_common_prefix(
            &rename_actions
                .iter()
                .map(RenameAction::source)
                .collect::<Vec<_>>(),
        );

        let mut actions = self.move_files(rename_actions)?;

        debug!("Common prefix of path: {:?}", common_prefix);

        if let Some(common_path) = common_prefix {
            let removed =
                self.config.fs_handler().remove_empty_subdirectories(
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

    fn move_files(
        &self,
        rename_actions: Vec<RenameAction>,
    ) -> Result<Vec<Action>> {
        let options = ProgressBarOptions::bar("Moving files:", "Moved files.")?;

        let bar =
            ProgressBar::with_length(options, rename_actions.len() as u64);

        let actions = RenameAction::create_actions(rename_actions);

        let handler = ActionHandler::new(self.config.fs_handler());

        for action in &actions {
            handler.apply(action)?;

            if action.is_rename() {
                bar.inc_found();

                #[cfg(feature = "debug")]
                crate::debug::delay();
            }
        }

        bar.finish();

        Ok(actions)
    }

    fn store_history(&self, actions: Vec<Action>) -> Result<()> {
        if !self.config.fs_handler().dry_run() {
            let mut history = load_history(self.config)?.unwrap();

            let metadata = ActionRecordMetadata::new(
                self.options.template.as_str().to_owned(),
                self.options.arguments.clone(),
            );

            let record = Record::with_metadata(actions, metadata);

            history.push(record)?;

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
