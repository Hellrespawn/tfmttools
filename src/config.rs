use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;

pub(crate) const DRY_RUN_PREFIX: &str = "[D] ";

const DEFAULT_PREVIEW_AMOUNT: usize = 8;
const DEFAULT_RECURSION_DEPTH: usize = 4;

pub(crate) struct Config {
    template_directory: Utf8PathBuf,
    working_directory: Utf8PathBuf,
    history_file: Utf8PathBuf,
    dry_run: bool,
    recursion_depth: usize,
    preview_amount: usize,
}

impl Config {
    pub(crate) fn new(
        dry_run: bool,
        template_directory: &Utf8Path,
    ) -> Result<Self> {
        let config = Self {
            template_directory: template_directory.to_owned(),
            working_directory: std::env::current_dir()?.try_into()?,
            history_file: template_directory.join("history.json"),
            dry_run,
            recursion_depth: DEFAULT_RECURSION_DEPTH,
            preview_amount: DEFAULT_PREVIEW_AMOUNT,
        };

        Ok(config)
    }

    pub(crate) fn template_directory(&self) -> &Utf8Path {
        &self.template_directory
    }

    pub(crate) fn working_directory(&self) -> &Utf8Path {
        &self.working_directory
    }

    pub(crate) fn history_file(&self) -> &Utf8Path {
        &self.history_file
    }

    pub(crate) fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub(crate) fn dry_run_mut(&mut self) -> &mut bool {
        &mut self.dry_run
    }

    pub(crate) fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    pub(crate) fn preview_amount(&self) -> usize {
        self.preview_amount
    }

    pub(crate) fn with_recursion_depth(mut self, depth: Option<usize>) -> Self {
        self.recursion_depth = depth.unwrap_or(self.recursion_depth);
        self
    }

    pub(crate) fn default_path() -> Result<Utf8PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(format!(".{}", env!("CARGO_PKG_NAME")));

            Ok(path.try_into()?)
        } else {
            Err(eyre!("Unable to read home directory!"))
        }
    }
}
