use std::convert::Infallible;
use std::sync::Arc;

use path_template::{
    ArgumentPolicy, BoundScript, Diagnostic, RenderError, RenderedPath, Script,
};

use super::audio_context::AudioContext;
use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::error::{TFMTError, TFMTResult};
use crate::item_keys::ItemKeys;
use crate::util::Utf8PathExt;
use crate::warning::Warning;

#[derive(Clone, Debug)]
pub struct CompiledTemplate {
    script: Script,
    lookup_name: String,
    source: Arc<str>,
}

#[derive(Debug)]
pub struct BoundTemplate {
    template: CompiledTemplate,
    script: BoundScript,
}

impl CompiledTemplate {
    pub fn compile(lookup_name: &str, source: String) -> TFMTResult<Self> {
        let forbidden: Vec<char> = FORBIDDEN_CHARACTERS
            .iter()
            .flat_map(|entry| entry.char().chars())
            .collect();
        let script = Script::compile(&source, ArgumentPolicy::new(&forbidden))
            .map_err(|error| Self::diagnostic(lookup_name, &source, &error))?;
        for reference in script.tag_references() {
            if reference.name != "date"
                && ItemKeys::from_string(&reference.name).is_err()
            {
                return Err(Self::diagnostic(
                    lookup_name,
                    &source,
                    &Diagnostic {
                        message: format!("Unknown tag: '{}'", reference.name),
                        span: reference.span,
                    },
                ));
            }
        }
        Ok(Self {
            script,
            lookup_name: lookup_name.to_owned(),
            source: source.into(),
        })
    }

    fn diagnostic(name: &str, source: &str, error: &Diagnostic) -> TFMTError {
        let (line, column) = error.line_column(source);
        let legacy = source.contains("{{")
            || source.contains("{%-")
            || source.contains("{%")
            || source.trim_start().starts_with("+++")
            || source.trim_start().starts_with("{#");
        let hint = if legacy {
            " Legacy Jinja/frontmatter syntax requires manual migration; use a new script such as `path: ({$artist} / {$title})`."
        } else {
            ""
        };
        TFMTError::Template(format!(
            "Template '{name}' at {line}:{column}: {}{hint}",
            error.message
        ))
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.script.metadata().name.as_deref().unwrap_or(&self.lookup_name)
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.script.metadata().description.as_deref()
    }

    #[must_use]
    pub fn declared_args(&self) -> &[path_template::ArgSpec] {
        self.script.arguments()
    }

    pub fn bind(&self, arguments: &[String]) -> TFMTResult<BoundTemplate> {
        let script = self.script.bind(arguments).map_err(|error| {
            Self::diagnostic(&self.lookup_name, &self.source, &error)
        })?;
        Ok(BoundTemplate { template: self.clone(), script })
    }
}

impl BoundTemplate {
    pub fn render(
        &self,
        audio_file: &AudioFile,
    ) -> TFMTResult<(RenderedPath, Vec<Warning>)> {
        let mut context = AudioContext::new(audio_file);
        let result = self
            .script
            .render(|name| Ok::<_, Infallible>(context.resolve(name)));
        let output = result.map_err(|error| {
            let diagnostic = match error {
                RenderError::Template(error) => error,
                RenderError::Resolver { source, .. } => match source {},
            };
            TFMTError::TemplateRender {
                file: audio_file.file().clone().into_path_buf(),
                source: Box::new(CompiledTemplate::diagnostic(
                    &self.template.lookup_name,
                    &self.template.source,
                    &diagnostic,
                )),
            }
        })?;
        Ok((output, context.take_warnings()))
    }
}
