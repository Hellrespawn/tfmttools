use camino::Utf8PathBuf;
use clap::{Args, Command, CommandFactory, Parser, Subcommand};
use tfmttools_fs::FileOrName;
use tracing::debug;

use crate::ui::PreviewListSize;

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

    /// Don't display progress bars.
    #[arg(long)]
    pub simple: bool,

    #[arg(short, long, alias = "yes")]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub no_confirm: bool,

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

    #[command(alias = "history")]
    /// Show a summary of the rename history.
    ShowHistory,

    #[command(alias = "templates")]
    /// Show a summary of template in the current and application directory.
    ListTemplates(ListTemplatesArgs),

    /// Use a template to rename audio files according to their tags.
    Rename(RenameArgs),

    /// Undo actions.
    Undo(UndoRedoArgs),

    /// Redo actions.
    Redo(UndoRedoArgs),
}

impl TFMTSubcommand {
    pub fn name(&self) -> String {
        match self {
            TFMTSubcommand::ClearHistory => "clear-history",
            TFMTSubcommand::ShowHistory => "history",
            TFMTSubcommand::ListTemplates(..) => "list-templates",
            TFMTSubcommand::Rename(..) => "rename",
            TFMTSubcommand::Undo(..) => "undo",
            TFMTSubcommand::Redo(..) => "redo",
        }
        .to_owned()
    }
}

#[derive(Args, Debug)]
pub struct ShowHistoryArgs {
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Increase output verbosity.
    pub verbose: u8,
}

#[derive(Args, Debug)]
pub struct ListTemplatesArgs {
    #[arg(short = 't', long = "template-directory")]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    #[arg(short = 'i', long = "input-directory")]
    /// Directory to scan for input files. Defaults to the current directory.
    pub custom_input_directory: Option<Utf8PathBuf>,

    #[arg(long = "template-directory")]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(long = "bin-directory")]
    /// Directory to move deleted covers and such to. Defaults to a subfolder of the configuration directory
    pub custom_bin_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    /// Set custom recursion depth for scan.
    pub recursion_depth: Option<usize>,

    #[arg(hide = true, long)]
    pub always_copy: bool,

    #[command(flatten)]
    pub template_args: TemplateArgs,

    /// Template arguments.
    pub arguments: Vec<String>,
}

#[derive(Args, Debug)]
#[group(required = false, multiple = false)]
pub struct TemplateArgs {
    #[arg(short, long)]
    /// Path to or name of template.
    pub template: Option<FileOrName>,

    #[arg(short, long)]
    /// Path to or name of template.
    pub script: Option<String>,
}

#[derive(Args, Debug)]
pub struct Seed {
    #[arg(short = 't', long = "template-directory")]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,
}

#[derive(Args, Debug)]
pub struct UndoRedoArgs {
    /// Amount of actions.
    pub amount: Option<usize>,
}
