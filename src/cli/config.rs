use super::Args;
use crate::cli::ui;
use crate::template::Template;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const HISTORY_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const DRY_RUN_PREFIX: &str = "[D] ";
pub(crate) const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

const DEFAULT_PREVIEW_AMOUNT: usize = 8;
const DEFAULT_RECURSION_DEPTH: usize = 4;

pub(crate) struct Config {
    config_dir: PathBuf,
    current_dir: PathBuf,
    dry_run: bool,
    recursion_depth: usize,
    preview_amount: usize,
}

impl Config {
    pub(crate) fn new(config_dir: &Path, dry_run: bool) -> Result<Self> {
        let config = Self {
            config_dir: config_dir.to_owned(),
            current_dir: std::env::current_dir()?,
            dry_run,
            recursion_depth: DEFAULT_RECURSION_DEPTH,
            preview_amount: DEFAULT_PREVIEW_AMOUNT,
        };

        Ok(config)
    }

    pub(crate) fn from_args(args: &Args) -> Result<Self> {
        let path = if let Some(path) = &args.config {
            path.clone()
        } else {
            Self::default_path()?
        };

        let dry_run = args.dry_run();

        Self::new(&path, dry_run)
    }

    pub(crate) fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub(crate) fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    pub(crate) fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub(crate) fn recursion_depth(&self) -> usize {
        self.recursion_depth
    }

    pub(crate) fn preview_amount(&self) -> usize {
        self.preview_amount
    }

    pub(crate) fn aggregate_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = self.dry_run || dry_run;
        self
    }

    pub(crate) fn with_recursion_depth(mut self, depth: Option<usize>) -> Self {
        self.recursion_depth = depth.unwrap_or(self.recursion_depth);
        self
    }

    /// Search a path for files matching `predicate`, recursing for `depth`.
    pub(crate) fn search_path<P>(
        path: &Path,
        depth: usize,
        predicate: &P,
        spinner: Option<&ui::AudioFileSpinner>,
    ) -> Vec<PathBuf>
    where
        P: Fn(&Path) -> bool,
    {
        let mut found_paths = Vec::new();

        if let Ok(iter) = fs::read_dir(path) {
            for entry in iter.flatten() {
                let entry_path = entry.path();

                let matches_predicate = predicate(&entry_path);

                if entry_path.is_file() {
                    if let Some(spinner) = spinner {
                        spinner.inc_total();
                    }

                    if matches_predicate {
                        if let Some(spinner) = spinner {
                            spinner.inc_found();
                        }
                        found_paths.push(entry_path);

                        #[cfg(debug_assertions)]
                        crate::debug::delay();
                    }
                } else if entry_path.is_dir() && depth > 0 {
                    found_paths.extend(Config::search_path(
                        &entry_path,
                        depth - 1,
                        predicate,
                        spinner,
                    ));
                }
            }
        }

        found_paths
    }

    pub(crate) fn get_templates(&self) -> Result<Vec<Template>> {
        let paths = self.get_template_paths()?;

        let mut templates = Vec::new();

        for path in paths {
            templates.push(Template::from_file(&path)?);
        }

        Ok(templates)
    }

    pub(crate) fn get_template(&self, name: &str) -> Result<Template> {
        let templates = self.get_templates()?;

        let found_templates: Vec<Template> =
            templates.into_iter().filter(|s| s.name() == name).collect();

        let length = found_templates.len();

        if length == 0 {
            let path = PathBuf::from(name);

            if path.is_file() {
                Ok(Template::from_file(&path)?)
            } else {
                Err(eyre!("Unable to find template \"{}\"", name))
            }
        } else if length > 1 {
            Err(eyre!("Found {} templates with name \"{}\"", length, name))
        } else {
            let template = found_templates.into_iter().next();

            // This unwrap is always safe, as we check the length manually.
            debug_assert!(template.is_some());

            Ok(template.unwrap())
        }
    }

    pub(crate) fn default_path() -> Result<PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(format!(".{}", env!("CARGO_PKG_NAME")));

            Ok(path)
        } else {
            Err(eyre!("Unable to read home directory!"))
        }
    }

    pub(crate) fn create_dir(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            Ok(())
        } else if !path.exists() {
            Ok(fs::create_dir(path)?)
        } else if !path.is_dir() {
            Err(eyre!("Unable to create configuration directory!"))
        } else {
            Ok(())
        }
    }

    fn get_template_paths(&self) -> Result<Vec<PathBuf>> {
        let predicate: fn(&Path) -> bool = |p| {
            p.extension().map_or(false, |s| {
                TEMPLATE_EXTENSIONS.iter().any(|ext| s == *ext)
            })
        };

        let mut paths =
            Config::search_path(self.config_dir(), 0, &predicate, None);
        paths.extend(Config::search_path(
            &std::env::current_dir()?,
            0,
            &predicate,
            None,
        ));

        Ok(paths)
    }
}
