use camino::Utf8Path;
use color_eyre::Result;
use fs_err as fs;
use minijinja::{escape_formatter, Environment, Value};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::tags::Tags;

pub(crate) const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

#[derive(Debug)]
pub(crate) struct Template<'s> {
    name: String,
    environment: Environment<'s>,
    arguments: Vec<String>,
}

impl<'s> Template<'s> {
    pub(crate) fn from_file(path: &Utf8Path) -> Result<Self> {
        let name =
            path.file_stem().expect("File should have a file name.").to_owned();

        let template = fs::read_to_string(path)?;

        let mut environment = Self::create_environment();
        environment.add_template_owned(name.clone(), template)?;

        Ok(Template {
            name: name.to_string(),
            environment,
            arguments: Vec::new(),
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn description(&self) -> Result<Option<String>> {
        let template = self.get_template()?;
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

            Ok(option)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn with_arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = arguments;
        self
    }

    pub(crate) fn render(&self, tags: &dyn Tags) -> Result<String> {
        let template = self.get_template()?;

        let context = self.create_context(tags)?;

        let output = template.render(context)?;

        Ok(output)
    }

    pub(crate) fn path_predicate(path: &Utf8Path) -> bool {
        path.extension().map_or(false, |string| {
            TEMPLATE_EXTENSIONS.iter().any(|ext| string == *ext)
        })
    }

    fn get_template(&self) -> Result<minijinja::Template> {
        Ok(self.environment.get_template(&self.name)?)
    }

    fn create_environment() -> Environment<'s> {
        let mut env = Environment::new();

        env.set_formatter(|out, state, value| {
            escape_formatter(
                out,
                state,
                if value.is_none() { &Value::UNDEFINED } else { value },
            )
        });

        env.add_filter("year", year);
        env.add_filter("zero_pad", zero_pad);

        env
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

fn year(date: &str) -> Result<String, minijinja::Error> {
    static RE_ISO: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(\d{4})-\d{2}-\d{2}").unwrap());

    static RE_AMBIGUOUS: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\d{2}-\d{2}-(\d{4})").unwrap());

    static RE_YEAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d{4})").unwrap());

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
