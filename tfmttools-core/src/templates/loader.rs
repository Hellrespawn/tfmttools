use camino::Utf8Path;
use color_eyre::Result;
use fs_err as fs;
use minijinja::{escape_formatter, Environment, Value};
use once_cell::sync::Lazy;
use regex::Regex;

use super::{Template, TEMPLATE_EXTENSIONS};
use crate::fs::PathIterator;

#[derive(Debug)]
pub struct TemplateLoader<'tl> {
    template_names: Vec<String>,
    environment: Environment<'tl>,
}

impl<'tl> TemplateLoader<'tl> {
    pub fn read_directory(template_directory: &Utf8Path) -> Result<Self> {
        let mut template_names = Vec::new();
        let mut environment = Self::create_environment();

        let iter = PathIterator::new(template_directory, None)
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

    pub fn read_filename(path: &Utf8Path, name: &str) -> Result<Self> {
        let mut environment = Self::create_environment();

        let template = fs_err::read_to_string(path)?;

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

        let syntax = self.environment.syntax();

        if source.trim().starts_with(syntax.comment_start.as_ref()) {
            let option = source.split_once(syntax.comment_end.as_ref()).map(
                |(left, _)| {
                    left.replace(syntax.comment_start.as_ref(), "")
                        .replace(syntax.comment_end.as_ref(), "")
                        .trim()
                        .to_owned()
                },
            );

            option
        } else {
            None
        }
    }

    fn path_is_template(path: &Utf8Path) -> bool {
        path.extension().map_or(false, |string| {
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

        static RE_ISO: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(\d{4})-\d{2}-\d{2}").unwrap());

        static RE_AMBIGUOUS: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\d{2}-\d{2}-(\d{4})").unwrap());

        static RE_YEAR: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(\d{4})").unwrap());

        if let Some(m) = RE_ISO.find(&date) {
            Ok(m.as_str().to_owned())
        } else if let Some(m) = RE_AMBIGUOUS.find(&date) {
            Ok(m.as_str().to_owned())
        } else if let Some(m) = RE_YEAR.find(&date) {
            Ok(m.as_str().to_owned())
        } else {
            Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                "Unable to parse date: {date}",
            ))
        }
    }

    fn zero_pad(value: Value, width: usize) -> String {
        format!("{value:0>width$}")
    }
}
