use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::{ArgSpec, Template};
use tfmttools_core::util::Utf8Directory;
use tfmttools_core::warning::Warning;
use tfmttools_fs::TemplateLoader;

use crate::ui::terminal_width;

pub fn list_templates(template_directory: &Utf8Directory) -> Result<()> {
    let (loader, warnings) = TemplateLoader::read_directory(template_directory)?;

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

    if let Some(report) = format_template_warnings(&warnings) {
        println!();
        println!("{report}");
    }

    Ok(())
}

fn format_template_warnings(warnings: &[Warning]) -> Option<String> {
    if warnings.is_empty() {
        return None;
    }

    let lines: Vec<String> = warnings
        .iter()
        .filter_map(|w| match w {
            Warning::DeprecatedPositionalArgs { template } => Some(format!(
                "  \u{26a0} '{template}': uses positional args[N] without frontmatter; declare arguments to migrate."
            )),
            Warning::DeprecatedLeadingComment { template } => Some(format!(
                "  \u{26a0} '{template}': uses a leading comment as its description; move it to frontmatter's `description` field."
            )),
            Warning::WhitespaceInTag { .. } => None,
        })
        .collect();

    if lines.is_empty() {
        None
    } else {
        Some(format!("Warnings:\n{}", lines.join("\n")))
    }
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
    let requirement = if arg.required() {
        "required".to_owned()
    } else if let Some(default) = arg.default() {
        format!("default: {default:?}")
    } else {
        "optional".to_owned()
    };

    let description = arg
        .description()
        .map(|description| format!(" - {description}"))
        .unwrap_or_default();

    format!(
        "    {} ({}, {}){}",
        arg.name(),
        arg.kind(),
        requirement,
        description
    )
}

#[cfg(test)]
mod tests {
    use tfmttools_fs::TemplateLoader;

    use super::*;

    #[test]
    fn format_template_lists_declared_arguments() {
        let script = "+++\nname = \"Test Template\"\ndescription = \"A test template.\"\n\nargs = [\n    { name = \"prefix\", type = \"path\", required = true, description = \"Directory prefix.\" },\n    { name = \"suffix\", type = \"string\", required = false, default = \"\", description = \"Optional suffix.\" },\n]\n+++\n{{- prefix -}}{{- suffix -}}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();
        let templates = loader.get_all_templates();

        let formatted = format_template(&templates[0]);

        assert!(formatted.contains("Test Template: A test template."));
        assert!(formatted.contains("prefix"));
        assert!(formatted.contains("path"));
        assert!(formatted.contains("required"));
        assert!(formatted.contains("suffix"));
        assert!(formatted.contains("string"));
        assert!(formatted.contains("default"));
        assert!(formatted.contains("Optional suffix."));
    }

    #[test]
    fn format_template_without_args_has_no_arg_lines() {
        let (loader, _warnings) =
            TemplateLoader::read_script("{{ artist }}").unwrap();
        let templates = loader.get_all_templates();

        let formatted = format_template(&templates[0]);

        assert_eq!(formatted, TemplateLoader::DEFAULT_SCRIPT_NAME);
    }

    #[test]
    fn format_template_warnings_is_empty_for_modern_template() {
        let script = "+++\nname = \"Test Template\"\n+++\n{{ artist }}";
        let (_, warnings) = TemplateLoader::read_script(script).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn format_template_warnings_contains_deprecated_template_name() {
        let script = "{{ args[0] }}";
        let (_, warnings) = TemplateLoader::read_script(script).unwrap();
        assert!(!warnings.is_empty());
    }
}
