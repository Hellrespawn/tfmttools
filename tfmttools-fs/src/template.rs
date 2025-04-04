use std::sync::LazyLock;

use camino::Utf8Path;
use fs_err as fs;
use minijinja::{Environment, Value, escape_formatter};
use regex::Regex;
use tfmttools_core::error::TFMTResult;
use tfmttools_core::templates::Template;
use tfmttools_core::util::Utf8Directory;

use crate::PathIterator;

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

#[derive(Debug)]
pub struct TemplateLoader<'tl> {
    template_names: Vec<String>,
    environment: Environment<'tl>,
}

impl<'tl> TemplateLoader<'tl> {
    pub fn read_directory(
        template_directory: &Utf8Directory,
    ) -> TFMTResult<Self> {
        let mut template_names = Vec::new();
        let mut environment = Self::create_environment();

        let iter = PathIterator::single_directory(template_directory.as_path())
            .flatten()
            .filter(|path| Self::path_is_template(path));

        for template_path in iter {
            let name = template_path
                .file_stem()
                .expect("Template::path_is_template should only return files.")
                .to_owned();

            template_names.push(name.clone());
            environment
                .add_template_owned(name, fs::read_to_string(template_path)?)?;
        }

        Ok(Self { template_names, environment })
    }

    pub fn read_filename(path: &Utf8Path, name: &str) -> TFMTResult<Self> {
        let mut environment = Self::create_environment();

        let template = fs::read_to_string(path)?;

        environment.add_template_owned(name.to_owned(), template)?;

        Ok(Self { template_names: vec![name.to_owned()], environment })
    }

    pub fn get_template(
        &self,
        name: &str,
        arguments: Vec<String>,
    ) -> Option<Template> {
        let minijinja_template: minijinja::Template<'_, '_> =
            self.environment.get_template(name).ok()?;

        let description = self.description(&minijinja_template);

        let template = Template::new(
            minijinja_template,
            name.to_owned(),
            description,
            arguments,
        );

        Some(template)
    }

    pub fn get_all_templates(&self) -> Vec<Template> {
        self.template_names
            .iter()
            .map(|name| self.get_template(name, Vec::new()).expect("Templates::template_names should not contain names of non-existent templates.")).collect()
    }

    fn description(&self, template: &minijinja::Template) -> Option<String> {
        let source = template.source();

        const COMMENT_START: &str = "{#";
        const COMMENT_END: &str = "{#";

        if source.trim().starts_with(COMMENT_START) {
            let option = source.split_once(COMMENT_END).map(|(left, _)| {
                left.replace(COMMENT_START, "")
                    .replace(COMMENT_END, "")
                    .trim()
                    .to_owned()
            });

            option
        } else {
            None
        }
    }

    fn path_is_template(path: &Utf8Path) -> bool {
        path.extension().is_some_and(|string| {
            TEMPLATE_EXTENSIONS.iter().any(|ext| string == *ext)
        })
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

    fn year(date: Value) -> Result<String, minijinja::Error> {
        let date = date.to_string();

        static RE_ISO: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})-\d{2}-\d{2}").unwrap());

        static RE_AMBIGUOUS: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\d{2}-\d{2}-(\d{4})").unwrap());

        static RE_YEAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})").unwrap());

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

    fn zero_pad(value: Value, width: usize) -> String {
        format!("{value:0>width$}")
    }
}
