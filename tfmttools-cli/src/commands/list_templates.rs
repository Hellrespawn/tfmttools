use camino::Utf8PathBuf;
use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::{Template, TemplateLoader};

use super::Command;
use crate::{config::Config, TERM};

#[derive(Debug)]
pub struct ListTemplates {
    template_directory: Utf8PathBuf,
}

impl ListTemplates {
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

impl Command for ListTemplates {
    fn run(&self, _config: &Config) -> Result<()> {
        let templates =
            TemplateLoader::read_directory(&self.template_directory)?;

        let all_templates = templates.get_all_templates();

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
