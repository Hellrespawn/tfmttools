use std::convert::Infallible;

use tfmttools_picotmpl::{
    ArgumentPolicy, BoundTemplate, Diagnostic, RenderError, RenderedPath,
    Template,
};

use super::context::AudioContext;
use crate::action::FORBIDDEN_CHARACTERS;
use crate::audiofile::AudioFile;
use crate::error::{TFMTError, TFMTResult};
use crate::item_keys::resolve_tag_name;
use crate::util::Utf8PathExt;
use crate::warning::Warning;

/// Compile a path template and validate every reference against the audio schema.
pub fn compile_audio_template(source: &str) -> TFMTResult<Template> {
    let forbidden: Vec<char> = FORBIDDEN_CHARACTERS
        .iter()
        .flat_map(|entry| entry.char().chars())
        .collect();
    let template = Template::compile(source, ArgumentPolicy::new(&forbidden))
        .map_err(|error| compilation_error(source, &error))?;
    for reference in template.tag_references() {
        if resolve_tag_name(&reference.name).is_none() {
            return Err(TFMTError::Template(template.format_diagnostic(
                &Diagnostic {
                    message: format!("Unknown tag: '{}'", reference.name),
                    span: reference.span,
                },
            )));
        }
    }
    Ok(template)
}

fn compilation_error(source: &str, error: &Diagnostic) -> TFMTError {
    let (line, column) = error.line_column(source);
    let legacy = source.contains("{{")
        || source.contains("{%")
        || source.trim_start().starts_with("+++")
        || source.trim_start().starts_with("{#");
    let hint = if legacy {
        " Legacy Jinja/frontmatter syntax requires manual migration; provide an explicit replacement with `--script 'path: ({$artist} / {$title})'` or `--template` pointing to a migrated file."
    } else {
        ""
    };
    TFMTError::Template(format!("{line}:{column}: {}{hint}", error.message))
}

/// Render using sanitized audio metadata, retaining warnings and file context.
pub fn render_audio_path(
    template: &BoundTemplate,
    audio_file: &AudioFile,
) -> TFMTResult<(RenderedPath, Vec<Warning>)> {
    let mut context = AudioContext::new(audio_file);
    let output = template
        .render(|name| Ok::<_, Infallible>(context.resolve(name)))
        .map_err(|error| {
            let diagnostic = match error {
                RenderError::Template(error) => error,
                RenderError::Resolver { source, .. } => match source {},
            };
            TFMTError::TemplateRender {
                file: audio_file.file().clone().into_path_buf(),
                source: Box::new(TFMTError::Template(
                    template.format_diagnostic(&diagnostic),
                )),
            }
        })?;
    Ok((output, context.take_warnings()))
}
