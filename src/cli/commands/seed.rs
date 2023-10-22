use crate::cli::config::DRY_RUN_PREFIX;
use crate::cli::Config;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;

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
        fs::remove_dir_all(config.config_dir())?;
    } else if config.config_dir().is_dir() {
        let has_files = config
            .config_dir()
            .read_dir()
            .map(|rd| rd.count() > 0)
            .unwrap_or(false);

        if has_files {
            return Err(eyre!(
                "Configuration folder already exists and is not empty: {}",
                config.config_dir().display()
            ));
        }
    }

    config.create_dir(config.config_dir())?;

    for file in &DEFAULT_FILES {
        let path = config.config_dir().join(file.name);

        let prefix = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

        if !config.dry_run() {
            fs::write(path, file.content)?;
        }

        println!(
            "{prefix}Wrote default files to {}",
            config.config_dir().display()
        );
    }

    Ok(())
}
