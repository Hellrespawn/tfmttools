use camino::Utf8PathBuf;
use clap::Args;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;

use super::super::config::{Config, DRY_RUN_PREFIX};
use super::Command;
use crate::cli::config::default_template_and_config_dir;

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
    fn run(&self, config: &Config) -> Result<()> {
        if self.force {
            crate::fs::remove_dir_all(self.dry_run, &config.config_directory)?;
        } else if config.config_directory.is_dir() {
            let has_files = config
                .config_directory
                .read_dir()
                .map(|rd| rd.count() > 0)
                .unwrap_or(false);

            if has_files {
                return Err(eyre!(
                    "Configuration folder already exists and is not empty: {}",
                    config.config_directory
                ));
            }
        }

        crate::fs::create_dir(self.dry_run, &config.config_directory)?;

        for file in &DEFAULT_FILES {
            let path = config.config_directory.join(file.name);

            if self.dry_run {
                print!("{DRY_RUN_PREFIX}");
            } else {
                fs::write(path, file.content)?;
            }

            println!("Wrote default files to {}", config.config_directory);
        }

        Ok(())
    }

    fn override_dry_run(&mut self, dry_run: bool) {
        if dry_run {
            self.dry_run = true;
        }
    }
}
