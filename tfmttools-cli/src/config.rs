use camino::Utf8PathBuf;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use tfmttools_history::HistoryMode;

use super::args::Args;
use super::commands::list_templates::ListTemplates;
use super::commands::rename::Rename;
use super::commands::seed::Seed;
use super::commands::undo_redo::UndoRedo;
use super::commands::Command;
use crate::args::Subcommand;
use crate::commands::clear_history::ClearHistory;
use crate::commands::show_history::ShowHistory;

pub const DRY_RUN_PREFIX: &str = "[D] ";

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
    dry_run: bool,
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

        let dry_run = args.dry_run;

        let command: Box<dyn Command> = match args.command {
            Subcommand::ClearHistory => Box::new(ClearHistory),
            Subcommand::ListTemplates(list_templates) => {
                let template_directory = Self::get_template_directory(
                    list_templates.custom_template_directory,
                )?;

                Box::new(ListTemplates::new(template_directory))
            },
            Subcommand::Rename(rename) => {
                let input_dir =
                    Self::get_input_directory(rename.custom_input_directory)?;

                let template_directory = Self::get_template_directory(
                    rename.custom_template_directory,
                )?;

                Box::new(Rename::new(
                    input_dir,
                    template_directory,
                    rename.force,
                    rename.recursion_depth,
                    rename.template,
                    rename.arguments,
                ))
            },
            Subcommand::Seed(seed) => {
                let template_directory = Self::get_template_directory(
                    seed.custom_template_directory,
                )?;

                Box::new(Seed::new(template_directory, seed.force))
            },
            Subcommand::Undo(undo_redo) => {
                Box::new(UndoRedo::new(
                    undo_redo.force,
                    undo_redo.amount.unwrap_or(1),
                    HistoryMode::Undo,
                ))
            },
            Subcommand::Redo(undo_redo) => {
                Box::new(UndoRedo::new(
                    undo_redo.force,
                    undo_redo.amount.unwrap_or(1),
                    HistoryMode::Redo,
                ))
            },
            Subcommand::ShowHistory(show_history) => {
                Box::new(ShowHistory::new(show_history.verbose))
            },
        };

        Ok(Config { directory: config_directory, dry_run, command })
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
        self.dry_run
    }

    pub fn command(&self) -> &dyn Command {
        self.command.as_ref()
    }
}
