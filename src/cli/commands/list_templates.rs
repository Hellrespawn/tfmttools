use camino::Utf8PathBuf;
use clap::Args;
use color_eyre::Result;

use super::Command;
use crate::cli::config::{default_template_and_config_dir, Config};
use crate::cli::ui::table::Table;
use crate::template::{Template, Templates};

#[derive(Args, Debug)]
pub struct ListTemplates {
    #[arg(short, long, default_value_t = default_template_and_config_dir())]
    pub template_directory: Utf8PathBuf,
}

impl Command for ListTemplates {
    fn run(&self, _config: &Config) -> Result<()> {
        let templates = Templates::read_directory(&self.template_directory)?;

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

    fn override_dry_run(&mut self, _dry_run: bool) {
    }
}

impl ListTemplates {
    fn format_template(template: &Template) -> String {
        let name = template.name();

        if let Some(description) = template.description() {
            format!("{name}: {description}")
        } else {
            name.to_owned()
        }
    }
}
