use camino::Utf8PathBuf;

use super::Command;

#[derive(Debug)]
pub struct CopyTagsCommand {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl CopyTagsCommand {
    pub fn new(source: Utf8PathBuf, target: Utf8PathBuf) -> Self {
        Self { source, target }
    }
}

impl Command for CopyTagsCommand {
    fn run(
        &self,
        config: &crate::config::Config,
    ) -> color_eyre::eyre::Result<()> {
        todo!()
    }
}
