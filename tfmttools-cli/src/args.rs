use camino::Utf8PathBuf;
use clap::{Args, Command, CommandFactory, Parser, Subcommand};
use tfmttools_fs::FileOrName;
use tracing::debug;

use crate::ui::PreviewListSize;

fn default_recursion_depth() -> usize {
    4
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub struct TFMTArgs {
    /// Sets a custom configuration directory. Defaults to '~/.tfmttools'.
    #[arg(short = 'c', long = "config-directory")]
    pub custom_config_directory: Option<Utf8PathBuf>,

    /// Don't actually perform actions.
    #[arg(short, long)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: TFMTSubcommand,

    #[arg(hide = true, long = "run-id")]
    pub custom_run_id: Option<String>,

    #[arg(hide = true, long)]
    pub preview_list_size: Option<PreviewListSize>,
}

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

#[derive(Subcommand, Debug)]
pub enum TFMTSubcommand {
    /// Clear the rename history.
    ClearHistory,

    #[command(alias = "show-history")]
    /// Show a summary of the rename history.
    History(History),

    #[command(alias = "list-templates")]
    /// Show a summary of template in the current and application directory.
    Templates(Templates),

    /// Use a template to rename audio files according to their tags.
    Rename(Rename),

    /// Undo actions.
    Undo(UndoRedo),

    /// Redo actions.
    Redo(UndoRedo),
}

impl TFMTSubcommand {
    pub fn name(&self) -> String {
        match self {
            TFMTSubcommand::ClearHistory => "clear-history",
            TFMTSubcommand::History(..) => "history",
            TFMTSubcommand::Templates(..) => "templates",
            TFMTSubcommand::Rename(..) => "rename",
            TFMTSubcommand::Undo(..) => "undo",
            TFMTSubcommand::Redo(..) => "redo",
        }
        .to_owned()
    }
}

#[derive(Args, Debug)]
pub struct History {
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Increase output verbosity.
    pub verbose: u8,
}

#[derive(Args, Debug)]
pub struct Templates {
    #[arg(short = 't', long = "template-directory")]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,
}

#[derive(Args, Debug)]
pub struct Rename {
    #[arg(short = 'i', long = "input-directory")]
    /// Directory to scan for input files. Defaults to the current directory.
    pub custom_input_directory: Option<Utf8PathBuf>,

    #[arg(short = 't', long = "template-directory")]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(long = "bin-directory")]
    /// Directory to move deleted covers and such to. Defaults to a subfolder of the configuration directory
    pub custom_bin_directory: Option<Utf8PathBuf>,

    #[arg(short, long, default_value_t = default_recursion_depth())]
    /// Set custom recursion depth for scan.
    pub recursion_depth: usize,

    #[arg(short, long)]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub yes: bool,

    /// Path to or name of template.
    pub template: Option<FileOrName>,

    /// Template arguments.
    pub arguments: Vec<String>,

    #[arg(hide = true, long)]
    pub always_copy: bool,
}

#[derive(Args, Debug)]
pub struct Seed {
    #[arg(short = 't', long = "template-directory")]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct UndoRedo {
    #[arg(short, long)]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub yes: bool,

    /// Amount of actions.
    pub amount: Option<usize>,
}
