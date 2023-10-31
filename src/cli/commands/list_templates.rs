use color_eyre::Result;

use crate::cli::ui::table::Table;
use crate::config::Config;
use crate::template::Template;

pub(crate) fn list_templates(config: &Config) -> Result<()> {
    let templates = config.get_templates()?;

    let mut table = Table::new();

    if templates.is_empty() {
        table.set_heading(format!(
            "Couldn't find any templates at {} or in the current directory.",
            config.directory()
        ));
    } else {
        table.set_heading(format!("Found {} templates", templates.len()));
    }

    for template in templates {
        table.push_string(format_template(&template)?);
    }

    println!("{table}");

    Ok(())
}

fn format_template(template: &Template) -> Result<String> {
    let name = template.name();

    if let Some(description) = template.description()? {
        Ok(format!("{name}: {description}"))
    } else {
        Ok(name.to_owned())
    }
}
