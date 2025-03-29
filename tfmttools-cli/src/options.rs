use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::util::{ActionMode, MoveMode};
use tfmttools_fs::FileOrName;

use crate::args::{RenameArgs, TFMTArgs};
use crate::term::{current_dir_utf8, terminal_height};
use crate::ui::PreviewListSize;

const BIN_DIRECTORY_NAME: &str = "trash";
const DEFAULT_RECURSION_DEPTH: usize = 4;

#[derive(Debug, Default, Copy, Clone)]
pub enum ConfirmMode {
    #[default]
    Confirm,
    NoConfirm,
}

#[derive(Debug, Default, Copy, Clone)]
pub enum DisplayMode {
    Simple,
    #[default]
    Fancy,
}

#[derive(Debug, Clone)]
pub struct TFMTOptions {
    config_directory: Utf8PathBuf,
    action_mode: ActionMode,
    display_mode: DisplayMode,
    confirm_mode: ConfirmMode,
    verbosity: u8,
    preview_list_size: PreviewListSize,
    run_id: String,
}

impl TFMTOptions {
    pub fn config_directory(&self) -> &Utf8Path {
        self.config_directory.as_ref()
    }

    pub fn action_mode(&self) -> ActionMode {
        self.action_mode
    }

    pub fn display_mode(&self) -> DisplayMode {
        self.display_mode
    }

    pub fn confirm_mode(&self) -> ConfirmMode {
        self.confirm_mode
    }

    pub fn preview_list_size(&self) -> PreviewListSize {
        self.preview_list_size
    }

    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}

impl TFMTOptions {
    pub fn default_application_dir() -> Result<Utf8PathBuf> {
        let path = dirs::home_dir()
            .ok_or(eyre!("Unable to determine home directory."))?
            .join(format!(".{}", crate::PKG_NAME));

        Ok(Utf8PathBuf::try_from(path)?)
    }

    pub fn history_file_path(&self) -> Utf8PathBuf {
        let filename = format!("{}.hist", crate::PKG_NAME);
        self.config_directory.join(filename)
    }

    fn path_or_default(path: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
        if let Some(path) = path {
            Ok(path.to_owned())
        } else {
            Ok(Self::default_application_dir()?)
        }
    }
}

impl TryFrom<&TFMTArgs> for TFMTOptions {
    type Error = color_eyre::Report;

    fn try_from(args: &TFMTArgs) -> Result<Self> {
        let config_directory =
            Self::path_or_default(args.custom_config_directory.as_deref())?;

        let preview_list_size = args
            .preview_list_size
            .unwrap_or_else(|| PreviewListSize::new(terminal_height()));

        Ok(Self {
            config_directory,
            action_mode: if args.dry_run {
                ActionMode::DryRun
            } else {
                ActionMode::Default
            },
            display_mode: if args.simple {
                DisplayMode::Simple
            } else {
                DisplayMode::Fancy
            },
            confirm_mode: if args.no_confirm {
                ConfirmMode::NoConfirm
            } else {
                ConfirmMode::Confirm
            },
            verbosity: 0,
            preview_list_size,
            run_id: args.custom_run_id.clone().unwrap_or_else(|| {
                std::iter::repeat_with(fastrand::alphanumeric)
                    .take(12)
                    .collect()
            }),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RenameOptions {
    input_directory: Utf8PathBuf,
    template_directory: Utf8PathBuf,
    bin_directory: Utf8PathBuf,
    recursion_depth: usize,
    move_mode: MoveMode,
    template: Option<FileOrName>,
    arguments: Vec<String>,
}

impl RenameOptions {
    pub fn input_directory(&self) -> &Utf8Path {
        self.input_directory.as_ref()
    }

    pub fn template_directory(&self) -> &Utf8Path {
        self.template_directory.as_ref()
    }

    pub fn bin_directory(&self) -> &Utf8Path {
        self.bin_directory.as_ref()
    }

    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    pub fn move_mode(&self) -> MoveMode {
        self.move_mode
    }

    pub fn template(&self) -> Option<&FileOrName> {
        self.template.as_ref()
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

impl TryFrom<(RenameArgs, &TFMTOptions)> for RenameOptions {
    type Error = color_eyre::Report;

    fn try_from(
        (rename_args, options): (RenameArgs, &TFMTOptions),
    ) -> Result<Self> {
        Ok(Self {
            input_directory: if let Some(input_directory) =
                rename_args.custom_input_directory
            {
                input_directory
            } else {
                current_dir_utf8()?
            },
            template_directory: if let Some(template_directory) =
                rename_args.custom_template_directory
            {
                template_directory
            } else {
                options.config_directory().to_owned()
            },
            bin_directory: if let Some(bin_directory) =
                rename_args.custom_bin_directory
            {
                bin_directory
            } else {
                options.config_directory().join(BIN_DIRECTORY_NAME)
            },
            recursion_depth: rename_args
                .recursion_depth
                .unwrap_or(DEFAULT_RECURSION_DEPTH),
            move_mode: if rename_args.always_copy {
                MoveMode::AlwaysCopy
            } else {
                MoveMode::Auto
            },
            template: rename_args.template,
            arguments: rename_args.arguments,
        })
    }
}
