use color_eyre::Result;
use tfmttools_core::util::Utf8PathExt;
use tfmttools_fs::{FsHandler, PathIteratorOptions};

use crate::cli::{RenameArgs, RenameOptions, TFMTOptions};

#[derive(Debug)]
pub struct RenameContext<'rc> {
    fs_handler: &'rc FsHandler,
    app_options: &'rc TFMTOptions,
    rename_options: RenameOptions,
}

impl<'rc> RenameContext<'rc> {
    pub fn try_from_args(
        fs_handler: &'rc FsHandler,
        app_options: &'rc TFMTOptions,
        rename_args: RenameArgs,
    ) -> Result<Self> {
        Ok(Self {
            fs_handler,
            app_options,
            rename_options: RenameOptions::try_from((
                rename_args,
                app_options,
            ))?,
        })
    }

    pub fn fs_handler(&self) -> &FsHandler {
        self.fs_handler
    }

    pub fn path_iterator_options(&self) -> PathIteratorOptions<'_> {
        PathIteratorOptions::with_depth(
            self.rename_options.input_directory().as_path(),
            self.rename_options.recursion_depth(),
        )
    }

    pub fn app_options(&self) -> &TFMTOptions {
        self.app_options
    }

    pub fn rename_options(&self) -> &RenameOptions {
        &self.rename_options
    }
}
