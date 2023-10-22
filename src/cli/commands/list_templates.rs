use crate::cli::Config;
use crate::template::Template;
use color_eyre::Result;

pub(crate) fn list_templates(config: &Config) -> Result<()> {
    let templates = config.get_templates()?;

    if templates.is_empty() {
        println!(
            "Couldn't find any templates at {} or in the current directory.",
            config.path().display()
        );
    } else {
        println!("Templates:");
    }

    for template in templates {
        print_template_info(&template);
    }

    Ok(())
}

fn print_template_info(script: &Template) {
    println!("  {}", script.name());
}
