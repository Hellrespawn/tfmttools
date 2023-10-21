use crate::tags::Tags;
use color_eyre::Result;
use minijinja::Environment;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

#[derive(Debug)]
pub struct Template<'s> {
    name: String,
    environment: Environment<'s>,
    arguments: Vec<String>,
}

impl<'s> Template<'s> {
    pub fn from_file(path: &Path) -> Result<Self> {
        let name = path
            .file_stem()
            .map(std::ffi::OsStr::to_string_lossy)
            .expect("File should have a file name.")
            .to_string();

        let template = std::fs::read_to_string(path)?;

        let mut environment = Self::create_environment();
        environment.add_template_owned(name.clone(), template)?;

        Ok(Template {
            name: name.to_string(),
            environment,
            arguments: Vec::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arguments_mut(&mut self) -> &mut Vec<String> {
        &mut self.arguments
    }

    pub fn render(&self, tags: &dyn Tags) -> Result<String> {
        let template = self.environment.get_template(&self.name)?;

        let context = self.create_context(tags);

        let output = template.render(context)?;

        Ok(output)
    }

    fn create_environment() -> Environment<'s> {
        let mut env = Environment::new();

        env.add_filter("year", year);
        env.add_filter("pad", pad);

        env
    }

    fn create_context(&self, tags: &dyn Tags) -> minijinja::Value {
        minijinja::context! {
            args => self.arguments,

            album => &tags.album(),

            albumartist => &tags.album_artist(),
            album_artist => &tags.album_artist(),

            albumsort => &tags.albumsort(),
            album_sort => &tags.albumsort(),

            artist => &tags.artist(),

            genre => &tags.genre(),

            title => &tags.title(),

            date => &tags.date(),

            year => &tags.year(),

            tracknumber => &tags.track_number(),
            track_number => &tags.track_number(),

            discnumber => &tags.disc_number(),
            disc_number => &tags.disc_number(),
            disknumber => &tags.disc_number(),
            disk_number => &tags.disc_number(),
        }
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

fn pad(value: &str, char: &str, width: usize) -> String {
    // FIXME Doesn't work? format!("{value:char$>width$}")

    let n = width.saturating_sub(value.len()).saturating_div(char.len());

    format!("{}{}", char.repeat(n), value)
}
