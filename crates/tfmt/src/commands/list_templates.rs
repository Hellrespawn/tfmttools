use color_eyre::Result;
use textwrap::Options;
use tfmttools_core::templates::{ArgSpec, Template};
use tfmttools_core::util::Utf8Directory;
use tfmttools_fs::TemplateLoader;

#[allow(unused_imports)]
use crate::ui::terminal_width;

#[cfg(test)]
mod test_utils {
    pub fn get_terminal_width() -> usize {
        120
    }
}

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

    #[cfg(test)]
    let width = test_utils::get_terminal_width();
    #[cfg(not(test))]
    let width = terminal_width();

    let header = textwrap::fill(
        &header_string,
        Options::new(width)
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

    format!("    {} ({}, {}){}", arg.name(), arg.kind(), requirement, description)
}

#[cfg(test)]
mod tests {
    use tfmttools_fs::TemplateLoader;

    use super::*;

    #[test]
    fn format_template_lists_declared_arguments() {
        let script = "+++\nname = \"Test Template\"\ndescription = \"A test template.\"\n\nargs = [\n    { name = \"prefix\", type = \"path\", required = true, description = \"Directory prefix.\" },\n    { name = \"suffix\", type = \"string\", required = false, default = \"\", description = \"Optional suffix.\" },\n]\n+++\n{{- prefix -}}{{- suffix -}}";

        let loader = TemplateLoader::read_script(script).unwrap();
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
        let loader = TemplateLoader::read_script("{{ artist }}").unwrap();
        let templates = loader.get_all_templates();

        let formatted = format_template(&templates[0]);

        assert_eq!(formatted, TemplateLoader::DEFAULT_SCRIPT_NAME);
    }
}
