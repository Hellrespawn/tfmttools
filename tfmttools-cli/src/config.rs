use camino::Utf8PathBuf;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use tfmttools_fs::FsHandler;
use tfmttools_history::HistoryMode;

use super::args::Args;
use super::commands::list_templates::ListTemplatesCommand;
use super::commands::rename::RenameCommand;
use super::commands::undo_redo::UndoRedoCommand;
use super::commands::Command;
use crate::args::Subcommand;
use crate::commands::clear_history::ClearHistoryCommand;
use crate::commands::copy_tags::CopyTagsCommand;
use crate::commands::fix::FixCommand;
use crate::commands::show_history::ShowHistoryCommand;

pub fn default_input_dir() -> Result<Utf8PathBuf> {
    let path = std::env::current_dir()?;

    Ok(path.clone().try_into()?)
}

pub fn default_template_and_config_dir() -> Result<Utf8PathBuf> {
    let home =
        dirs::home_dir().ok_or(eyre!("Unable to determine home directory."))?;

    let path = home.join(format!(".{}", crate::PKG_NAME));

    Ok(path.clone().try_into()?)
}

/// Holds application-wide command line arguments.
#[derive(Debug)]
pub struct Config {
    directory: Utf8PathBuf,
    fs_handler: FsHandler,
    command: Box<dyn Command>,
}

impl Config {
    pub fn from_args(args: Args) -> Result<Self> {
        let config_directory =
            if let Some(config) = args.custom_config_directory {
                Ok(config)
            } else {
                default_template_and_config_dir()
            }?;

        let fs_handler = FsHandler::new(args.dry_run);

        let command: Box<dyn Command> = match args.command {
            Subcommand::ClearHistory => Box::new(ClearHistoryCommand),
            Subcommand::Templates(list_templates) => {
                let template_directory = Self::get_template_directory(
                    list_templates.custom_template_directory,
                )?;

                Box::new(ListTemplatesCommand::new(template_directory))
            },
            Subcommand::Rename(rename) => {
                let input_dir =
                    Self::get_input_directory(rename.custom_input_directory)?;

                let template_directory = Self::get_template_directory(
                    rename.custom_template_directory,
                )?;

                Box::new(RenameCommand::new(
                    input_dir,
                    template_directory,
                    rename.yes,
                    rename.recursion_depth,
                    rename.template,
                    rename.arguments,
                ))
            },
            Subcommand::Undo(undo_redo) => {
                Box::new(UndoRedoCommand::new(
                    undo_redo.yes,
                    undo_redo.amount.unwrap_or(1),
                    HistoryMode::Undo,
                ))
            },
            Subcommand::Redo(undo_redo) => {
                Box::new(UndoRedoCommand::new(
                    undo_redo.yes,
                    undo_redo.amount.unwrap_or(1),
                    HistoryMode::Redo,
                ))
            },
            Subcommand::History(show_history) => {
                Box::new(ShowHistoryCommand::new(show_history.verbose))
            },
            Subcommand::Fix(fix) => {
                let input_dir =
                    Self::get_input_directory(fix.custom_input_directory)?;

                Box::new(FixCommand::new(
                    input_dir,
                    fix.yes,
                    fix.recursion_depth,
                ))
            },

            Subcommand::CopyTags(copy_tags) => {
                Box::new(CopyTagsCommand::new(
                    copy_tags.source,
                    copy_tags.target,
                    copy_tags.yes,
                ))
            },
        };

        Ok(Config { directory: config_directory, fs_handler, command })
    }

    #[allow(clippy::unused_self)]
    pub fn working_directory(&self) -> Result<Utf8PathBuf> {
        let path = std::env::current_dir()?;

        Ok(path.try_into()?)
    }

    pub fn history_file(&self) -> Utf8PathBuf {
        let filename = format!("{}.hist", crate::PKG_NAME);
        self.directory.join(filename)
    }

    fn get_input_directory(
        custom_input_directory: Option<Utf8PathBuf>,
    ) -> Result<Utf8PathBuf> {
        let input_directory =
            if let Some(input_directory) = custom_input_directory {
                Ok(input_directory)
            } else {
                default_input_dir()
            }?;

        Ok(input_directory)
    }

    fn get_template_directory(
        custom_template_directory: Option<Utf8PathBuf>,
    ) -> Result<Utf8PathBuf> {
        let template_directory =
            if let Some(template_directory) = custom_template_directory {
                Ok(template_directory)
            } else {
                default_template_and_config_dir()
            }?;

        Ok(template_directory)
    }

    pub fn dry_run(&self) -> bool {
        self.fs_handler().dry_run()
    }

    pub fn fs_handler(&self) -> &FsHandler {
        &self.fs_handler
    }

    pub fn command(&self) -> &dyn Command {
        self.command.as_ref()
    }
}
