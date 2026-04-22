use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecordMetadata, TemplateMetadata};
use tfmttools_fs::{FileOrName, TemplateLoader};
use tfmttools_history::{History, LoadHistoryResult};
use tracing::debug;

use super::RenameSession;
use crate::cli::TemplateOption;

pub(super) struct ResolvedTemplate {
    pub(super) loader: TemplateLoader<'static>,
    pub(super) template_name: String,
    pub(super) arguments: Vec<String>,
    pub(super) metadata: ActionRecordMetadata,
}

pub(super) fn resolve_template(
    session: &RenameSession,
    history: &History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<ResolvedTemplate> {
    match session.rename_options().template_option() {
        TemplateOption::None => {
            resolve_previous_template(session, history, load_history_result)
        },
        TemplateOption::FileOrName(file_or_name) => {
            resolve_file_or_name(
                session,
                file_or_name,
                session.rename_options().arguments(),
            )
        },
        TemplateOption::Script(script) => {
            resolve_script(
                session,
                script,
                session.rename_options().arguments(),
            )
        },
    }
}

fn resolve_previous_template(
    session: &RenameSession,
    history: &History<Action, ActionRecordMetadata>,
    load_history_result: LoadHistoryResult,
) -> Result<ResolvedTemplate> {
    if let LoadHistoryResult::Loaded = load_history_result
        && let Some(record) = history.get_previous_record()?
    {
        let metadata = record.metadata();
        let arguments = metadata.arguments().to_owned();

        debug!("Using data from previous rename");

        return match metadata.template() {
            TemplateMetadata::FileOrName(file_or_name) => {
                let template_name = FileOrName::from(file_or_name.as_str());

                println!(
                    "Re-using template '{template_name}' and arguments from previous rename."
                );

                resolve_file_or_name(session, &template_name, &arguments)
            },
            TemplateMetadata::Script(script) => {
                println!(
                    "Re-using script\n```\n'{script}'\n```\n and arguments from previous rename."
                );

                resolve_script(session, script, &arguments)
            },
        };
    }

    Err(eyre!("No template specified and no data from previous run available."))
}

fn resolve_file_or_name(
    session: &RenameSession,
    file_or_name: &FileOrName,
    arguments: &[String],
) -> Result<ResolvedTemplate> {
    debug!("Using template: '{file_or_name}'");
    debug!("Template arguments: '{}'", arguments.join("', '"));

    let loader = match file_or_name {
        FileOrName::File(path, name) => {
            TemplateLoader::read_filename(path, name)
        },
        FileOrName::Name(_) => {
            TemplateLoader::read_directory(
                session.rename_options().template_directory(),
            )
        },
    }?;

    let template_name = file_or_name.as_str().to_owned();
    let arguments = arguments.to_vec();
    let metadata = create_metadata(
        &TemplateMetadata::FileOrName(template_name.clone()),
        session.app_options().run_id(),
        &arguments,
    );

    Ok(ResolvedTemplate { loader, template_name, arguments, metadata })
}

fn resolve_script(
    session: &RenameSession,
    script: &str,
    arguments: &[String],
) -> Result<ResolvedTemplate> {
    debug!("Using script:\n```\n{script}\n```");
    debug!("Template arguments: '{}'", arguments.join("', '"));

    let loader = TemplateLoader::read_script(script)?;
    let arguments = arguments.to_vec();
    let metadata = create_metadata(
        &TemplateMetadata::Script(script.to_owned()),
        session.app_options().run_id(),
        &arguments,
    );

    Ok(ResolvedTemplate {
        loader,
        template_name: TemplateLoader::DEFAULT_SCRIPT_NAME.to_owned(),
        arguments,
        metadata,
    })
}

fn create_metadata(
    template: &TemplateMetadata,
    run_id: &str,
    arguments: &[String],
) -> ActionRecordMetadata {
    ActionRecordMetadata::new(
        template.to_owned(),
        arguments.to_vec(),
        run_id.to_owned(),
    )
}
