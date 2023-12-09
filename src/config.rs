use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;

pub const DRY_RUN_PREFIX: &str = "[D] ";

const DEFAULT_RECURSION_DEPTH: usize = 4;
const DEFAULT_HISTORY_FILENAME: &str =
    concat!(env!("CARGO_PKG_NAME"), ".history.json");

#[derive(Debug)]
pub struct Config {
    config_and_template_directory: Utf8PathBuf,
    working_directory: Utf8PathBuf,
    history_file: Utf8PathBuf,
    dry_run: bool,
    force: bool,
    recursion_depth: usize,
}

impl Config {
    pub fn new(
        dry_run: bool,
        force: bool,
        config_and_template_directory: &Utf8Path,
    ) -> Result<Self> {
        let config = Self {
            config_and_template_directory: config_and_template_directory
                .to_owned(),
            working_directory: std::env::current_dir()?.try_into()?,
            history_file: config_and_template_directory
                .join(DEFAULT_HISTORY_FILENAME),
            dry_run,
            force,
            recursion_depth: DEFAULT_RECURSION_DEPTH,
        };

        Ok(config)
    }

    pub fn config_and_template_directory(&self) -> &Utf8Path {
        &self.config_and_template_directory
    }

    pub fn working_directory(&self) -> &Utf8Path {
        &self.working_directory
    }

    pub fn history_file(&self) -> &Utf8Path {
        &self.history_file
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn dry_run_mut(&mut self) -> &mut bool {
        &mut self.dry_run
    }

    pub fn force(&self) -> bool {
        self.force
    }

    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    pub fn set_recursion_depth(&mut self, depth: Option<usize>) {
        self.recursion_depth = depth.unwrap_or(DEFAULT_RECURSION_DEPTH);
    }

    pub fn default_path() -> Result<Utf8PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(format!(".{}", env!("CARGO_PKG_NAME")));

            Ok(path.try_into()?)
        } else {
            Err(eyre!("Unable to read home directory!"))
        }
    }
}
