use camino::Utf8PathBuf;
use tfmttools_fs::{FileOrName, FsHandler, PathIteratorOptions};

use crate::config::paths::AppPaths;

#[derive(Debug)]
pub struct RenameTemplateOptions {
    pub template_directory: Utf8PathBuf,

    pub template: Option<FileOrName>,
    pub arguments: Vec<String>,
}

impl RenameTemplateOptions {
    pub fn new(
        template_directory: Utf8PathBuf,
        template: Option<FileOrName>,
        arguments: Vec<String>,
    ) -> Self {
        Self { template_directory, template, arguments }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenameMiscOptions {
    pub always_copy: bool,
    pub no_confirm: bool,
    pub dry_run: bool,
}

impl RenameMiscOptions {
    pub fn new(always_copy: bool, no_confirm: bool, dry_run: bool) -> Self {
        Self { always_copy, no_confirm, dry_run }
    }
}

#[derive(Debug)]
pub struct RenameContext<'rc> {
    pub app_paths: &'rc AppPaths,
    pub fs_handler: &'rc FsHandler,
    pub path_iterator_options: &'rc PathIteratorOptions<'rc>,
    pub template_options: &'rc RenameTemplateOptions,
    pub misc_options: RenameMiscOptions,
}

impl<'rc> RenameContext<'rc> {
    pub fn new(
        app_paths: &'rc AppPaths,
        fs_handler: &'rc FsHandler,
        path_iterator_options: &'rc PathIteratorOptions,
        template_options: &'rc RenameTemplateOptions,
        misc_options: RenameMiscOptions,
    ) -> Self {
        Self {
            app_paths,
            fs_handler,
            path_iterator_options,
            template_options,
            misc_options,
        }
    }
}
