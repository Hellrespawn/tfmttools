use clap::{Command, CommandFactory, Parser};
use color_eyre::Result;
use tfmttools_core::history::HistoryMode;
use tfmttools_fs::{FsHandler, verify_directory};
use tracing::debug;

use super::TFMTOptions;
pub use super::args_definition::{
    RenameArgs, TFMTArgs, TFMTSubcommand, TemplateArgs, ValidateArgs,
    ValidateCommonArgs, ValidateType,
};
use crate::commands::{
    clear_history, list_templates, rename, resolve_history, show_history,
    undo_redo, validate,
};

impl TFMTArgs {
    pub fn parse() -> Self {
        let args = <Self as Parser>::parse();

        debug!("Command-line arguments:\n{:#?}", args);

        args
    }

    pub fn command() -> Command {
        <Self as CommandFactory>::command()
    }
}

impl TFMTSubcommand {
    pub fn run(
        self,
        app_options: &TFMTOptions,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        match self {
            TFMTSubcommand::ResolveHistory(args) => {
                let outcome = match args.outcome {
                    super::args_definition::ResolutionOutcome::Applied => {
                        tfmttools_core::history::AttemptOutcome::Applied
                    },
                    super::args_definition::ResolutionOutcome::NotApplied => {
                        tfmttools_core::history::AttemptOutcome::NotApplied
                    },
                };
                resolve_history(app_options, args.attempt, outcome)?;
            },
            TFMTSubcommand::ClearHistory => {
                clear_history(app_options)?;
            },
            TFMTSubcommand::ListTemplates(list_templates_args) => {
                let template_directory =
                    match list_templates_args.custom_template_directory {
                        Some(path) => verify_directory(path)?,
                        None => app_options.config_directory().to_owned(),
                    };

                list_templates(&template_directory)?;
            },
            TFMTSubcommand::Rename(rename_args) => {
                rename(fs_handler, app_options, rename_args)?;
            },
            TFMTSubcommand::Validate(validate_args) => {
                validate(fs_handler, app_options, validate_args)?;
            },
            TFMTSubcommand::Undo(undo_redo_args) => {
                undo_redo(
                    HistoryMode::Undo,
                    undo_redo_args.amount.unwrap_or(1),
                    app_options,
                    fs_handler,
                )?;
            },
            TFMTSubcommand::Redo(undo_redo_args) => {
                undo_redo(
                    HistoryMode::Redo,
                    undo_redo_args.amount.unwrap_or(1),
                    app_options,
                    fs_handler,
                )?;
            },
            TFMTSubcommand::ShowHistory => {
                show_history(app_options)?;
            },
        }

        Ok(())
    }

    pub fn name(&self) -> &'static str {
        match self {
            TFMTSubcommand::ClearHistory => "clear-history",
            TFMTSubcommand::ResolveHistory(..) => "resolve-history",
            TFMTSubcommand::ShowHistory => "history",
            TFMTSubcommand::ListTemplates(..) => "list-templates",
            TFMTSubcommand::Rename(..) => "rename",
            TFMTSubcommand::Validate(..) => "validate",
            TFMTSubcommand::Undo(..) => "undo",
            TFMTSubcommand::Redo(..) => "redo",
        }
    }
}
