use camino::Utf8PathBuf;
use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::Template;
use tfmttools_fs::{FsHandler, TemplateLoader};

use super::Command;
use crate::config::paths::AppPaths;
use crate::TERM;

#[derive(Debug)]
pub struct ListTemplatesCommand {
    template_directory: Utf8PathBuf,
}

impl ListTemplatesCommand {
    pub fn new(template_directory: Utf8PathBuf) -> Self {
        Self { template_directory }
    }

    fn format_template(template: &Template) -> String {
        let name = template.name();

        let string = if let Some(description) = template.description() {
            format!("{name}: {description}")
        } else {
            name.to_owned()
        };

        textwrap::fill(
            &string,
            Options::new(TERM.size().1 as usize)
                .subsequent_indent(&" ".repeat(name.len() + 2)),
        )
    }
}

impl Command for ListTemplatesCommand {
    fn run(
        &self,
        _app_paths: &AppPaths,
        _fs_handler: &FsHandler,
    ) -> Result<()> {
        let loader = TemplateLoader::read_directory(&self.template_directory)?;

        let all_templates = loader.get_all_templates();

        match all_templates.len() {
            0 => println!("Couldn't find any templates at {} or in the current directory.", self.template_directory),
            1 => println!("Found 1 template:"),
            other => println!("Found {other} templates:")
        }

        for template in all_templates {
            println!("{}", Self::format_template(&template));
        }

        Ok(())
    }
}
