use camino::Utf8Path;
use color_eyre::Result;
use path_template::Template;
use tfmttools_core::error::TFMTError;
use tfmttools_core::templates::compile_audio_template;
use tfmttools_fs::read_template;

pub(super) fn compile(lookup_name: &str, source: &str) -> Result<Template> {
    compile_audio_template(source)
        .map_err(|error| named_error(lookup_name, error).into())
}

pub(super) fn load(lookup_name: &str, path: &Utf8Path) -> Result<Template> {
    compile(lookup_name, &read_template(path)?)
}

pub(super) fn named_error(name: &str, error: TFMTError) -> TFMTError {
    match error {
        TFMTError::Template(message) => {
            TFMTError::Template(format!("Template '{name}' at {message}"))
        },
        TFMTError::TemplateRender { file, source } => {
            TFMTError::TemplateRender {
                file,
                source: Box::new(named_error(name, *source)),
            }
        },
        other => other,
    }
}
