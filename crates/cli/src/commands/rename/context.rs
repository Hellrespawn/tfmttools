use tfmttools_fs::{FsHandler, PathIteratorOptions};

use crate::options::{RenameOptions, TFMTOptions};

#[derive(Debug)]
pub struct RenameContext<'rc> {
    fs_handler: &'rc FsHandler,
    path_iterator_options: &'rc PathIteratorOptions<'rc>,
    app_options: &'rc TFMTOptions,
    rename_options: &'rc RenameOptions,
}

impl<'rc> RenameContext<'rc> {
    pub fn new(
        fs_handler: &'rc FsHandler,
        path_iterator_options: &'rc PathIteratorOptions<'rc>,
        app_options: &'rc TFMTOptions,
        rename_options: &'rc RenameOptions,
    ) -> Self {
        Self { fs_handler, path_iterator_options, app_options, rename_options }
    }

    pub fn fs_handler(&self) -> &FsHandler {
        self.fs_handler
    }

    pub fn path_iterator_options(&self) -> &PathIteratorOptions<'rc> {
        self.path_iterator_options
    }

    pub fn app_options(&self) -> &TFMTOptions {
        self.app_options
    }

    pub fn rename_options(&self) -> &RenameOptions {
        self.rename_options
    }
}
