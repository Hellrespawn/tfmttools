use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::RenameAction;
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::error::TFMTResult;
use tfmttools_core::templates::Template;
use tfmttools_fs::PathIterator;
use tracing::{debug, trace};

use super::RenameSession;
use super::template_resolution::ResolvedTemplate;
use crate::ui::{ProgressBar, current_dir_utf8};

pub(super) fn create_actions_from_template(
    session: &RenameSession,
    resolved: &ResolvedTemplate,
) -> Result<Vec<RenameAction>> {
    let template = resolved
        .loader
        .get_template(&resolved.template_name, resolved.arguments.clone())
        .ok_or(eyre!("Unable to find template: {}", resolved.template_name))?;

    let paths = gather_file_paths(session);

    debug!("Read {} files.", paths.len());

    let audio_files = read_files(session, paths)?;

    debug!("Found {} audio files.", audio_files.len());

    let rename_actions =
        create_rename_actions(session, &template, &audio_files)?;

    Ok(rename_actions)
}

fn gather_file_paths(session: &RenameSession) -> Vec<Utf8PathBuf> {
    let spinner = ProgressBar::spinner(
        session.app_options().display_mode(),
        "audio",
        "total files",
        "Gathering files...",
        "Gathered files.",
    );

    let file_paths = PathIterator::new(&session.path_iterator_options())
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

fn read_files(
    session: &RenameSession,
    file_paths: Vec<Utf8PathBuf>,
) -> Result<Vec<AudioFile>> {
    let bar = ProgressBar::bar(
        session.app_options().display_mode(),
        "Reading files...",
        "Read files.",
        file_paths.len() as u64,
        false,
    );

    let audio_files = file_paths
        .into_iter()
        .inspect(|_| {
            bar.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .map(|path| {
            let audio_file = AudioFile::new(path)?;

            trace!("Found audio file: {audio_file:?}");

            Ok(audio_file)
        })
        .collect::<TFMTResult<Vec<_>>>();

    bar.finish();

    Ok(audio_files?)
}

fn create_rename_actions(
    session: &RenameSession,
    template: &Template,
    files: &[AudioFile],
) -> Result<Vec<RenameAction>> {
    let cwd = current_dir_utf8()?;

    let bar = ProgressBar::bar(
        session.app_options().display_mode(),
        "Determining output paths:",
        "Determined output paths.",
        files.len() as u64,
        true,
    );

    let rename_actions: Result<Vec<RenameAction>> = files
        .iter()
        .map(|audiofile| {
            let rename_action = RenameAction::new(
                audiofile.file().to_owned(),
                audiofile.construct_target_path(template, &cwd)?,
            );

            bar.inc_found();
            trace!("Created rename action: {rename_action:?}");

            #[cfg(feature = "debug")]
            crate::debug::delay();

            Ok(rename_action)
        })
        .collect();

    bar.finish();

    rename_actions
}
