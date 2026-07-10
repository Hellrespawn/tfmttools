use std::collections::HashMap;
use std::sync::LazyLock;

use camino::Utf8Path;
use fs_err as fs;
use minijinja::{Environment, Value, escape_formatter};
use regex::Regex;
use tfmttools_core::templates::{Frontmatter, Template, parse_template_source};
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};
use tfmttools_core::warning::Warning;

use crate::PathIterator;
use crate::error::{FsError, FsResult};

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

#[derive(Debug)]
pub struct TemplateLoader<'tl> {
    template_names: Vec<String>,
    frontmatters: HashMap<String, Frontmatter>,
    environment: Environment<'tl>,
}

impl<'tl> TemplateLoader<'tl> {
    pub const DEFAULT_SCRIPT_NAME: &'static str = "script";

    pub fn read_directory(
        template_directory: &Utf8Directory,
    ) -> FsResult<(Self, Vec<Warning>)> {
        let iter = PathIterator::single_directory(template_directory.as_path())
            .flatten()
            .filter(|path| Self::path_is_template(path));

        let mut sources = Vec::new();

        for template_path in iter {
            let name = template_path
                .file_stem()
                .expect("Template::path_is_template should only return files.")
                .to_owned();

            let source = fs::read_to_string(&template_path)?;

            sources.push((name, source));
        }

        Self::build(sources)
    }

    pub fn read_filename(
        path: &Utf8Path,
        name: &str,
    ) -> FsResult<(Self, Vec<Warning>)> {
        let source = fs::read_to_string(path)?;

        Self::build([(name.to_owned(), source)])
    }

    pub fn read_script(script: &str) -> FsResult<(Self, Vec<Warning>)> {
        Self::build([(Self::DEFAULT_SCRIPT_NAME.to_owned(), script.to_owned())])
    }

    /// Registers each `(name, source)` pair against a fresh [`Environment`]
    /// and assembles the resulting loader. Shared by all `read_*`
    /// constructors so the environment/frontmatter setup lives in one place.
    fn build(
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> FsResult<(Self, Vec<Warning>)> {
        let mut template_names = Vec::new();
        let mut frontmatters = HashMap::new();
        let mut environment = Self::create_environment();
        let mut warnings = Vec::new();

        for (name, source) in sources {
            let template_warnings = Self::register_template(
                &mut environment,
                &mut frontmatters,
                &name,
                source,
            )?;

            warnings.extend(template_warnings);
            template_names.push(name);
        }

        Ok((Self { template_names, frontmatters, environment }, warnings))
    }

    pub fn get_template(
        &'_ self,
        name: &str,
        arguments: Vec<String>,
    ) -> FsResult<Option<Template<'_, '_>>> {
        let Ok(minijinja_template) = self.environment.get_template(name) else {
            return Ok(None);
        };

        let (display_name, description, frontmatter) =
            self.resolve_display_metadata(name, &minijinja_template);

        let template = Template::new(
            minijinja_template,
            name,
            display_name,
            description,
            arguments,
            frontmatter,
        )?;

        Ok(Some(template))
    }

    pub fn get_all_templates(&'_ self) -> Vec<Template<'_, '_>> {
        self.template_names
            .iter()
            .map(|name| {
                let minijinja_template = self.environment.get_template(name).expect(
                    "TemplateLoader::template_names should not contain names of non-existent templates.",
                );

                let (display_name, description, frontmatter) =
                    self.resolve_display_metadata(name, &minijinja_template);

                let declared_args = frontmatter
                    .map(|frontmatter| frontmatter.args().to_vec())
                    .unwrap_or_default();

                Template::for_display(
                    minijinja_template,
                    display_name,
                    description,
                    declared_args,
                )
            })
            .collect()
    }

    /// Resolves the display name, description, and frontmatter (if any) for
    /// a registered template, following the "frontmatter description never
    /// falls back to a leading comment" rule shared by `get_template` and
    /// `get_all_templates`.
    fn resolve_display_metadata(
        &self,
        name: &str,
        minijinja_template: &minijinja::Template<'_, '_>,
    ) -> (String, Option<String>, Option<&Frontmatter>) {
        let frontmatter = self.frontmatters.get(name);

        let description = match frontmatter {
            Some(frontmatter) => {
                frontmatter.description().map(ToOwned::to_owned)
            },
            None => Self::description(minijinja_template.source()),
        };

        let display_name = frontmatter
            .and_then(Frontmatter::name)
            .map_or_else(|| name.to_owned(), ToOwned::to_owned);

        (display_name, description, frontmatter)
    }

    fn register_template(
        environment: &mut Environment<'tl>,
        frontmatters: &mut HashMap<String, Frontmatter>,
        name: &str,
        source: String,
    ) -> FsResult<Vec<Warning>> {
        let (body, frontmatter, warnings) =
            parse_template_source(name, source)?;

        if let Some(frontmatter) = frontmatter {
            frontmatters.insert(name.to_owned(), frontmatter);
        }

        environment
            .add_template_owned(name.to_owned(), body)
            .map_err(|e| FsError::Core(e.into()))?;

        Ok(warnings)
    }

    fn description(source: &str) -> Option<String> {
        const COMMENT_START: &str = "{#";
        const COMMENT_END: &str = "#}";

        if source.trim().starts_with(COMMENT_START) {
            source.split_once(COMMENT_END).map(|(left, _)| {
                left.replace(COMMENT_START, "")
                    .replace(COMMENT_END, "")
                    .trim()
                    .to_owned()
            })
        } else {
            None
        }
    }

    fn path_is_template(path: &Utf8Path) -> bool {
        path.extension()
            .is_some_and(|string| TEMPLATE_EXTENSIONS.contains(&string))
    }

    fn create_environment() -> Environment<'tl> {
        let mut env = Environment::new();

        env.set_formatter(|out, state, value| {
            escape_formatter(
                out,
                state,
                if value.is_none() { &Value::UNDEFINED } else { value },
            )
        });

        env.add_filter("year", Self::year);
        env.add_filter("zero_pad", Self::zero_pad);

        env
    }

    fn year(date: &Value) -> Result<String, minijinja::Error> {
        static RE_ISO: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})-\d{2}-\d{2}").unwrap());

        static RE_AMBIGUOUS: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\d{2}-\d{2}-(\d{4})").unwrap());

        static RE_YEAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})").unwrap());

        let date = date.to_string();

        if let Some(m) = RE_ISO.find(&date) {
            let year = &m.as_str()[0..4];

            Ok(year.to_owned())
        } else if let Some(m) = RE_AMBIGUOUS.find(&date) {
            let string = m.as_str();

            let year = &string[string.len() - 4..string.len()];

            Ok(year.to_owned())
        } else if let Some(m) = RE_YEAR.find(&date) {
            Ok(m.as_str().to_owned())
        } else {
            Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                format!("Unable to parse date: {date}"),
            ))
        }
    }

    fn zero_pad(value: &Value, width: usize) -> String {
        format!("{value:0>width$}")
    }
}

#[cfg(test)]
mod tests {
    use tfmttools_core::warning::Warning;

    use super::*;
    use crate::error::FsError;

    #[test]
    fn read_script_without_frontmatter_using_indexed_args_returns_warning() {
        let (_, warnings) =
            TemplateLoader::read_script("{{ args[0] }}").unwrap();

        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            Warning::DeprecatedPositionalArgs { ref template }
            if template == TemplateLoader::DEFAULT_SCRIPT_NAME
        ));
    }

    #[test]
    fn read_script_without_frontmatter_with_leading_comment_returns_warning() {
        let (_, warnings) =
            TemplateLoader::read_script("{# A description #}\n{{ artist }}")
                .unwrap();

        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            Warning::DeprecatedLeadingComment { ref template }
            if template == TemplateLoader::DEFAULT_SCRIPT_NAME
        ));
    }

    #[test]
    fn read_script_with_frontmatter_returns_no_warnings() {
        let (_, warnings) = TemplateLoader::read_script(
            "+++\nname = \"Test\"\n+++\n{{ artist }}",
        )
        .unwrap();

        assert!(warnings.is_empty());
    }

    #[test]
    fn read_script_populates_frontmatter_side_table() {
        let script = "+++\nname = \"Test\"\n+++\n{{ artist }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();

        assert_eq!(
            loader
                .frontmatters
                .get(TemplateLoader::DEFAULT_SCRIPT_NAME)
                .unwrap()
                .name(),
            Some("Test")
        );
    }

    #[test]
    fn read_script_without_frontmatter_has_empty_side_table() {
        let (loader, _warnings) =
            TemplateLoader::read_script("{{ args[0] }}").unwrap();

        assert!(loader.frontmatters.is_empty());
    }

    #[test]
    fn get_template_errors_on_missing_required_argument() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\", required = true }]\n+++\n{{ prefix }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();

        let error = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap_err();

        assert!(matches!(
            error,
            FsError::Core(
                tfmttools_core::error::TFMTError::MissingRequiredArgument(
                    _,
                    _,
                    _
                )
            )
        ));
    }

    #[test]
    fn get_template_resolves_declared_arguments() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\" }]\n+++\n{{ prefix }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();

        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, vec![
                "a".to_owned(),
            ])
            .unwrap();

        assert!(template.is_some());
    }

    #[test]
    fn get_all_templates_never_errors_for_required_arguments() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\", required = true }]\n+++\n{{ prefix }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();

        let templates = loader.get_all_templates();

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].declared_args().len(), 1);
    }

    #[test]
    fn description_comes_only_from_frontmatter_when_present() {
        let script = "+++\ndescription = \"From frontmatter.\"\n+++\n{# Leading comment #}\n{{ artist }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(
            template.description(),
            Some(&"From frontmatter.".to_owned())
        );
    }

    #[test]
    fn display_name_falls_back_to_lookup_name_without_override() {
        let (loader, _warnings) =
            TemplateLoader::read_script("{{ artist }}").unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.name(), TemplateLoader::DEFAULT_SCRIPT_NAME);
    }

    #[test]
    fn display_name_uses_frontmatter_override() {
        let script = "+++\nname = \"Pretty Name\"\n+++\n{{ artist }}";

        let (loader, _warnings) = TemplateLoader::read_script(script).unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.name(), "Pretty Name");
    }
}
