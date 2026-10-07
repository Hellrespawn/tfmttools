use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::{ArgKind, ArgSpec, Template};
use tfmttools_core::util::Utf8Directory;
use tfmttools_fs::TemplateLoader;

use crate::ui::terminal_width;

pub fn list_templates(template_directory: &Utf8Directory) -> Result<()> {
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

    let header_string = if let Some(description) = template.description() {
        format!("{name}: {description}")
    } else {
        name.to_owned()
    };

    let header = textwrap::fill(
        &header_string,
        Options::new(terminal_width())
            .subsequent_indent(&" ".repeat(name.len() + 2)),
    );

    let arg_lines: Vec<String> =
        template.declared_args().iter().map(format_arg).collect();

    if arg_lines.is_empty() {
        header
    } else {
        format!("{header}\n{}", arg_lines.join("\n"))
    }
}

fn format_arg(arg: &ArgSpec) -> String {
    let requirement = arg.default.as_ref().map_or_else(
        || "required".to_owned(),
        |default| format!("default: {default:?}"),
    );
    let description = arg
        .description
        .as_ref()
        .map(|description| format!(" - {description}"))
        .unwrap_or_default();
    let kind = match arg.kind {
        ArgKind::String => "string",
        ArgKind::Int => "int",
        ArgKind::Path => "path",
    };
    format!("    {} ({kind}, {requirement}){description}", arg.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_metadata_and_argument_requirements_without_binding() {
        let loader = TemplateLoader::read_script(
            r#"
            name: "Test Template"
            description: "A test template."
            arg prefix: path(description: "Directory prefix.")
            arg suffix: string(default: "", description: "Optional suffix.")
            path: ({prefix} {$title} {suffix})
        "#,
        )
        .unwrap();
        let formatted = format_template(&loader.get_all_templates()[0]);
        assert!(formatted.contains("Test Template: A test template."));
        assert!(formatted.contains("prefix (path, required)"));
        assert!(formatted.contains("suffix (string, default: \"\")"));
        assert!(formatted.contains("Directory prefix."));
        assert!(formatted.contains("Optional suffix."));
    }

    #[test]
    fn listing_uses_lookup_name_without_metadata() {
        let loader = TemplateLoader::read_script(r#"path: ("Song")"#).unwrap();
        assert_eq!(format_template(&loader.get_all_templates()[0]), "script");
    }
}
