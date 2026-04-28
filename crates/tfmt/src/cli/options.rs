use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::util::{
    FSMode, MoveMode, Utf8Directory, Utf8File, Utf8PathExt,
};
use tfmttools_fs::FileOrName;

use crate::cli::args::{RenameArgs, TFMTArgs, TemplateArgs};
use crate::ui::{PreviewListSize, current_dir_utf8, terminal_height};

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
    config_directory: Utf8Directory,
    fs_mode: FSMode,
    display_mode: DisplayMode,
    confirm_mode: ConfirmMode,
    verbosity: u8,
    preview_list_size: PreviewListSize,
    run_id: String,
}

impl TFMTOptions {
    #[must_use]
    pub fn config_directory(&self) -> &Utf8Directory {
        &self.config_directory
    }

    #[must_use]
    pub fn fs_mode(&self) -> FSMode {
        self.fs_mode
    }

    #[must_use]
    pub fn display_mode(&self) -> DisplayMode {
        self.display_mode
    }

    #[must_use]
    pub fn confirm_mode(&self) -> ConfirmMode {
        self.confirm_mode
    }

    #[must_use]
    pub fn preview_list_size(&self) -> PreviewListSize {
        self.preview_list_size
    }

    #[must_use]
    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }

    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}

impl TFMTOptions {
    pub fn default_application_dir() -> Result<Utf8Directory> {
        let path = dirs::home_dir()
            .ok_or(eyre!("Unable to determine home directory."))?
            .join(format!(".{}", crate::PKG_NAME));

        let utf8_path = Utf8PathBuf::try_from(path)?;

        Ok(Utf8Directory::new(utf8_path)?)
    }

    pub fn history_file_path(&self) -> Result<Utf8File> {
        let filename = format!("{}.hist", crate::PKG_NAME);
        let path = self.config_directory.as_path().join(filename);

        Ok(Utf8File::new(path)?)
    }

    fn path_or_default(path: Option<&Utf8Path>) -> Result<Utf8Directory> {
        if let Some(path) = path {
            Ok(Utf8Directory::new(path)?)
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
            fs_mode: if args.dry_run {
                FSMode::DryRun
            } else {
                FSMode::Default
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
pub enum TemplateOption {
    None,
    FileOrName(FileOrName),
    Script(String),
}

impl From<TemplateArgs> for TemplateOption {
    fn from(template_args: TemplateArgs) -> Self {
        match (template_args.template, template_args.script) {
            (None, None) => Self::None,
            (None, Some(script)) => Self::Script(script),
            (Some(file_or_name), None) => Self::FileOrName(file_or_name),
            (Some(_), Some(_)) => {
                unreachable!("Mutual exclusion should be guaranteed by clap.")
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenameOptions {
    input_directory: Utf8Directory,
    template_directory: Utf8Directory,
    bin_directory: Utf8Directory,
    recursion_depth: usize,
    move_mode: MoveMode,
    template_option: TemplateOption,
    arguments: Vec<String>,
}

impl RenameOptions {
    #[must_use]
    pub fn input_directory(&self) -> &Utf8Directory {
        &self.input_directory
    }

    #[must_use]
    pub fn template_directory(&self) -> &Utf8Directory {
        &self.template_directory
    }

    #[must_use]
    pub fn bin_directory(&self) -> &Utf8Directory {
        &self.bin_directory
    }

    #[must_use]
    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    #[must_use]
    pub fn move_mode(&self) -> MoveMode {
        self.move_mode
    }

    #[must_use]
    pub fn template_option(&self) -> &TemplateOption {
        &self.template_option
    }

    #[must_use]
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
                Utf8Directory::new(input_directory)?
            } else {
                current_dir_utf8()?
            },
            template_directory: if let Some(template_directory) =
                rename_args.custom_template_directory
            {
                Utf8Directory::new(template_directory)?
            } else {
                options.config_directory().to_owned()
            },
            bin_directory: if let Some(bin_directory) =
                rename_args.custom_bin_directory
            {
                Utf8Directory::new(bin_directory)?
            } else {
                Utf8Directory::new(
                    options
                        .config_directory()
                        .as_path()
                        .join(BIN_DIRECTORY_NAME),
                )?
            },
            recursion_depth: rename_args
                .recursion_depth
                .unwrap_or(DEFAULT_RECURSION_DEPTH),
            move_mode: if rename_args.always_copy {
                MoveMode::AlwaysCopy
            } else {
                MoveMode::Auto
            },
            template_option: rename_args.template_args.into(),
            arguments: rename_args.arguments,
        })
    }
}
