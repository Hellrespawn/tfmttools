use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use tfmttools_core::audiofile::encoding::convert_encoding_to_utf8;
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::error::TFMTError;
use tfmttools_fs::PathIterator;

use crate::config::paths::AppPaths;
use crate::ui::{
    ConfirmationPrompt, ItemName, PreviewList, ProgressBar, ProgressBarOptions,
};

const UTF_16_ERROR_TEXT: &str =
    "Text decoding: UTF-16 string has an odd length";

#[derive(Debug)]
pub struct FixCommand {
    input_directory: Utf8PathBuf,

    yes: bool,
    dry_run: bool,

    recursion_depth: usize,
}

impl FixCommand {
    pub fn new(
        input_directory: Utf8PathBuf,
        yes: bool,
        dry_run: bool,
        recursion_depth: usize,
    ) -> Self {
        Self { input_directory, yes, dry_run, recursion_depth }
    }
}

impl FixCommand {
    pub fn run(&self, app_paths: &AppPaths) -> Result<()> {
        let paths = self.gather_file_paths();

        let utf16_error_paths = Self::get_utf16_error_files(paths);

        if utf16_error_paths.is_empty() {
            println!("There are no files to fix.");
        } else {
            let cwd = app_paths.working_directory()?;

            Self::preview_files_to_fix(&utf16_error_paths, &cwd)?;

            let confirmation_prompt = ConfirmationPrompt::new(
                "Attempt to fix files? This can not be undone.",
            );

            if self.yes || confirmation_prompt.prompt()? {
                Self::fix_files(&utf16_error_paths, self.dry_run)?;
            }
        }

        Ok(())
    }

    fn gather_file_paths(&self) -> Vec<Utf8PathBuf> {
        let options = ProgressBarOptions::spinner(
            "audio",
            "total",
            "Gathering files...",
            "Gathered files.",
        );

        let spinner = ProgressBar::new(options);

        let file_paths = PathIterator::new(
            &self.input_directory,
            Some(self.recursion_depth),
        )
        .flatten()
        .inspect(|_| spinner.inc_total())
        .filter(|path| AudioFile::path_predicate(path))
        .inspect(|_| {
            spinner.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .collect::<Vec<_>>();

        spinner.finish();

        file_paths
    }

    fn get_utf16_error_files(file_paths: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
        let options = ProgressBarOptions::bar(
            "Reading error files...",
            "Read error files.",
        );

        let bar = ProgressBar::with_length(options, file_paths.len() as u64);

        let error_paths = file_paths
            .into_iter()
            .filter_map(|path| {
                match AudioFile::new(path.clone()) {
                    Err(TFMTError::Lofty(path, err))
                        if err.to_string().contains(UTF_16_ERROR_TEXT) =>
                    {
                        bar.inc_found();
                        Some(path)
                    },
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        bar.finish();

        error_paths
    }

    fn preview_files_to_fix(
        files_to_fix: &[Utf8PathBuf],
        working_directory: &Utf8Path,
    ) -> Result<()> {
        const LEADING_LINES: usize = 3;
        const TRAILING_LINES: usize = 3;

        let iter = files_to_fix.iter().map(|path| {
            let path = path.strip_prefix(working_directory).unwrap_or(path);

            if path.is_relative() {
                format!(".{}{path}", std::path::MAIN_SEPARATOR)
            } else {
                format!("{path}")
            }
        });

        let preview_list = PreviewList::new(iter)
            .leading(LEADING_LINES)
            .trailing(TRAILING_LINES)
            .item_name(ItemName::simple("file"));

        preview_list.print()?;

        Ok(())
    }

    fn fix_files(files_to_fix: &[Utf8PathBuf], dry_run: bool) -> Result<()> {
        let options =
            ProgressBarOptions::bar("Fixing files...", "Fixed files.");

        let bar = ProgressBar::with_length(options, files_to_fix.len() as u64);

        for path in files_to_fix {
            if dry_run {
                bar.inc_found();
            } else {
                let result = convert_encoding_to_utf8(path);

                if result.is_err() {
                    bar.finish();
                } else {
                    bar.inc_found();
                }

                result?;
            }
        }

        bar.finish();

        Ok(())
    }
}
