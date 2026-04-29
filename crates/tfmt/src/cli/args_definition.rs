use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use tfmttools_fs::FileOrName;

use crate::ui::PreviewListSize;

/// Holds application-wide command line arguments.
#[derive(Parser, Debug)]
#[command(name = "tfmt", version, about, long_about = None)]
pub struct TFMTArgs {
    /// Sets a custom configuration directory. Defaults to '~/.tfmt'.
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

    /// Validate audio files and their tags.
    Validate(ValidateArgs),

    /// Undo actions.
    Undo(UndoRedoArgs),

    /// Redo actions.
    Redo(UndoRedoArgs),
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
pub struct ValidateArgs {
    #[command(flatten)]
    pub common_args: ValidateCommonArgs,

    #[command(subcommand)]
    pub command: Option<ValidateSubcommand>,
}

#[derive(Args, Debug)]
pub struct ValidateCommonArgs {
    #[arg(short = 'i', long = "input-directory")]
    /// Directory to scan for input files. Defaults to the current directory.
    pub custom_input_directory: Option<Utf8PathBuf>,

    #[arg(short, long)]
    /// Set custom recursion depth for scan.
    pub recursion_depth: Option<usize>,
}

#[derive(Subcommand, Debug)]
pub enum ValidateSubcommand {
    /// Check audio files and their tags.
    Check,

    /// Fix audio files and their tags.
    Fix(ValidateFixArgs),
}

#[derive(Args, Debug)]
pub struct ValidateFixArgs {
    #[command(subcommand)]
    pub command: ValidateFixSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ValidateFixSubcommand {
    /// Fix ID3 text decoded as the wrong encoding in MP3 files.
    #[command(name = "id3-encoding")]
    Id3Encoding(FixId3EncodingArgs),

    /// Replace forbidden characters in tag values.
    Characters,
}

#[derive(Args, Debug)]
pub struct FixId3EncodingArgs {
    #[arg(long, default_value = "UTF-16")]
    /// Target ID3 text encoding to write. Defaults to UTF-16.
    pub encoding: String,
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
pub struct UndoRedoArgs {
    /// Amount of actions.
    pub amount: Option<usize>,
}
