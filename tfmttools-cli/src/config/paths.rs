use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::args::Args;

#[derive(Debug)]
pub struct AppPaths {
    config_directory: Utf8PathBuf,
}

impl AppPaths {
    pub fn from_args(args: &Args) -> Result<Self> {
        let config_directory =
            if let Some(config_directory) = &args.custom_config_directory {
                Ok(config_directory.to_owned())
            } else {
                Self::default_template_and_config_dir()
            }?;

        Ok(Self { config_directory })
    }

    pub fn default_template_and_config_dir() -> Result<Utf8PathBuf> {
        let home = dirs::home_dir()
            .ok_or(eyre!("Unable to determine home directory."))?;

        let path = home.join(format!(".{}", crate::PKG_NAME));

        Ok(path.clone().try_into()?)
    }

    pub fn config_directory(&self) -> &Utf8Path {
        self.config_directory.as_ref()
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
