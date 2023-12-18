use camino::Utf8PathBuf;
use clap::Args;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;

use super::super::config::{Config, DRY_RUN_PREFIX};
use super::Command;
use crate::cli::config::default_template_and_config_dir;

// TODO? Interactive screen for seed?

struct DefaultFile {
    name: &'static str,
    content: &'static str,
}

static DEFAULT_FILES: [DefaultFile; 1] = [DefaultFile {
    name: "stef.tfmt",
    content: include_str!("../../../examples/stef.tfmt"),
}];

#[derive(Args, Debug)]
pub struct Seed {
    #[arg(short, long, default_value_t = default_template_and_config_dir())]
    template_directory: Utf8PathBuf,

    #[arg(short, long)]
    dry_run: bool,

    #[arg(short, long)]
    force: bool,
}

impl Command for Seed {
    fn run(&self, _config: &Config) -> Result<()> {
        let template_directory = &self.template_directory;

        if self.force {
            crate::fs::remove_dir_all(self.dry_run, template_directory)?;
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

        crate::fs::create_dir(self.dry_run, template_directory)?;

        for file in &DEFAULT_FILES {
            let path = template_directory.join(file.name);

            if self.dry_run {
                print!("{DRY_RUN_PREFIX}");
            } else {
                fs::write(path, file.content)?;
            }

            println!("Wrote default files to {template_directory}");
        }

        Ok(())
    }
}
