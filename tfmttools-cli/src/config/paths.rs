use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::args::{Args, Rename, Subcommand};

#[derive(Debug)]
pub struct AppPaths {
    config_directory: Utf8PathBuf,
    bin_directory: Utf8PathBuf,
}

impl AppPaths {
    pub fn from_args(args: &Args) -> Result<Self> {
        let config_directory =
            Self::path_or_default(args.custom_config_directory.as_deref())?;

        let bin_directory = if let Subcommand::Rename(Rename {
            custom_bin_directory: Some(custom_bin_directory),
            ..
        }) = &args.command
        {
            custom_bin_directory.clone()
        } else {
            config_directory.join("bin")
        };

        Ok(Self { config_directory, bin_directory })
    }

    fn path_or_default(path: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
        if let Some(path) = path {
            Ok(path.to_owned())
        } else {
            Ok(Self::default_application_dir()?)
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
