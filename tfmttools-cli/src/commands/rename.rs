use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::{Action, RenameAction, validate_rename_actions};
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::error::TFMTResult;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::templates::Template;
use tfmttools_fs::{
    ActionHandler, FileOrName, FsHandler, PathIterator, PathIteratorOptions,
    RemoveDirResult, TemplateLoader, get_longest_common_prefix,
};
use tfmttools_history::{History, HistoryError, LoadHistoryResult};
use tracing::debug;

use crate::config::paths::AppPaths;
use crate::history::load_history;
use crate::ui::{
    ConfirmationPrompt, ItemName, PreviewList, ProgressBar, ProgressBarOptions,
};

#[derive(Debug)]
pub struct RenameTemplateOptions {
    template_directory: Utf8PathBuf,

    template: Option<FileOrName>,
    arguments: Vec<String>,
}

impl RenameTemplateOptions {
    pub fn new(
        template_directory: Utf8PathBuf,
        template: Option<FileOrName>,
        arguments: Vec<String>,
    ) -> Self {
        Self { template_directory, template, arguments }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenameMiscOptions {
    always_copy: bool,
    no_confirm: bool,
    dry_run: bool,
}

impl RenameMiscOptions {
    pub fn new(always_copy: bool, no_confirm: bool, dry_run: bool) -> Self {
        Self { always_copy, no_confirm, dry_run }
    }
}

#[derive(Debug)]
pub struct RenameContext<'rc> {
    app_paths: &'rc AppPaths,
    fs_handler: &'rc FsHandler,
    path_iterator_options: &'rc PathIteratorOptions<'rc>,
    template_options: &'rc RenameTemplateOptions,
    misc_options: RenameMiscOptions,
}

impl<'rc> RenameContext<'rc> {
    pub fn new(
        app_paths: &'rc AppPaths,
        fs_handler: &'rc FsHandler,
        path_iterator_options: &'rc PathIteratorOptions,
        template_options: &'rc RenameTemplateOptions,
        misc_options: RenameMiscOptions,
    ) -> Self {
        Self {
            app_paths,
            fs_handler,
            path_iterator_options,
            template_options,
            misc_options,
        }
    }
}

pub fn rename(context: &RenameContext) -> Result<()> {
    let (file_or_name, arguments) = get_template_name_and_arguments(context)?;

    let loader = match &file_or_name {
        FileOrName::File(path, string) => {
            TemplateLoader::read_filename(path, string)
        },
        FileOrName::Name(_) => {
            TemplateLoader::read_directory(
                &context.template_options.template_directory,
            )
        },
    }?;

    let template_name = file_or_name.as_str();

    let template = loader
        .get_template(template_name, arguments.clone())
        .ok_or(eyre!("Unable to find template: {}", template_name))?;

    let paths = gather_file_paths(context);

    let audio_files = read_files(paths)?;

    let rename_actions =
        create_rename_actions(context, &template, &audio_files)?;
    let rename_actions =
        RenameAction::filter_unchanged_destinations(rename_actions);

    if rename_actions.is_empty() {
        println!("There are no audio files to rename.");
    } else {
        validate_rename_action_errors(&rename_actions)?;

        let confirmation = context.misc_options.no_confirm
            || confirm_rename_actions(context, &rename_actions)?;

        if confirmation {
            let actions = perform_rename_actions(context, rename_actions)?;

            store_history(context, actions, template_name, &arguments)?;
        } else {
            println!("Aborting!");
        }
    }

    Ok(())
}

fn get_template_name_and_arguments(
    context: &RenameContext,
) -> Result<(FileOrName, Vec<String>)> {
    if let Some(file_or_name) = &context.template_options.template {
        debug!("Using template and arguments from command line.");

        Ok((file_or_name.clone(), context.template_options.arguments.clone()))
    } else {
        let (history, load_history_result) =
            load_history(&context.app_paths.history_file())?;

        if let LoadHistoryResult::Loaded = load_history_result {
            let metadata_option = history
                .get_previous_record()?
                .map(tfmttools_history::Record::metadata);

            if let Some(metadata) = metadata_option {
                let template_name = FileOrName::from(metadata.template());

                let arguments = metadata.arguments().to_owned();

                println!(
                    "Re-using template '{template_name}' and arguments from previous rename."
                );

                debug!(
                    "Using previous rename data:\ntemplate: '{}'\narguments: '{}'",
                    template_name,
                    arguments.join("', '")
                );

                return Ok((template_name, arguments));
            }
        }

        Err(eyre!(
            "No template specified and no data from previous run available."
        ))
    }
}

fn gather_file_paths(context: &RenameContext) -> Vec<Utf8PathBuf> {
    let progress_bar_options = ProgressBarOptions::spinner(
        "audio",
        "total",
        "Gathering files...",
        "Gathered files.",
    );

    let spinner = ProgressBar::new(progress_bar_options);

    let file_paths = PathIterator::new(context.path_iterator_options)
        .flatten()
        .inspect(|_| spinner.inc_total())
        .filter(|path| AudioFile::path_predicate(path))
        .inspect(|_| {
            spinner.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .collect::<Vec<_>>();

    spinner.finish();

    file_paths
}

fn read_files(file_paths: Vec<Utf8PathBuf>) -> Result<Vec<AudioFile>> {
    let options = ProgressBarOptions::bar("Reading files...", "Read files.");

    let bar = ProgressBar::with_length(options, file_paths.len() as u64);

    let audio_files = file_paths
        .into_iter()
        .inspect(|_| {
            bar.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .map(AudioFile::new)
        .collect::<TFMTResult<Vec<_>>>();

    bar.finish();

    Ok(audio_files?)
}

fn create_rename_actions(
    context: &RenameContext,
    template: &Template,
    files: &[AudioFile],
) -> Result<Vec<RenameAction>> {
    let cwd = context.app_paths.working_directory()?;

    let options = ProgressBarOptions::bar(
        "Determining output paths:",
        "Determined output paths.",
    );

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

fn validate_rename_action_errors(
    rename_actions: &[RenameAction],
) -> Result<()> {
    let validation_errors = validate_rename_actions(rename_actions);

    if validation_errors.is_empty() {
        Ok(())
    } else {
        let error_string = validation_errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        Err(eyre!("Had validation errors:\n{error_string}"))
    }
}

fn confirm_rename_actions(
    context: &RenameContext,
    rename_actions: &[RenameAction],
) -> Result<bool> {
    let cwd = context.app_paths.working_directory()?;

    preview_rename_actions(rename_actions, &cwd)?;

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
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>> {
    let common_prefix = get_longest_common_prefix(
        &rename_actions.iter().map(RenameAction::source).collect::<Vec<_>>(),
    );

    let mut actions = move_files(context, rename_actions)?;

    debug!("Common prefix of path: {:?}", common_prefix);

    if let Some(common_path) = common_prefix {
        let removed = context.fs_handler.remove_empty_subdirectories(
            &common_path,
            context.path_iterator_options.recursion_depth(),
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
    context: &RenameContext,
    rename_actions: Vec<RenameAction>,
) -> Result<Vec<Action>> {
    let options = ProgressBarOptions::bar("Moving files:", "Moved files.");

    let bar = ProgressBar::with_length(options, rename_actions.len() as u64);

    let initial_actions = RenameAction::create_actions(rename_actions);

    let mut applied_actions = Vec::new();

    let handler = ActionHandler::new(
        context.fs_handler,
        context.misc_options.always_copy,
    );

    for action in initial_actions {
        let actions = handler.apply(action)?;

        let is_rename_action = actions
            .iter()
            .any(tfmttools_core::action::Action::is_rename_action);

        applied_actions.extend(actions);

        if is_rename_action {
            bar.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        }
    }

    bar.finish();

    Ok(applied_actions)
}

fn store_history(
    context: &RenameContext,
    actions: Vec<Action>,
    template_name: &str,
    arguments: &[String],
) -> Result<()> {
    if context.misc_options.dry_run {
        Ok(())
    } else {
        let (mut history, _) = load_history(&context.app_paths.history_file())?;

        let metadata = ActionRecordMetadata::new(
            template_name.to_owned(),
            arguments.to_owned(),
        );

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
