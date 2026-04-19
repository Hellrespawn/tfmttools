use camino::Utf8PathBuf;
use clap::{Args, Command, CommandFactory, Parser, Subcommand};
use color_eyre::Result;
use tfmttools_core::util::Utf8Directory;
use tfmttools_fs::{FileOrName, FsHandler};
use tfmttools_history::HistoryMode;
use tracing::debug;

use super::{ConfirmMode, TFMTOptions};
use crate::commands::{
    RenameContext, UndoRedoCommand, clear_history, list_templates, rename,
    show_history,
};
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
    pub fn run(
        self,
        app_options: &TFMTOptions,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        match self {
            TFMTSubcommand::ClearHistory => {
                clear_history(app_options)?;
            },
            TFMTSubcommand::ListTemplates(list_templates_args) => {
                let template_directory = list_templates_args
                    .custom_template_directory
                    .map(Utf8Directory::new)
                    .unwrap_or(Ok(app_options.config_directory().to_owned()))?;

                list_templates(&template_directory)?;
            },
            TFMTSubcommand::Rename(rename_args) => {
                let rename_context = RenameContext::try_from_args(
                    fs_handler,
                    app_options,
                    rename_args,
                )?;

                rename(&rename_context)?;
            },
            TFMTSubcommand::Undo(undo_redo_args) => {
                undo_redo_args.run(
                    HistoryMode::Undo,
                    app_options,
                    fs_handler,
                )?;
            },
            TFMTSubcommand::Redo(undo_redo_args) => {
                undo_redo_args.run(
                    HistoryMode::Redo,
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
            TFMTSubcommand::ShowHistory => "history",
            TFMTSubcommand::ListTemplates(..) => "list-templates",
            TFMTSubcommand::Rename(..) => "rename",
            TFMTSubcommand::Undo(..) => "undo",
            TFMTSubcommand::Redo(..) => "redo",
        }
    }
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
pub struct UndoRedoArgs {
    /// Amount of actions.
    pub amount: Option<usize>,
}

impl UndoRedoArgs {
    fn run(
        self,
        mode: HistoryMode,
        app_options: &TFMTOptions,
        fs_handler: &FsHandler,
    ) -> Result<()> {
        UndoRedoCommand::new(
            matches!(app_options.confirm_mode(), ConfirmMode::NoConfirm),
            self.amount.unwrap_or(1),
            mode,
            app_options.preview_list_size(),
        )
        .run(app_options, fs_handler)
    }
}
