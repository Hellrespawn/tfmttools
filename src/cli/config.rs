use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use color_eyre::Result;

use super::commands::clear_history::ClearHistory;
use super::commands::list_templates::ListTemplates;
use super::commands::rename::Rename;
use super::commands::seed::Seed;
use super::commands::undo_redo::{Redo, Undo};
use super::commands::Command;

pub const DRY_RUN_PREFIX: &str = "[D] ";

const DEFAULT_HISTORY_FILENAME: &str = concat!(env!("CARGO_PKG_NAME"), ".hist");

pub fn default_input_dir() -> Utf8PathBuf {
    let path =
        std::env::current_dir().expect("Unable to determine current directory");

    path.clone().try_into().unwrap_or_else(|_| {
        panic!("Current directory is not valid UTF-8: {}", path.display())
    })
}

pub fn default_template_and_config_dir() -> Utf8PathBuf {
    let home = dirs::home_dir().expect("Unable to read home directory!");

    let path = home.join(format!(".{}", env!("CARGO_PKG_NAME")));

    path.clone().try_into().unwrap_or_else(|_| {
        panic!("Path is not valid UTF-8: {}", path.display())
    })
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub struct Config {
    /// Sets a custom config and template_directory
    #[arg(short, long, alias = "config", default_value_t = default_template_and_config_dir())]
    pub config_directory: Utf8PathBuf,

    #[command(subcommand)]
    pub command: CommandOptions,
}

#[derive(Subcommand, Debug)]
pub enum CommandOptions {
    #[command(name = "clear")]
    ClearHistory(ClearHistory),
    #[command(name = "list")]
    ListTemplates(ListTemplates),
    Rename(Rename),
    Seed(Seed),
    Undo(Undo),
    Redo(Redo),
}

impl Config {
    pub fn parse_from_args() -> Self {
        Self::parse()
    }

    pub fn command(&self) -> &dyn Command {
        match &self.command {
            CommandOptions::ClearHistory(c) => c,
            CommandOptions::ListTemplates(l) => l,
            CommandOptions::Rename(r) => r,
            CommandOptions::Seed(s) => s,
            CommandOptions::Undo(u) => u,
            CommandOptions::Redo(r) => r,
        }
    }

    #[allow(clippy::unused_self)]
    pub fn working_directory(&self) -> Result<Utf8PathBuf> {
        let path = std::env::current_dir()?;

        Ok(path.try_into()?)
    }

    pub fn history_file(&self) -> Utf8PathBuf {
        self.config_directory.join(DEFAULT_HISTORY_FILENAME)
    }
}
