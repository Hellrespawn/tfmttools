use camino::Utf8PathBuf;
use clap::{Args as ClapArgs, Parser, Subcommand as ClapSubcommand};

use crate::cli::util::PathOrString;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Holds application-wide command line arguments.
pub struct Args {
    /// Sets a custom configuration directory.
    #[arg(short = 'c', long = "config")]
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
    #[command(name = "clear")]
    ClearHistory,
    // ClearHistory(ClearHistory),
    #[command(name = "list")]
    ListTemplates(ListTemplates),
    Rename(Rename),
    Seed(Seed),
    Undo(UndoRedo),
    Redo(UndoRedo),
}

#[derive(ClapArgs, Debug)]
pub struct ClearHistory {}

#[derive(ClapArgs, Debug)]
pub struct ListTemplates {
    #[arg(short = 't', long = "template-directory")]
    pub custom_template_directory: Option<Utf8PathBuf>,
}

#[derive(ClapArgs, Debug)]
pub struct Rename {
    #[arg(short = 'i', long = "input")]
    pub custom_input_directory: Option<Utf8PathBuf>,

    #[arg(short = 't', long = "template-directory")]
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    pub force: bool,

    #[arg(short, long)]
    pub recursion_depth: Option<usize>,

    pub template: PathOrString,

    pub arguments: Vec<String>,
}

#[derive(ClapArgs, Debug)]
pub struct Seed {
    #[arg(short = 't', long = "template-directory")]
    pub custom_template_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    pub force: bool,
}

#[derive(ClapArgs, Debug)]
pub struct UndoRedo {
    #[arg(short, long)]
    pub force: bool,

    pub amount: Option<usize>,
}
