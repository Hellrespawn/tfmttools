use std::iter::repeat_with;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_fs::{FileOrName, FsHandler, PathIteratorOptions};

use crate::config::paths::AppPaths;

#[derive(Debug)]
pub struct RenameTemplateOptions {
    template_directory: Utf8PathBuf,

    template: Option<FileOrName>,
    arguments: Vec<String>,
}

impl RenameTemplateOptions {
    pub fn new(
        template_directory: Utf8PathBuf,
        template: Option<FileOrName>,
        arguments: Vec<String>,
    ) -> Self {
        Self { template_directory, template, arguments }
    }

    pub fn template_directory(&self) -> &Utf8Path {
        self.template_directory.as_ref()
    }

    pub fn template(&self) -> Option<&FileOrName> {
        self.template.as_ref()
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

#[derive(Debug, Clone)]
pub struct RenameMiscOptions {
    always_copy: bool,
    no_confirm: bool,
    dry_run: bool,
    run_id: String,
}

impl RenameMiscOptions {
    pub fn new(always_copy: bool, no_confirm: bool, dry_run: bool) -> Self {
        let run_id = repeat_with(fastrand::alphanumeric).take(12).collect();

        Self::with_run_id(always_copy, no_confirm, dry_run, run_id)
    }

    pub fn with_run_id(
        always_copy: bool,
        no_confirm: bool,
        dry_run: bool,
        run_id: String,
    ) -> Self {
        Self { always_copy, no_confirm, dry_run, run_id }
    }

    pub fn always_copy(&self) -> bool {
        self.always_copy
    }

    pub fn no_confirm(&self) -> bool {
        self.no_confirm
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}

#[derive(Debug)]
pub struct RenameContext<'rc> {
    app_paths: &'rc AppPaths,
    fs_handler: &'rc FsHandler,
    path_iterator_options: &'rc PathIteratorOptions<'rc>,
    template_options: &'rc RenameTemplateOptions,
    misc_options: &'rc RenameMiscOptions,
}

impl<'rc> RenameContext<'rc> {
    pub fn new(
        app_paths: &'rc AppPaths,
        fs_handler: &'rc FsHandler,
        path_iterator_options: &'rc PathIteratorOptions,
        template_options: &'rc RenameTemplateOptions,
        misc_options: &'rc RenameMiscOptions,
    ) -> Self {
        Self {
            app_paths,
            fs_handler,
            path_iterator_options,
            template_options,
            misc_options,
        }
    }

    pub fn app_paths(&self) -> &AppPaths {
        self.app_paths
    }

    pub fn fs_handler(&self) -> &FsHandler {
        self.fs_handler
    }

    pub fn path_iterator_options(&self) -> &PathIteratorOptions<'rc> {
        self.path_iterator_options
    }

    pub fn template_options(&self) -> &RenameTemplateOptions {
        self.template_options
    }

    pub fn misc_options(&self) -> &RenameMiscOptions {
        self.misc_options
    }
}
