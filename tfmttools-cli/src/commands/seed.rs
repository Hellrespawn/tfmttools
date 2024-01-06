use camino::Utf8PathBuf;
use color_eyre::eyre::eyre;
use color_eyre::Result;

use super::super::config::Config;
use super::Command;

struct DefaultFile {
    name: &'static str,
    content: &'static str,
}

static DEFAULT_FILES: [DefaultFile; 1] = [DefaultFile {
    name: "stef.tfmt",
    content: include_str!("../../../examples/stef.tfmt"),
}];

#[derive(Debug)]
pub struct Seed {
    template_directory: Utf8PathBuf,
    force: bool,
}

impl Seed {
    pub fn new(template_directory: Utf8PathBuf, force: bool) -> Self {
        Self { template_directory, force }
    }
}

impl Command for Seed {
    fn run(&self, config: &Config) -> Result<()> {
        let template_directory = &self.template_directory;

        if self.force {
            config.fs_handler().remove_dir_all(template_directory)?;
        } else if self.template_directory.is_dir() {
            let has_files = template_directory
                .read_dir()
                .map(|rd| rd.count() > 0)
                .unwrap_or(false);

            if has_files {
                return Err(eyre!(
                    "Template directory already exists and is not empty: {}",
                    template_directory
                ));
            }
        }

        config.fs_handler().create_dir(template_directory)?;

        for file in &DEFAULT_FILES {
            let path = template_directory.join(file.name);

            config.fs_handler().write(path, file.content)?;

            println!("Wrote default files to {template_directory}");
        }

        Ok(())
    }
}
