use std::collections::HashMap;
use std::sync::LazyLock;

use camino::Utf8Path;
use fs_err as fs;
use minijinja::{Environment, Value, escape_formatter};
use regex::Regex;
use tfmttools_core::error::{TFMTError, TFMTResult};
use tfmttools_core::templates::{Frontmatter, Template};
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};

use crate::PathIterator;

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

const FRONTMATTER_FENCE: &str = "+++";

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
    ) -> TFMTResult<Self> {
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

    pub fn read_filename(path: &Utf8Path, name: &str) -> TFMTResult<Self> {
        let source = fs::read_to_string(path)?;

        Self::build([(name.to_owned(), source)])
    }

    pub fn read_script(script: &str) -> TFMTResult<Self> {
        Self::build([(Self::DEFAULT_SCRIPT_NAME.to_owned(), script.to_owned())])
    }

    /// Registers each `(name, source)` pair against a fresh [`Environment`]
    /// and assembles the resulting loader. Shared by all `read_*`
    /// constructors so the environment/frontmatter setup lives in one place.
    fn build(
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> TFMTResult<Self> {
        let mut template_names = Vec::new();
        let mut frontmatters = HashMap::new();
        let mut environment = Self::create_environment();

        for (name, source) in sources {
            Self::register_template(
                &mut environment,
                &mut frontmatters,
                &name,
                source,
            )?;

            template_names.push(name);
        }

        Ok(Self { template_names, frontmatters, environment })
    }

    pub fn get_template(
        &'_ self,
        name: &str,
        arguments: Vec<String>,
    ) -> TFMTResult<Option<Template<'_, '_>>> {
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
    ) -> TFMTResult<()> {
        let (body, frontmatter) = Self::split_frontmatter(name, source)?;

        if frontmatter.is_none() {
            Self::warn_on_deprecated_usage(name, &body);
        }

        if let Some(frontmatter) = frontmatter {
            frontmatters.insert(name.to_owned(), frontmatter);
        }

        environment.add_template_owned(name.to_owned(), body)?;

        Ok(())
    }

    fn split_frontmatter(
        label: &str,
        source: String,
    ) -> TFMTResult<(String, Option<Frontmatter>)> {
        // The regex crate doesn't support look-around, so the opening and
        // closing fences are matched with two separate anchored patterns
        // instead of one monolithic `open ... \r?\n ... close` capture. The
        // closing fence is found by searching for a line consisting solely
        // of `+++` (optionally followed by trailing spaces/tabs) starting
        // right after the opening fence. This lets the closing fence
        // immediately follow the opening fence's own newline when the
        // frontmatter block has no content (e.g. "+++\n+++\n"), since
        // `find_at` treats the position right after that newline as a valid
        // line start rather than requiring a second, independent `\r?\n`
        // between the two fences.
        static RE_OPENING_FENCE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\A\+\+\+[ \t]*\r?\n").unwrap());

        static RE_CLOSING_FENCE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?m)^\+\+\+[ \t]*\r?$").unwrap());

        if !source.starts_with(FRONTMATTER_FENCE) {
            return Ok((source, None));
        }

        let Some(opening) = RE_OPENING_FENCE.find(&source) else {
            return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
        };

        let Some(closing) = RE_CLOSING_FENCE.find_at(&source, opening.end())
        else {
            return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
        };

        let toml_text = &source[opening.end()..closing.start()];

        let frontmatter = Frontmatter::parse(toml_text, label)?;

        let mut body_start = closing.end();

        if let Some(rest) = source[body_start..].strip_prefix("\r\n") {
            body_start = source.len() - rest.len();
        } else if let Some(rest) = source[body_start..].strip_prefix('\n') {
            body_start = source.len() - rest.len();
        }

        let body = source[body_start..].to_owned();

        if Self::body_uses_indexed_args(&body) {
            return Err(TFMTError::IndexedArgsWithFrontmatter(
                label.to_owned(),
            ));
        }

        Ok((body, Some(frontmatter)))
    }

    fn body_uses_indexed_args(body: &str) -> bool {
        static RE_ARGS_INDEX: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\bargs\s*\[").unwrap());

        RE_ARGS_INDEX.is_match(body)
    }

    fn warn_on_deprecated_usage(label: &str, body: &str) {
        if Self::body_uses_indexed_args(body) {
            tracing::warn!(
                "Template '{label}' uses positional `args[N]` without frontmatter; declare arguments to migrate."
            );
        }

        if Self::description(body).is_some() {
            tracing::warn!(
                "Template '{label}' uses a leading comment as its description; move it to frontmatter's `description` field."
            );
        }
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
    use tfmttools_core::error::TFMTError;

    use super::*;

    #[test]
    fn split_frontmatter_returns_none_when_absent() {
        let source = "{{ artist }}/{{ title }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn split_frontmatter_parses_present_block() {
        let source = "+++\nname = \"Test\"\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert_eq!(frontmatter.unwrap().name(), Some("Test"));
    }

    #[test]
    fn split_frontmatter_handles_empty_toml_block() {
        let source = "+++\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert!(frontmatter.is_some());
        assert_eq!(frontmatter.unwrap().name(), None);
    }

    #[test]
    fn split_frontmatter_handles_empty_toml_block_crlf() {
        let source = "+++\r\n+++\r\n{{ artist }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert!(frontmatter.is_some());
        assert_eq!(frontmatter.unwrap().name(), None);
    }

    #[test]
    fn split_frontmatter_errors_when_unterminated() {
        let source = "+++\nname = \"Test\"\n{{ artist }}".to_owned();

        let error =
            TemplateLoader::split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::UnterminatedFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_errors_when_body_uses_indexed_args() {
        let source = "+++\nname = \"Test\"\n+++\n{{ args[0] }}".to_owned();

        let error =
            TemplateLoader::split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::IndexedArgsWithFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_allows_kwargs_identifier_with_frontmatter() {
        let source = "+++\nname = \"Test\"\n+++\n{{ kwargs[0] }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ kwargs[0] }}");
        assert!(frontmatter.is_some());
    }

    #[test]
    fn split_frontmatter_allows_indexed_args_without_frontmatter() {
        let source = "{{ args[0] }}".to_owned();

        let (body, frontmatter) =
            TemplateLoader::split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn read_script_populates_frontmatter_side_table() {
        let script = "+++\nname = \"Test\"\n+++\n{{ artist }}";

        let loader = TemplateLoader::read_script(script).unwrap();

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
        let loader = TemplateLoader::read_script("{{ args[0] }}").unwrap();

        assert!(loader.frontmatters.is_empty());
    }

    #[test]
    fn get_template_errors_on_missing_required_argument() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\", required = true }]\n+++\n{{ prefix }}";

        let loader = TemplateLoader::read_script(script).unwrap();

        let error = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap_err();

        assert!(matches!(error, TFMTError::MissingRequiredArgument(_, _, _)));
    }

    #[test]
    fn get_template_resolves_declared_arguments() {
        let script = "+++\nargs = [{ name = \"prefix\", type = \"string\" }]\n+++\n{{ prefix }}";

        let loader = TemplateLoader::read_script(script).unwrap();

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

        let loader = TemplateLoader::read_script(script).unwrap();

        let templates = loader.get_all_templates();

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].declared_args().len(), 1);
    }

    #[test]
    fn description_comes_only_from_frontmatter_when_present() {
        let script = "+++\ndescription = \"From frontmatter.\"\n+++\n{# Leading comment #}\n{{ artist }}";

        let loader = TemplateLoader::read_script(script).unwrap();
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
        let loader = TemplateLoader::read_script("{{ artist }}").unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.name(), TemplateLoader::DEFAULT_SCRIPT_NAME);
    }

    #[test]
    fn display_name_uses_frontmatter_override() {
        let script = "+++\nname = \"Pretty Name\"\n+++\n{{ artist }}";

        let loader = TemplateLoader::read_script(script).unwrap();
        let template = loader
            .get_template(TemplateLoader::DEFAULT_SCRIPT_NAME, Vec::new())
            .unwrap()
            .unwrap();

        assert_eq!(template.name(), "Pretty Name");
    }
}
