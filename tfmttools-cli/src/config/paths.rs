use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::args::{Args, Subcommand};

#[derive(Debug)]
pub struct AppPaths {
    config_directory: Utf8PathBuf,
    bin_directory: Utf8PathBuf,
}

impl AppPaths {
    pub fn from_args(args: &Args) -> Result<Self> {
        let config_directory =
            Self::path_or_default(args.custom_config_directory.as_deref())?;

        let bin_directory = if let Subcommand::Rename(rename) = &args.command {
            rename.custom_bin_directory.clone()
        } else {
            None
        };

        let bin_directory = Self::path_or_subfolder_of_default(
            bin_directory.as_deref(),
            "bin",
        )?;

        Ok(Self { config_directory, bin_directory })
    }

    fn path_or_default(path: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
        if let Some(path) = path {
            Ok(path.to_owned())
        } else {
            Ok(Self::default_application_dir()?)
        }
    }

    fn path_or_subfolder_of_default(
        path: Option<&Utf8Path>,
        subfolder: &str,
    ) -> Result<Utf8PathBuf> {
        if let Some(path) = path {
            Ok(path.to_owned())
        } else {
            Ok(Self::default_application_dir()?.join(subfolder))
        }
    }

    pub fn default_application_dir() -> Result<Utf8PathBuf> {
        let project_dirs =
            directories::ProjectDirs::from("nl", "korpors", crate::PKG_NAME)
                .ok_or(eyre!("Unable to determine home directory."))?;

        let path = project_dirs.config_dir();

        Ok(Utf8PathBuf::try_from(path.to_owned())?)
    }

    pub fn config_directory(&self) -> &Utf8Path {
        self.config_directory.as_ref()
    }

    pub fn bin_directory(&self) -> &Utf8Path {
        self.bin_directory.as_ref()
    }

    #[allow(clippy::unused_self)]
    pub fn working_directory(&self) -> Result<Utf8PathBuf> {
        let path = std::env::current_dir()?;

        Ok(path.try_into()?)
    }

    pub fn history_file(&self) -> Utf8PathBuf {
        let filename = format!("{}.hist", crate::PKG_NAME);
        self.config_directory.join(filename)
    }
}
