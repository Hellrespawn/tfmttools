use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;

use crate::config::{Config, DRY_RUN_PREFIX};

struct DefaultFile {
    name: &'static str,
    content: &'static str,
}

static DEFAULT_FILES: [DefaultFile; 1] = [DefaultFile {
    name: "stef.tfmt",
    content: include_str!("../../../examples/stef.tfmt"),
}];

pub(crate) fn seed(config: &Config, force: bool) -> Result<()> {
    if force {
        crate::fs::remove_dir_all(
            config.dry_run(),
            config.template_directory(),
        )?;
    } else if config.template_directory().is_dir() {
        let has_files = config
            .template_directory()
            .read_dir()
            .map(|rd| rd.count() > 0)
            .unwrap_or(false);

        if has_files {
            return Err(eyre!(
                "Configuration folder already exists and is not empty: {}",
                config.template_directory()
            ));
        }
    }

    crate::fs::create_dir(config.dry_run(), config.template_directory())?;

    for file in &DEFAULT_FILES {
        let path = config.template_directory().join(file.name);

        let prefix = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

        if !config.dry_run() {
            fs::write(path, file.content)?;
        }

        println!(
            "{prefix}Wrote default files to {}",
            config.template_directory()
        );
    }

    Ok(())
}
