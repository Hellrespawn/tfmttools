use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::{bail, eyre};
use tfmttools_core::util::{
    FSMode, MoveMode, Utf8Directory, Utf8File, Utf8PathExt,
};
use tfmttools_fs::{FileOrName, verify_directory, verify_file};

use crate::cli::args::{
    RenameArgs, TFMTArgs, TemplateArgs, ValidateCommonArgs,
};
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

        Ok(verify_directory(utf8_path)?)
    }

    pub fn history_file_path(&self) -> Result<Utf8File> {
        let filename = format!("{}.hist", crate::PKG_NAME);
        let path = self.config_directory.as_path().join(filename);

        Ok(verify_file(path)?)
    }

    fn path_or_default(path: Option<&Utf8Path>) -> Result<Utf8Directory> {
        if let Some(path) = path {
            Ok(verify_directory(path)?)
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
    InlineTemplate(String),
}

impl From<TemplateArgs> for TemplateOption {
    fn from(template_args: TemplateArgs) -> Self {
        match (template_args.template, template_args.inline_template) {
            (None, None) => Self::None,
            (None, Some(template)) => Self::InlineTemplate(template),
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

#[derive(Debug, Clone)]
pub struct ValidateOptions {
    input_directory: Utf8Directory,
    recursion_depth: usize,
}

impl ValidateOptions {
    #[must_use]
    pub fn input_directory(&self) -> &Utf8Directory {
        &self.input_directory
    }

    #[must_use]
    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }
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

fn resolve_input_scope(
    input: Option<Utf8PathBuf>,
    depth: Option<usize>,
) -> Result<(Utf8Directory, usize)> {
    let depth = depth.unwrap_or(DEFAULT_RECURSION_DEPTH);
    if depth > 256 {
        bail!("Recursion depth must not exceed 256");
    }
    let cwd = current_dir_utf8()?;
    let directory = match input {
        Some(path) => verify_directory(cwd.join(path))?,
        None => cwd,
    };
    Ok((directory, depth))
}

impl TryFrom<ValidateCommonArgs> for ValidateOptions {
    type Error = color_eyre::Report;

    fn try_from(validate_args: ValidateCommonArgs) -> Result<Self> {
        let (input_directory, recursion_depth) = resolve_input_scope(
            validate_args.custom_input_directory,
            validate_args.recursion_depth,
        )?;
        Ok(Self { input_directory, recursion_depth })
    }
}

impl TryFrom<(RenameArgs, &TFMTOptions)> for RenameOptions {
    type Error = color_eyre::Report;

    fn try_from(
        (rename_args, options): (RenameArgs, &TFMTOptions),
    ) -> Result<Self> {
        let (input_directory, recursion_depth) = resolve_input_scope(
            rename_args.custom_input_directory,
            rename_args.recursion_depth,
        )?;
        Ok(Self {
            input_directory,
            template_directory: if let Some(template_directory) =
                rename_args.custom_template_directory
            {
                verify_directory(template_directory)?
            } else {
                options.config_directory().to_owned()
            },
            bin_directory: if let Some(bin_directory) =
                rename_args.custom_bin_directory
            {
                verify_directory(bin_directory)?
            } else {
                verify_directory(
                    options
                        .config_directory()
                        .as_path()
                        .join(BIN_DIRECTORY_NAME),
                )?
            },
            recursion_depth,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn validation_scope(
        input: Option<Utf8PathBuf>,
        depth: Option<usize>,
    ) -> ValidateOptions {
        ValidateOptions::try_from(ValidateCommonArgs {
            custom_input_directory: input,
            recursion_depth: depth,
        })
        .unwrap()
    }

    #[test]
    fn relative_input_is_absolute_without_canonicalization() {
        let options =
            validation_scope(Some("missing scope/../songs".into()), None);
        assert_eq!(
            options.input_directory().as_path(),
            current_dir_utf8()
                .unwrap()
                .join("missing scope/../songs")
                .as_path()
        );
    }

    #[test]
    fn absolute_and_default_input_scopes() {
        let cwd = current_dir_utf8().unwrap();
        assert_eq!(validation_scope(None, None).input_directory(), &cwd);
        let path = cwd.join("missing absolute scope").into_path_buf();
        assert_eq!(
            validation_scope(Some(path.clone()), None)
                .input_directory()
                .as_path(),
            path
        );
    }

    #[test]
    fn depth_zero_and_missing_paths() {
        assert_eq!(
            validation_scope(Some("missing scope".into()), None)
                .recursion_depth(),
            4
        );
        assert_eq!(
            validation_scope(Some("missing scope".into()), Some(0))
                .recursion_depth(),
            0
        );
        assert!(
            ValidateOptions::try_from(ValidateCommonArgs {
                custom_input_directory: Some("Cargo.toml".into()),
                recursion_depth: None,
            })
            .is_err()
        );
    }
}
