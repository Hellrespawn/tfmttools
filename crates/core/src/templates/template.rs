use minijinja::Value;

use super::context::AudioFileContext;
use crate::audiofile::AudioFile;
use crate::error::TFMTResult;

#[derive(Debug)]
pub struct Template<'templates, 'source> {
    inner: minijinja::Template<'templates, 'source>,
    name: String,
    description: Option<String>,
    arguments: Vec<String>,
}

impl<'templates, 'source> Template<'templates, 'source> {
    #[must_use]
    pub fn new(
        inner: minijinja::Template<'templates, 'source>,
        name: String,
        description: Option<String>,
        arguments: Vec<String>,
    ) -> Self {
        Self { inner, name, description, arguments }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    pub fn render(&self, audio_file: &AudioFile) -> TFMTResult<String> {
        let context = AudioFileContext::safe(
            audio_file.to_owned(),
            self.arguments.clone(),
        );

        let context_value = Value::from_object(context);

        let output = self.inner.render(&context_value)?;

        Ok(output)
    }
}
