use minijinja::Value;

use super::context::AudioFileContext;
use super::frontmatter::ResolvedArgs;
use super::{ArgSpec, Frontmatter};
use crate::audiofile::AudioFile;
use crate::error::TFMTResult;

#[derive(Debug)]
pub struct Template<'templates, 'source> {
    inner: minijinja::Template<'templates, 'source>,
    name: String,
    description: Option<String>,
    declared_args: Vec<ArgSpec>,
    resolved: ResolvedArgs,
}

impl<'templates, 'source> Template<'templates, 'source> {
    pub fn new(
        inner: minijinja::Template<'templates, 'source>,
        lookup_name: &str,
        display_name: String,
        description: Option<String>,
        arguments: Vec<String>,
        frontmatter: Option<&Frontmatter>,
    ) -> TFMTResult<Self> {
        let (declared_args, resolved) = match frontmatter {
            Some(frontmatter) => (
                frontmatter.args().to_vec(),
                frontmatter.resolve(lookup_name, &arguments)?,
            ),
            None => (Vec::new(), ResolvedArgs::raw(arguments)),
        };

        Ok(Self { inner, name: display_name, description, declared_args, resolved })
    }

    #[must_use]
    pub fn for_display(
        inner: minijinja::Template<'templates, 'source>,
        display_name: String,
        description: Option<String>,
        declared_args: Vec<ArgSpec>,
    ) -> Self {
        Self {
            inner,
            name: display_name,
            description,
            declared_args,
            resolved: ResolvedArgs::default(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }

    #[must_use]
    pub fn declared_args(&self) -> &[ArgSpec] {
        &self.declared_args
    }

    pub fn render(&self, audio_file: &AudioFile) -> TFMTResult<String> {
        let context =
            AudioFileContext::safe(audio_file.to_owned(), self.resolved.clone());

        let context_value = Value::from_object(context);

        let output = self.inner.render(&context_value)?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use minijinja::Environment;

    use super::*;
    use crate::templates::Frontmatter;

    fn build_minijinja_template<'a>(
        env: &'a Environment<'static>,
        name: &'static str,
    ) -> minijinja::Template<'a, 'a> {
        env.get_template(name).unwrap()
    }

    #[test]
    fn new_without_frontmatter_keeps_raw_positional_arguments() {
        let mut env = Environment::new();
        env.add_template("t", "{{ args[0] }}").unwrap();
        let inner = build_minijinja_template(&env, "t");

        let template = Template::new(
            inner,
            "t",
            "t".to_owned(),
            None,
            vec!["raw:value".to_owned()],
            None,
        )
        .unwrap();

        assert_eq!(template.declared_args().len(), 0);
    }

    #[test]
    fn new_with_frontmatter_errors_on_missing_required_argument() {
        let mut env = Environment::new();
        env.add_template("t", "{{ prefix }}").unwrap();
        let inner = build_minijinja_template(&env, "t");

        let frontmatter = Frontmatter::parse(
            "args = [{ name = \"prefix\", type = \"string\", required = true }]",
            "t",
        )
        .unwrap();

        let error = Template::new(
            inner,
            "t",
            "t".to_owned(),
            None,
            Vec::new(),
            Some(&frontmatter),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            crate::error::TFMTError::MissingRequiredArgument(_, _, _)
        ));
    }

    #[test]
    fn for_display_never_resolves_arguments() {
        let mut env = Environment::new();
        env.add_template("t", "{{ prefix }}").unwrap();
        let inner = build_minijinja_template(&env, "t");

        let frontmatter = Frontmatter::parse(
            "args = [{ name = \"prefix\", type = \"string\", required = true }]",
            "t",
        )
        .unwrap();

        let template = Template::for_display(
            inner,
            "t".to_owned(),
            None,
            frontmatter.args().to_vec(),
        );

        assert_eq!(template.declared_args().len(), 1);
    }
}
