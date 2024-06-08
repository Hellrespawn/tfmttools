use camino::Utf8PathBuf;
use id3::Tag;

use super::Command;
use crate::ui::ConfirmationPrompt;

#[derive(Debug)]
pub struct CopyTagsCommand {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
    yes: bool,
}

impl CopyTagsCommand {
    pub fn new(source: Utf8PathBuf, target: Utf8PathBuf, yes: bool) -> Self {
        Self { source, target, yes }
    }
}

impl Command for CopyTagsCommand {
    fn run(
        &self,
        config: &crate::config::Config,
    ) -> color_eyre::eyre::Result<()> {
        let tag_source = Tag::read_from_path(&self.source)?;

        let prompt_text =
            format!("Copy tags from {} to {}?", &self.source, &self.target);

        let confirmation_prompt = ConfirmationPrompt::new(&prompt_text);

        if self.yes || confirmation_prompt.prompt()? {
            if !config.dry_run() {
                tag_source.write_to_path(&self.target, tag_source.version())?;
            }

            println!("Copied tags.");
        }

        Ok(())
    }
}
