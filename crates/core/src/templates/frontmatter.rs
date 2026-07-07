use std::collections::{HashMap, HashSet};

use minijinja::Value;
use serde::Deserialize;

use super::context::AudioFileContext;
use crate::error::{TFMTError, TFMTResult};

#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    args: Vec<ArgSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArgSpec {
    name: String,
    #[serde(rename = "type", default)]
    kind: ArgKind,
    #[serde(default)]
    required: bool,
    default: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgKind {
    #[default]
    String,
    Int,
    Path,
}

impl std::fmt::Display for ArgKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ArgKind::String => "string",
            ArgKind::Int => "int",
            ArgKind::Path => "path",
        })
    }
}

impl Frontmatter {
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    #[must_use]
    pub fn args(&self) -> &[ArgSpec] {
        &self.args
    }
}

impl ArgSpec {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn kind(&self) -> ArgKind {
        self.kind
    }

    #[must_use]
    pub fn required(&self) -> bool {
        self.required
    }

    #[must_use]
    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }

    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[allow(dead_code)]
fn describe(description: Option<&str>) -> String {
    description.map_or_else(
        || "(no description provided)".to_owned(),
        std::borrow::ToOwned::to_owned,
    )
}

impl Frontmatter {
    pub fn parse(toml_str: &str, label: &str) -> TFMTResult<Self> {
        let frontmatter: Frontmatter = toml::from_str(toml_str)
            .map_err(|error| TFMTError::FrontmatterParse(label.to_owned(), error))?;

        let mut seen = HashSet::new();

        for arg in &frontmatter.args {
            if !seen.insert(arg.name.as_str()) {
                return Err(TFMTError::DuplicateArgumentName(
                    label.to_owned(),
                    arg.name.clone(),
                ));
            }
        }

        Ok(frontmatter)
    }

    #[allow(dead_code)]
    pub(super) fn resolve(
        &self,
        label: &str,
        positional: &[String],
    ) -> TFMTResult<ResolvedArgs> {
        if !self.args.is_empty() && positional.len() > self.args.len() {
            return Err(TFMTError::TooManyArguments(
                label.to_owned(),
                self.args.len(),
                positional.len(),
            ));
        }

        let mut named = HashMap::new();
        let mut ordered = Vec::with_capacity(self.args.len());

        for (index, spec) in self.args.iter().enumerate() {
            let raw = positional.get(index).cloned().or_else(|| spec.default.clone());

            let value = match raw {
                Some(raw) => spec.coerce(label, &raw)?,
                None if spec.required => {
                    return Err(TFMTError::MissingRequiredArgument(
                        label.to_owned(),
                        spec.name.clone(),
                        describe(spec.description.as_deref()),
                    ));
                },
                None => Value::UNDEFINED,
            };

            named.insert(spec.name.clone(), value.clone());
            ordered.push(value);
        }

        Ok(ResolvedArgs { named, positional: ordered })
    }
}

impl ArgSpec {
    #[allow(dead_code)]
    fn coerce(&self, label: &str, raw: &str) -> TFMTResult<Value> {
        match self.kind {
            ArgKind::Int => raw.parse::<i64>().map(Value::from).map_err(|_| {
                TFMTError::InvalidArgumentValue(
                    label.to_owned(),
                    self.name.clone(),
                    describe(self.description.as_deref()),
                    raw.to_owned(),
                )
            }),
            ArgKind::String => {
                Ok(Value::from(AudioFileContext::remove_forbidden_characters(raw.to_owned())))
            },
            ArgKind::Path => Ok(Value::from(sanitize_path(raw))),
        }
    }
}

#[allow(dead_code)]
fn sanitize_path(raw: &str) -> String {
    let segments: Vec<String> = raw
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| AudioFileContext::remove_forbidden_characters(segment.to_owned()))
        .collect();

    if segments.is_empty() {
        String::new()
    } else {
        format!("{}/", segments.join("/"))
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(super) struct ResolvedArgs {
    named: HashMap<String, Value>,
    positional: Vec<Value>,
}

impl ResolvedArgs {
    #[allow(dead_code)]
    pub(super) fn raw(arguments: Vec<String>) -> Self {
        Self {
            named: HashMap::new(),
            positional: arguments.into_iter().map(Value::from).collect(),
        }
    }

    #[allow(dead_code)]
    pub(super) fn get_named(&self, name: &str) -> Option<Value> {
        self.named.get(name).cloned()
    }

    #[allow(dead_code)]
    pub(super) fn positional(&self) -> Value {
        Value::from(self.positional.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let toml = r#"
name = "Stef's layout"
description = "Group by artist and album."

args = [
    { name = "prefix", type = "path", required = true, description = "Directory prefix." },
    { name = "extra", type = "string", required = false, default = "", description = "Optional suffix." },
]
"#;

        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        assert_eq!(frontmatter.name(), Some("Stef's layout"));
        assert_eq!(
            frontmatter.description(),
            Some("Group by artist and album.")
        );
        assert_eq!(frontmatter.args().len(), 2);
        assert_eq!(frontmatter.args()[0].name(), "prefix");
        assert_eq!(frontmatter.args()[0].kind(), ArgKind::Path);
        assert!(frontmatter.args()[0].required());
        assert_eq!(frontmatter.args()[1].kind(), ArgKind::String);
        assert!(!frontmatter.args()[1].required());
    }

    #[test]
    fn parse_missing_arg_name_is_error() {
        let toml = "args = [{ type = \"string\" }]";

        assert!(Frontmatter::parse(toml, "test").is_err());
    }

    #[test]
    fn parse_unknown_type_is_error() {
        let toml = "args = [{ name = \"prefix\", type = \"float\" }]";

        assert!(Frontmatter::parse(toml, "test").is_err());
    }

    #[test]
    fn parse_duplicate_arg_names_is_error() {
        let toml = r#"
args = [
    { name = "prefix", type = "string" },
    { name = "prefix", type = "int" },
]
"#;

        let error = Frontmatter::parse(toml, "test").unwrap_err();

        assert!(matches!(error, TFMTError::DuplicateArgumentName(_, _)));
    }

    #[test]
    fn parse_defaults_kind_to_string_and_required_to_false() {
        let toml = "args = [{ name = \"prefix\" }]";

        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        assert_eq!(frontmatter.args()[0].kind(), ArgKind::String);
        assert!(!frontmatter.args()[0].required());
    }

    #[test]
    fn resolve_uses_default_when_argument_omitted() {
        let toml = "args = [{ name = \"suffix\", type = \"string\", default = \"tag\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved = frontmatter.resolve("test", &[]).unwrap();

        assert_eq!(resolved.get_named("suffix").unwrap().to_string(), "tag");
    }

    #[test]
    fn resolve_errors_on_missing_required_argument() {
        let toml =
            "args = [{ name = \"prefix\", type = \"string\", required = true }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let error = frontmatter.resolve("test", &[]).unwrap_err();

        assert!(matches!(error, TFMTError::MissingRequiredArgument(_, _, _)));
    }

    #[test]
    fn resolve_errors_on_too_many_positional_arguments() {
        let toml = "args = [{ name = \"prefix\", type = \"string\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let error = frontmatter
            .resolve("test", &["a".to_owned(), "b".to_owned()])
            .unwrap_err();

        assert!(matches!(error, TFMTError::TooManyArguments(_, _, _)));
    }

    #[test]
    fn resolve_allows_extra_positional_arguments_when_no_args_declared() {
        let toml = "name = \"No args\"";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved = frontmatter
            .resolve("test", &["a".to_owned(), "b".to_owned()])
            .unwrap();

        assert!(resolved.get_named("a").is_none());
    }

    #[test]
    fn resolve_errors_on_int_parse_failure() {
        let toml = "args = [{ name = \"count\", type = \"int\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let error = frontmatter
            .resolve("test", &["not-a-number".to_owned()])
            .unwrap_err();

        assert!(matches!(error, TFMTError::InvalidArgumentValue(_, _, _, _)));
    }

    #[test]
    fn resolve_sanitizes_string_argument() {
        let toml = "args = [{ name = \"tag\", type = \"string\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved =
            frontmatter.resolve("test", &["a:b*c.".to_owned()]).unwrap();

        assert_eq!(resolved.get_named("tag").unwrap().to_string(), "abc");
    }

    #[test]
    fn resolve_normalizes_path_argument_trailing_separators() {
        let toml = "args = [{ name = \"prefix\", type = \"path\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        for raw in ["a/b", "a/b/", "a/b//", "a\\b\\"] {
            let resolved =
                frontmatter.resolve("test", &[raw.to_owned()]).unwrap();

            assert_eq!(
                resolved.get_named("prefix").unwrap().to_string(),
                "a/b/"
            );
        }
    }

    #[test]
    fn resolve_sanitizes_each_path_segment() {
        let toml = "args = [{ name = \"prefix\", type = \"path\" }]";
        let frontmatter = Frontmatter::parse(toml, "test").unwrap();

        let resolved =
            frontmatter.resolve("test", &["a:b/c*d".to_owned()]).unwrap();

        assert_eq!(
            resolved.get_named("prefix").unwrap().to_string(),
            "ab/cd/"
        );
    }
}
