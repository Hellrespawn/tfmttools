use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::history::{
    ActionRecordMetadata, History, LoadHistoryResult, TemplateMetadata,
};
use tfmttools_fs::{FileOrName, discover_templates};
use tfmttools_picotmpl::{BoundTemplate, Template};
use tracing::debug;

use super::RenameSession;
use crate::cli::TemplateOption;
use crate::commands::templates;

pub(super) struct ResolvedTemplate {
    pub(super) template: BoundTemplate,
    pub(super) lookup_name: String,
    pub(super) metadata: ActionRecordMetadata,
}

pub(super) fn resolve_template(
    session: &RenameSession,
    history: &History,
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
        TemplateOption::InlineTemplate(template) => {
            resolve_inline_template(
                session,
                template,
                session.rename_options().arguments(),
            )
        },
    }
}

fn resolve_previous_template(
    session: &RenameSession,
    history: &History,
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
            TemplateMetadata::InlineTemplate(template) => {
                println!(
                    "Re-using template\n```\n'{template}'\n```\n and arguments from previous rename."
                );

                resolve_inline_template(session, template, &arguments)
            },
            TemplateMetadata::Validation(_) => {
                Err(eyre!("No previous rename run found."))
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

    let template = match file_or_name {
        FileOrName::File(path, name) => templates::load(name, path)?,
        FileOrName::Name(name) => {
            let paths = discover_templates(
                session.rename_options().template_directory(),
            )?;
            let matches: Vec<_> = paths
                .iter()
                .filter(|path| path.file_stem() == Some(name.as_str()))
                .collect();
            match matches.as_slice() {
                [path] => templates::load(name, path)?,
                [] => return Err(eyre!("Unable to find template: {name}")),
                _ => {
                    return Err(eyre!(
                        "Template name '{name}' is ambiguous; specify an explicit template file path."
                    ));
                },
            }
        },
    };

    let template_name = file_or_name.as_str().to_owned();
    let metadata = create_metadata(
        &TemplateMetadata::FileOrName(template_name.clone()),
        session.app_options().run_id(),
        arguments,
    );

    Ok(ResolvedTemplate {
        template: bind(&template, &template_name, arguments)?,
        lookup_name: template_name,
        metadata,
    })
}

fn resolve_inline_template(
    session: &RenameSession,
    template: &str,
    arguments: &[String],
) -> Result<ResolvedTemplate> {
    debug!("Using template:\n```\n{template}\n```");
    debug!("Template arguments: '{}'", arguments.join("', '"));

    let compiled = templates::compile("template", template)?;
    let metadata = create_metadata(
        &TemplateMetadata::InlineTemplate(template.to_owned()),
        session.app_options().run_id(),
        arguments,
    );

    Ok(ResolvedTemplate {
        template: bind(&compiled, "template", arguments)?,
        lookup_name: "template".to_owned(),
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

fn bind(
    template: &Template,
    name: &str,
    arguments: &[String],
) -> Result<BoundTemplate> {
    template.bind(arguments).map_err(|error| {
        templates::named_diagnostic(name, &template.format_diagnostic(&error))
            .into()
    })
}
