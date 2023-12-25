use camino::Utf8PathBuf;
use color_eyre::Result;
use tfmttools_core::templates::{Template, TemplateLoader};

use super::Command;
use crate::config::Config;
use crate::ui::table::Table;

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

        if let Some(description) = template.description() {
            format!("{name}: {description}")
        } else {
            name.to_owned()
        }
    }
}

impl Command for ListTemplates {
    fn run(&self, _config: &Config) -> Result<()> {
        let templates =
            TemplateLoader::read_directory(&self.template_directory)?;

        let all_templates = templates.get_all_templates();

        let mut table = Table::new();

        if all_templates.is_empty() {
            table.set_heading(format!(
                "Couldn't find any templates at {} or in the current directory.",
                self.template_directory
            ));
        } else {
            table.set_heading(format!(
                "Found {} templates",
                all_templates.len()
            ));
        }

        for template in all_templates {
            table.push_string(Self::format_template(&template));
        }

        println!("{table}");

        Ok(())
    }
}
