use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;

use crate::fs::PathIterator;
use crate::template::Template;

pub(crate) const HISTORY_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const DRY_RUN_PREFIX: &str = "[D] ";

const DEFAULT_PREVIEW_AMOUNT: usize = 8;
const DEFAULT_RECURSION_DEPTH: usize = 4;

pub(crate) struct Config {
    directory: Utf8PathBuf,
    current_dir: Utf8PathBuf,
    dry_run: bool,
    recursion_depth: usize,
    preview_amount: usize,
}

impl Config {
    pub(crate) fn new(directory: &Utf8Path, dry_run: bool) -> Result<Self> {
        let config = Self {
            directory: directory.to_owned(),
            current_dir: std::env::current_dir()?.try_into()?,
            dry_run,
            recursion_depth: DEFAULT_RECURSION_DEPTH,
            preview_amount: DEFAULT_PREVIEW_AMOUNT,
        };

        Ok(config)
    }

    pub(crate) fn directory(&self) -> &Utf8Path {
        &self.directory
    }

    pub(crate) fn current_dir(&self) -> &Utf8Path {
        &self.current_dir
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

        let found_templates: Vec<Template> = templates
            .into_iter()
            .filter(|template| template.name() == name)
            .collect();

        let length = found_templates.len();

        if length == 0 {
            let path = Utf8PathBuf::from(name);

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

    pub(crate) fn default_path() -> Result<Utf8PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(format!(".{}", env!("CARGO_PKG_NAME")));

            Ok(path.try_into()?)
        } else {
            Err(eyre!("Unable to read home directory!"))
        }
    }

    pub(crate) fn create_dir(&self, path: &Utf8Path) -> Result<()> {
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

    fn get_template_paths(&self) -> Result<Vec<Utf8PathBuf>> {
        let mut paths =
            PathIterator::new(self.directory()).collect::<Result<Vec<_>>>()?;

        let cwd: Utf8PathBuf = std::env::current_dir()?.try_into()?;

        paths.extend(PathIterator::new(&cwd).collect::<Result<Vec<_>>>()?);

        Ok(paths.into_iter().filter(|p| Template::path_predicate(p)).collect())
    }
}
