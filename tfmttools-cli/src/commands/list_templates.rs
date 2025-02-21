use camino::Utf8Path;
use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::Template;
use tfmttools_fs::TemplateLoader;

use crate::TERM;

pub fn list_templates(template_directory: &Utf8Path) -> Result<()> {
    let loader = TemplateLoader::read_directory(template_directory)?;

    let all_templates = loader.get_all_templates();

    match all_templates.len() {
        0 => {
            println!(
                "Couldn't find any templates at {template_directory} or in the current directory."
            );
        },
        1 => println!("Found 1 template:"),
        other => println!("Found {other} templates:"),
    }

    for template in all_templates {
        println!("{}", format_template(&template));
    }

    Ok(())
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
