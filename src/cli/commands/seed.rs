use crate::cli::config::DRY_RUN_PREFIX;
use crate::cli::Config;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::fs;

struct DefaultFile {
    name: &'static str,
    content: &'static str,
}

static DEFAULT_FILES: [DefaultFile; 1] = [DefaultFile {
    name: "sync.tfmt",
    content: include_str!("../../../examples/sync.tfmt"),
}];

pub(crate) fn seed(config: &Config, force: bool) -> Result<()> {
    let existing_files: Vec<&DefaultFile> = DEFAULT_FILES
        .iter()
        .filter(|file| config.config_dir().join(file.name).exists())
        .collect();

    if !force && !existing_files.is_empty() {
        return Err(eyre!(
            "The following files already exist:\n{}",
            existing_files
                .iter()
                .map(|f| f.name)
                .collect::<Vec<&str>>()
                .join("\n")
        ));
    }

    Config::create_dir(config.config_dir())?;

    for file in &DEFAULT_FILES {
        let path = config.config_dir().join(file.name);

        let pp = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

        if !config.dry_run() {
            fs::write(path, file.content)?;
        }

        println!(
            "{pp}Wrote default files to {}",
            config.config_dir().display()
        );
    }

    Ok(())
}
