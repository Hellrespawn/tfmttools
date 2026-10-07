use std::collections::HashMap;

use camino::Utf8Path;
use fs_err as fs;
use tfmttools_core::templates::Template;
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};

use crate::PathIterator;
use crate::error::FsResult;

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

#[derive(Debug)]
pub struct TemplateLoader {
    template_names: Vec<String>,
    templates: HashMap<String, Template>,
}

impl TemplateLoader {
    pub const DEFAULT_SCRIPT_NAME: &'static str = "script";

    pub fn read_directory(
        template_directory: &Utf8Directory,
    ) -> FsResult<Self> {
        let iter = PathIterator::single_directory(template_directory.as_path())
            .flatten()
            .filter(|path| Self::path_is_template(path));
        let mut sources = Vec::new();
        for path in iter {
            let name =
                path.file_stem().expect("Template path has a stem").to_owned();
            sources.push((name, fs::read_to_string(path)?));
        }
        Self::build(sources)
    }

    pub fn read_filename(path: &Utf8Path, name: &str) -> FsResult<Self> {
        Self::build([(name.to_owned(), fs::read_to_string(path)?)])
    }

    pub fn read_script(script: &str) -> FsResult<Self> {
        Self::build([(Self::DEFAULT_SCRIPT_NAME.to_owned(), script.to_owned())])
    }

    fn build(
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> FsResult<Self> {
        let mut template_names = Vec::new();
        let mut templates = HashMap::new();
        for (name, source) in sources {
            let template = Template::compile(&name, source)?;
            template_names.push(name.clone());
            templates.insert(name, template);
        }
        Ok(Self { template_names, templates })
    }

    #[must_use]
    pub fn get_template(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    #[must_use]
    pub fn get_all_templates(&self) -> Vec<&Template> {
        self.template_names.iter().map(|name| &self.templates[name]).collect()
    }

    fn path_is_template(path: &Utf8Path) -> bool {
        path.extension()
            .is_some_and(|extension| TEMPLATE_EXTENSIONS.contains(&extension))
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;

    use super::*;

    #[test]
    fn compiled_metadata_does_not_require_binding() {
        let loader = TemplateLoader::read_script(
            r#"name: "Display" description: "Description" arg prefix: path path: ({prefix} {$title})"#,
        ).unwrap();
        let templates = loader.get_all_templates();
        assert_eq!(templates[0].name(), "Display");
        assert_eq!(templates[0].declared_args().len(), 1);
    }

    #[test]
    fn new_scripts_keep_lookup_names_and_extensions() {
        let directory = assert_fs::TempDir::new().unwrap();
        for extension in TEMPLATE_EXTENSIONS {
            directory
                .child(format!("test.{extension}"))
                .write_str(r#"path: ("Song")"#)
                .unwrap();
            let loader = TemplateLoader::read_filename(
                &camino::Utf8PathBuf::from_path_buf(
                    directory.path().join(format!("test.{extension}")),
                )
                .unwrap(),
                "test",
            )
            .unwrap();
            assert_eq!(loader.get_all_templates()[0].name(), "test");
        }
    }

    #[test]
    fn loader_rejects_invalid_defaults_and_legacy_syntax() {
        let message = TemplateLoader::read_script(
            r#"arg unused: string(default: "bad?") path: ("Song")"#,
        )
        .unwrap_err()
        .to_string();
        assert!(message.contains("unused"));
        let message =
            TemplateLoader::read_script("{{ title }}").unwrap_err().to_string();
        assert!(message.contains("migrat"));
    }
}
