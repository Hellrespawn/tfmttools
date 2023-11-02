use camino::Utf8Path;
use color_eyre::Result;
use fs_err as fs;
use minijinja::{escape_formatter, Environment, Value};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::fs::PathIterator;
use crate::tags::Tags;

pub(crate) const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

#[derive(Debug)]
pub(crate) struct Templates<'t> {
    template_names: Vec<String>,
    environment: Environment<'t>,
}

impl<'t> Templates<'t> {
    pub(crate) fn read_directory(
        template_directory: &Utf8Path,
    ) -> Result<Self> {
        let mut template_names = Vec::new();
        let mut environment = Self::create_environment();

        let iter = PathIterator::new(template_directory, None)
            .flatten()
            .filter(|path| Self::path_predicate(path));

        for template_path in iter {
            let name = template_path
                .file_stem()
                .expect("Template::path_predicate should only return files.")
                .to_owned();

            template_names.push(name.clone());
            environment
                .add_template_owned(name, fs::read_to_string(template_path)?)?;
        }

        Ok(Self { template_names, environment })
    }

    pub(crate) fn get_template(
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

    pub(crate) fn get_all_templates(&self) -> Vec<Template> {
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

    fn path_predicate(path: &Utf8Path) -> bool {
        path.extension().map_or(false, |string| {
            TEMPLATE_EXTENSIONS.iter().any(|ext| string == *ext)
        })
    }

    fn create_environment() -> Environment<'t> {
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

    fn year(date: &str) -> Result<String, minijinja::Error> {
        static RE_ISO: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(\d{4})-\d{2}-\d{2}").unwrap());

        static RE_AMBIGUOUS: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\d{2}-\d{2}-(\d{4})").unwrap());

        static RE_YEAR: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(\d{4})").unwrap());

        if let Some(m) = RE_ISO.find(date) {
            Ok(m.as_str().to_owned())
        } else if let Some(m) = RE_AMBIGUOUS.find(date) {
            Ok(m.as_str().to_owned())
        } else if let Some(m) = RE_YEAR.find(date) {
            Ok(m.as_str().to_owned())
        } else {
            Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                "Unable to parse date: {date}",
            ))
        }
    }

    fn zero_pad(value: usize, width: usize) -> String {
        format!("{value:0>width$}")
    }
}

#[derive(Debug)]
pub(crate) struct Template<'templates, 'source> {
    inner: minijinja::Template<'templates, 'source>,
    name: String,
    description: Option<String>,
    arguments: Vec<String>,
}

impl<'templates, 'source> Template<'templates, 'source> {
    pub(crate) fn new(
        inner: minijinja::Template<'templates, 'source>,
        name: String,
        description: Option<String>,
        arguments: Vec<String>,
    ) -> Self {
        Self { inner, name, description, arguments }
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub(crate) fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    pub(crate) fn render(&self, tags: &dyn Tags) -> Result<String> {
        let context = self.create_context(tags)?;

        let output = self.inner.render(context)?;

        Ok(output)
    }

    fn create_context(&self, tags: &dyn Tags) -> Result<Value> {
        let albumsort: Option<usize> =
            tags.albumsort().map(|string| string.parse()).transpose()?;

        let track_number: Option<usize> =
            tags.track_number().map(|string| string.parse()).transpose()?;

        let disc_number: Option<usize> =
            tags.disc_number().map(|string| string.parse()).transpose()?;

        let ctx = minijinja::context! {
            args => self.arguments,

            album => &tags.album(),

            albumartist => &tags.album_artist(),
            album_artist => &tags.album_artist(),

            albumsort => albumsort,
            album_sort => albumsort,

            artist => &tags.artist(),

            genre => &tags.genre(),

            title => &tags.title(),

            date => &tags.date(),

            year => &tags.year(),

            tracknumber => track_number,
            track_number => track_number,

            discnumber => disc_number,
            disc_number => disc_number,
            disknumber => disc_number,
            disk_number => disc_number,
        };

        Ok(ctx)
    }
}
