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

pub(crate) fn seed(preview: bool, force: bool, config: &Config) -> Result<()> {
    let existing_files: Vec<&DefaultFile> = DEFAULT_FILES
        .iter()
        .filter(|file| config.path().join(file.name).exists())
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

    for file in &DEFAULT_FILES {
        let path = config.path().join(file.name);

        let pp = if preview { Config::PREVIEW_PREFIX } else { "" };

        if !preview {
            fs::write(path, file.content)?;
        }
        println!("{pp}Wrote default files to {}", config.path().display());
    }

    Ok(())
}
