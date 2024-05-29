use camino::Utf8PathBuf;
use clap::{Args as ClapArgs, Parser, Subcommand as ClapSubcommand};
use tfmttools_fs::FileOrName;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub struct Args {
    /// Sets a custom configuration directory. Defaults to '~/.tfmttools'.
    #[arg(short = 'c', long)]
    pub custom_config_directory: Option<Utf8PathBuf>,

    #[arg(short, long, hide = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Subcommand,
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[derive(ClapSubcommand, Debug)]
pub enum Subcommand {
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

#[derive(ClapArgs, Debug)]
pub struct History {
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Increase output verbosity.
    pub verbose: u8,
}

#[derive(ClapArgs, Debug)]
pub struct Templates {
    #[arg(short = 't', long)]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,
}

#[derive(ClapArgs, Debug)]
pub struct Rename {
    #[arg(short = 'i', long)]
    /// Directory to scan for input files. Defaults to the current directory.
    pub custom_input_directory: Option<Utf8PathBuf>,

    #[arg(short = 't', long)]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    /// Set custom recursion depth for scan.
    pub recursion_depth: Option<usize>,

    #[arg(short, long)]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub yes: bool,

    /// Path to or name of template.
    pub template: FileOrName,

    /// Template arguments.
    pub arguments: Vec<String>,
}

#[derive(ClapArgs, Debug)]
pub struct Seed {
    #[arg(short = 't', long)]
    /// Directory to read templates from. Defaults to the configuration directory.
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub yes: bool,
}

#[derive(ClapArgs, Debug)]
pub struct UndoRedo {
    #[arg(short, long)]
    /// Skips confirmation prompt. Suitable for non-interactive use.
    pub yes: bool,

    /// Amount of actions.
    pub amount: Option<usize>,
}
