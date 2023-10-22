use crate::cli::ui;
use crate::template::Template;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const HISTORY_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const PREVIEW_PREFIX: &str = "[P] ";
pub(crate) const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];
pub(crate) const DEFAULT_PREVIEW_AMOUNT: usize = 8;
pub(crate) const DEFAULT_RECURSION_DEPTH: usize = 4;

pub(crate) struct Config {
    path: PathBuf,
}

impl Config {
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let config = Self {
            path: path.to_owned(),
        };

        Config::create_dir(&config.path)?;

        Ok(config)
    }

    pub(crate) fn default() -> Result<Self> {
        Self::new(&Self::default_path()?)
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

    pub(crate) fn path(&self) -> &Path {
        &self.path
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

    fn default_path() -> Result<PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(format!(".{}", env!("CARGO_PKG_NAME")));

            Ok(path)
        } else {
            Err(eyre!("Unable to read home directory!"))
        }
    }

    fn create_dir(path: &Path) -> Result<()> {
        if !path.exists() {
            fs::create_dir(path)?;
        } else if !path.is_dir() {
            return Err(eyre!("Unable to create configuration directory!"));
        }

        Ok(())
    }

    fn get_template_paths(&self) -> Result<Vec<PathBuf>> {
        let predicate: fn(&Path) -> bool = |p| {
            p.extension().map_or(false, |s| {
                TEMPLATE_EXTENSIONS.iter().any(|ext| s == *ext)
            })
        };

        let mut paths = Config::search_path(self.path(), 0, &predicate, None);
        paths.extend(Config::search_path(
            &std::env::current_dir()?,
            0,
            &predicate,
            None,
        ));

        Ok(paths)
    }
}
