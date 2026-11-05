use tfmttools_picotmpl::{
    BoundTemplate, Diagnostic, RenderError, RenderedPath, Template,
};

use super::context::AudioContext;
use super::sanitize::sanitize_component;
use crate::audiofile::AudioFile;
use crate::error::{TFMTError, TFMTResult};
use crate::item_keys::resolve_tag_name;
use crate::util::Utf8PathExt;
use crate::warning::Warning;

/// Compile a path template and validate every reference against the audio schema.
pub fn compile_audio_template(source: &str) -> TFMTResult<Template> {
    let template = Template::compile(source)
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
        " Legacy Jinja/frontmatter syntax requires manual migration; provide an explicit replacement with `--script 'template: ($artist / $title)'` or `--template` pointing to a migrated file."
    } else {
        ""
    };
    TFMTError::Template(format!("{line}:{column}: {}{hint}", error.message))
}

/// Render raw metadata, then apply character rules to completed components.
pub fn render_audio_path(
    template: &BoundTemplate,
    audio_file: &AudioFile,
    strict: bool,
) -> TFMTResult<(RenderedPath, Vec<Warning>)> {
    let mut context = AudioContext::new(audio_file);
    let output =
        template.render(|name| context.resolve(name)).map_err(|error| {
            let diagnostic = match error {
                RenderError::Template(error) => error,
                RenderError::Resolver { source, .. } => {
                    return TFMTError::TemplateRender {
                        file: audio_file.file().clone().into_path_buf(),
                        source: Box::new(source),
                    };
                },
            };
            TFMTError::TemplateRender {
                file: audio_file.file().clone().into_path_buf(),
                source: Box::new(TFMTError::Template(
                    template.format_diagnostic(&diagnostic),
                )),
            }
        })?;
    let output = output
        .try_map_components(|component| sanitize_component(component, strict))
        .map_err(|source| {
            TFMTError::TemplateRender {
                file: audio_file.file().clone().into_path_buf(),
                source: Box::new(source),
            }
        })?;
    Ok((output, context.take_warnings()))
}
