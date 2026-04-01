use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::error::TFMTResult;
use tfmttools_core::history::{ActionRecordMetadata, TemplateMetadata};
use tfmttools_core::templates::Template;
use tfmttools_fs::{FileOrName, PathIterator, TemplateLoader};
use tfmttools_history_core::{History, LoadHistoryResult};
use tracing::{debug, trace};

use super::RenameContext;
use crate::options::TemplateOption;
use crate::term::current_dir_utf8;
use crate::ui::ProgressBar;

pub fn create_actions(
    context: &RenameContext,
    history: &mut History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<(Vec<RenameAction>, ActionRecordMetadata)> {
    match context.rename_options().template_option() {
        TemplateOption::None => {
            create_actions_from_previous_template(
                context,
                history,
                load_history_result,
            )
        },
        TemplateOption::FileOrName(file_or_name) => {
            create_actions_from_file_or_name(
                context,
                file_or_name,
                context.rename_options().arguments(),
            )
        },
        TemplateOption::Script(script) => {
            create_actions_from_script(
                context,
                script,
                context.rename_options().arguments(),
            )
        },
    }
}

fn create_actions_from_previous_template(
    context: &RenameContext,
    history: &mut History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<(Vec<RenameAction>, ActionRecordMetadata)> {
    if let LoadHistoryResult::Loaded = load_history_result {
        let record = history.get_previous_record()?;

        if let Some(record) = record {
            let metadata = record.metadata();

            match metadata.template() {
                TemplateMetadata::FileOrName(file_or_name) => {
                    let template_name = FileOrName::from(file_or_name.as_str());

                    let arguments = metadata.arguments().to_owned();

                    println!(
                        "Re-using template '{template_name}' and arguments from previous rename."
                    );

                    debug!("Using data from previous rename");

                    return create_actions_from_file_or_name(
                        context,
                        &template_name,
                        &arguments,
                    );
                },
                TemplateMetadata::Script(script) => {
                    let arguments = metadata.arguments().to_owned();

                    println!(
                        "Re-using script\n```\n'{script}'\n```\n and arguments from previous rename."
                    );

                    debug!("Using data from previous rename");

                    return create_actions_from_script(
                        context, script, &arguments,
                    );
                },
            }
        }
    }

    Err(eyre!("No template specified and no data from previous run available."))
}

fn create_actions_from_file_or_name(
    context: &RenameContext,
    file_or_name: &FileOrName,
    arguments: &[String],
) -> Result<(Vec<RenameAction>, ActionRecordMetadata)> {
    debug!("Using template: '{file_or_name}'");
    debug!("Template arguments: '{}'", arguments.join("', '"));

    let loader = match &file_or_name {
        FileOrName::File(path, string) => {
            TemplateLoader::read_filename(path, string)
        },
        FileOrName::Name(_) => {
            TemplateLoader::read_directory(
                context.rename_options().template_directory(),
            )
        },
    }?;

    let template_name = file_or_name.as_str();

    let metadata = ActionRecordMetadata::new(
        TemplateMetadata::FileOrName(template_name.to_owned()),
        arguments.to_vec(),
        context.app_options().run_id().to_owned(),
    );

    create_actions_from_loader_and_script_name(
        context,
        &loader,
        template_name,
        arguments,
        metadata,
    )
}

fn create_actions_from_script(
    context: &RenameContext,
    script: &str,
    arguments: &[String],
) -> Result<(Vec<RenameAction>, ActionRecordMetadata)> {
    debug!("Using script:\n```\n{script}\n```");
    debug!("Template arguments: '{}'", arguments.join("', '"));
    let loader = TemplateLoader::read_script(script)?;
    let metadata = ActionRecordMetadata::new(
        TemplateMetadata::Script(script.to_owned()),
        arguments.to_vec(),
        context.app_options().run_id().to_owned(),
    );

    create_actions_from_loader_and_script_name(
        context,
        &loader,
        TemplateLoader::DEFAULT_SCRIPT_NAME,
        arguments,
        metadata,
    )
}

fn create_actions_from_loader_and_script_name(
    context: &RenameContext,
    loader: &TemplateLoader,
    template_name: &str,
    arguments: &[String],
    metadata: ActionRecordMetadata,
) -> Result<(Vec<RenameAction>, ActionRecordMetadata)> {
    let template = loader
        .get_template(template_name, arguments.to_vec())
        .ok_or(eyre!("Unable to find template: {}", template_name))?;

    let paths = gather_file_paths(context);

    debug!("Read {} files.", paths.len());

    let audio_files = read_files(context, paths)?;

    debug!("Found {} audio files.", audio_files.len());

    let rename_actions =
        create_rename_actions(context, &template, &audio_files)?;

    Ok((rename_actions, metadata))
}

fn gather_file_paths(context: &RenameContext) -> Vec<Utf8PathBuf> {
    let spinner = ProgressBar::spinner(
        context.app_options().display_mode(),
        "audio",
        "total files",
        "Gathering files...",
        "Gathered files.",
    );

    let file_paths = PathIterator::new(context.path_iterator_options())
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

fn read_files(
    context: &RenameContext,
    file_paths: Vec<Utf8PathBuf>,
) -> Result<Vec<AudioFile>> {
    let bar = ProgressBar::bar(
        context.app_options().display_mode(),
        "Reading files...",
        "Read files.",
        file_paths.len() as u64,
        false,
    );

    let audio_files = file_paths
        .into_iter()
        .inspect(|_| {
            bar.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .map(|path| {
            let audio_file = AudioFile::new(path)?;

            trace!("Found audio file: {audio_file:?}");

            Ok(audio_file)
        })
        .collect::<TFMTResult<Vec<_>>>();

    bar.finish();

    Ok(audio_files?)
}

fn create_rename_actions(
    context: &RenameContext,
    template: &Template,
    files: &[AudioFile],
) -> Result<Vec<RenameAction>> {
    let cwd = current_dir_utf8()?;

    let bar = ProgressBar::bar(
        context.app_options().display_mode(),
        "Determining output paths:",
        "Determined output paths.",
        files.len() as u64,
        true,
    );

    let rename_actions: Result<Vec<RenameAction>> = files
        .iter()
        .map(|audiofile| {
            let rename_action = RenameAction::new(
                audiofile.file().to_owned(),
                audiofile.construct_target_path(template, &cwd)?,
            );

            bar.inc_found();
            trace!("Created rename action: {rename_action:?}");

            #[cfg(feature = "debug")]
            crate::debug::delay();

            Ok(rename_action)
        })
        .collect();

    bar.finish();

    rename_actions
}
