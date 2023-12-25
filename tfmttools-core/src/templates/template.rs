use color_eyre::Result;
use minijinja::Value;

use super::context::AudioFileContext;
use crate::audiofile::AudioFile;

#[derive(Debug)]
pub struct Template<'templates, 'source> {
    inner: minijinja::Template<'templates, 'source>,
    name: String,
    description: Option<String>,
    arguments: Vec<String>,
}

impl<'templates, 'source> Template<'templates, 'source> {
    pub fn new(
        inner: minijinja::Template<'templates, 'source>,
        name: String,
        description: Option<String>,
        arguments: Vec<String>,
    ) -> Self {
        Self { inner, name, description, arguments }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    pub fn render(&self, audio_file: &AudioFile) -> Result<String> {
        let struct_object = AudioFileContext::safe(
            audio_file.to_owned(),
            self.arguments.clone(),
        );

        let context = Value::from_struct_object(struct_object);

        let output = self.inner.render(&context)?;

        Ok(output)
    }
}
