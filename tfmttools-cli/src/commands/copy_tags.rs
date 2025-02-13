use camino::Utf8PathBuf;
use color_eyre::Result;
use id3::Tag;
use tfmttools_fs::FsHandler;

use super::Command;
use crate::config::paths::AppPaths;
use crate::ui::ConfirmationPrompt;

#[derive(Debug)]
pub struct CopyTagsCommand {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
    yes: bool,
    dry_run: bool,
}

impl CopyTagsCommand {
    pub fn new(
        source: Utf8PathBuf,
        target: Utf8PathBuf,
        yes: bool,
        dry_run: bool,
    ) -> Self {
        Self { source, target, yes, dry_run }
    }
}

impl Command for CopyTagsCommand {
    fn run(
        &self,
        _app_paths: &AppPaths,
        _fs_handler: &FsHandler,
    ) -> Result<()> {
        let tag_source = Tag::read_from_path(&self.source)?;

        let prompt_text =
            format!("Copy tags from {} to {}?", &self.source, &self.target);

        let confirmation_prompt = ConfirmationPrompt::new(&prompt_text);

        if self.yes || confirmation_prompt.prompt()? {
            if !self.dry_run {
                tag_source.write_to_path(&self.target, tag_source.version())?;
            }

            println!("Copied tags.");
        }

        Ok(())
    }
}
