use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use tfmttools_core::audiofile::AudioFile;
use tfmttools_fs::{FsResult, PathIterator, PathIteratorOptions};

use crate::cli::TFMTOptions;
use crate::ui::ProgressBar;

pub(crate) fn gather_audio_paths(
    app_options: &TFMTOptions,
    options: &PathIteratorOptions<'_>,
) -> Result<Vec<Utf8PathBuf>> {
    let spinner = ProgressBar::spinner(
        app_options.display_mode(),
        "audio",
        "total files",
        "Gathering files...",
        "Gathered files.",
    );
    collect_audio_paths(
        options.input_directory(),
        PathIterator::new(options),
        spinner,
    )
}

fn collect_audio_paths(
    directory: &Utf8Path,
    paths: impl Iterator<Item = FsResult<Utf8PathBuf>>,
    spinner: ProgressBar,
) -> Result<Vec<Utf8PathBuf>> {
    let mut found = Vec::new();
    for path in paths {
        let path = path.wrap_err_with(|| {
            format!("Unable to discover audio files in {directory}")
        })?;
        spinner.inc_total();
        if AudioFile::path_predicate(&path) {
            spinner.inc_found();
            #[cfg(feature = "debug")]
            crate::debug::delay();
            found.push(path);
        }
    }
    spinner.finish();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::DisplayMode;

    fn spinner() -> ProgressBar {
        ProgressBar::spinner(
            DisplayMode::Simple,
            "audio",
            "files",
            "Scanning",
            "Scanned",
        )
    }

    #[test]
    fn scan_error_discards_partial_results() {
        let result = collect_audio_paths(
            Utf8Path::new("input"),
            [
                Ok(Utf8PathBuf::from("input/song.mp3")),
                Err(tfmttools_fs::FsError::Execution(
                    "injected traversal error".into(),
                )),
            ]
            .into_iter(),
            spinner(),
        );
        let error = result.unwrap_err();
        assert!(error.to_string().contains("input"));
        assert!(format!("{error:?}").contains("injected traversal error"));
    }

    #[test]
    fn successful_scan_preserves_audio_order_and_extension_matching() {
        let paths = ["b.mp3", "cover.jpg", "a.ogg", "c.flac"];
        let result = collect_audio_paths(
            Utf8Path::new("input"),
            paths.into_iter().map(|p| Ok(Utf8PathBuf::from(p))),
            spinner(),
        )
        .unwrap();
        assert_eq!(result, ["b.mp3", "a.ogg"]);
    }
}
