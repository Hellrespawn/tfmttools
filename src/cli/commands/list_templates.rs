use color_eyre::Result;

use crate::cli::ui::table::Table;
use crate::config::Config;
use crate::template::{Template, Templates};

pub(crate) fn list_templates(config: &Config) -> Result<()> {
    let templates =
        Templates::read_directory(config.config_and_template_directory())?;

    let all_templates = templates.get_all_templates();

    let mut table = Table::new();

    if all_templates.is_empty() {
        table.set_heading(format!(
            "Couldn't find any templates at {} or in the current directory.",
            config.config_and_template_directory()
        ));
    } else {
        table.set_heading(format!("Found {} templates", all_templates.len()));
    }

    for template in all_templates {
        table.push_string(format_template(&template));
    }

    println!("{table}");

    Ok(())
}

fn format_template(template: &Template) -> String {
    let name = template.name();

    if let Some(description) = template.description() {
        format!("{name}: {description}")
    } else {
        name.to_owned()
    }
}
