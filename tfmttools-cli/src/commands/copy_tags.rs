use camino::Utf8Path;
use color_eyre::Result;
use id3::Tag;

use crate::ui::ConfirmationPrompt;

pub fn copy_tags(
    source: &Utf8Path,
    target: &Utf8Path,
    yes: bool,
    dry_run: bool,
) -> Result<()> {
    let tag_source = Tag::read_from_path(source)?;

    let prompt_text = format!("Copy tags from {} to {}?", &source, &target);

    let confirmation_prompt = ConfirmationPrompt::new(&prompt_text);

    if yes || confirmation_prompt.prompt()? {
        if !dry_run {
            tag_source.write_to_path(target, tag_source.version())?;
        }

        println!("Copied tags.");
    }

    Ok(())
}
