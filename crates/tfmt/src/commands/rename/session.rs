use std::collections::HashSet;

use camino::Utf8PathBuf;
use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{History, LoadHistoryResult};
use tfmttools_core::util::Utf8PathExt;
use tfmttools_fs::PathIteratorOptions;
use tracing::info;

use super::{RenameDecision, cleanup, confirmation, planning, preview};
use crate::cli::{RenameArgs, RenameOptions, TFMTOptions};
use crate::history::load_history_for_mode;
use crate::ui::ProgressBar;

fn file_move_completions(
    actions: &[Action],
    destinations: &HashSet<Utf8PathBuf>,
) -> Vec<bool> {
    actions.iter().enumerate().map(|(index, action)| match action {
        Action::MoveFile { target, .. } => destinations.contains(target),
        Action::RemoveFile(source) => index.checked_sub(1).is_some_and(|previous| {
            matches!(&actions[previous], Action::CopyFile { source: copied, target }
                if copied == source && destinations.contains(target))
        }),
        _ => false,
    }).collect()
}

pub struct RenameSession<'a> {
    app_options: &'a TFMTOptions,
    rename_options: RenameOptions,
    history: History,
    load_result: LoadHistoryResult,
}

impl<'a> RenameSession<'a> {
    pub fn from_args(
        app_options: &'a TFMTOptions,
        rename_args: RenameArgs,
    ) -> Result<Self> {
        let rename_options =
            RenameOptions::try_from((rename_args, app_options))?;

        let (mut history, load_result) = load_history_for_mode(
            &app_options.history_file_path()?,
            app_options.fs_mode(),
        )?;

        crate::history::execution::admit_command(
            &mut history,
            app_options.fs_mode(),
        )?;

        Ok(Self { app_options, rename_options, history, load_result })
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

    pub fn run(mut self) -> Result<()> {
        let plan =
            planning::create_plan(&self, &self.history, self.load_result)?;

        if !plan.actions.is_empty() {
            preview::preview(&self, &plan)?;
        }

        let execution = confirmation::confirm_and_plan(&self, plan)?;
        self.finish(execution)
    }

    fn finish(&mut self, execution: RenameDecision) -> Result<()> {
        match execution {
            RenameDecision::Confirmed {
                actions,
                destinations,
                unchanged_files,
                metadata,
            } => {
                let completions =
                    file_move_completions(&actions, &destinations);
                let actions = cleanup::append_cleanup_actions(
                    self,
                    actions,
                    &unchanged_files,
                )?;
                let cleanup_count = actions.len() - completions.len();
                let display_progress = !self.app_options.fs_mode().is_dry_run();
                let display_mode = self.app_options.display_mode();
                let mut bar = display_progress.then(|| {
                    ProgressBar::bar(
                        display_mode,
                        "Moving files:",
                        "Moved files.",
                        destinations.len() as u64,
                        true,
                    )
                });
                let mut position = 0;
                crate::history::execution::execute_recorded(
                    &mut self.history,
                    self.app_options.fs_mode(),
                    actions,
                    metadata,
                    || {
                        if (position >= completions.len()
                            || completions[position])
                            && let Some(bar) = &bar
                        {
                            bar.inc_found();
                        }
                        position += 1;
                        if position == completions.len() {
                            if let Some(bar) = bar.take() {
                                bar.finish();
                            }
                            if display_progress && cleanup_count > 0 {
                                bar = Some(ProgressBar::bar(
                                    display_mode,
                                    "Cleaning up:",
                                    "Cleaned up.",
                                    cleanup_count as u64,
                                    true,
                                ));
                            }
                        }
                    },
                )?;
                if let Some(bar) = bar {
                    bar.finish();
                }
                if !matches!(
                    self.app_options.fs_mode(),
                    tfmttools_core::util::FSMode::DryRun
                ) {
                    println!("Run completed.");
                    println!(
                        "Saved run #{} to history.",
                        self.app_options.run_id()
                    );
                }
            },
            RenameDecision::NothingToRename => {
                let msg = "There are no audio files to rename.";
                println!("{msg}");
                info!("{msg}");
            },
            RenameDecision::Aborted => {
                println!("Aborting!");
            },
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use tfmttools_core::action::Action;

    use super::file_move_completions;

    #[test]
    fn file_progress_ignores_directories_staging_and_copy_until_source_removed()
    {
        let path = Utf8PathBuf::from;
        let actions = vec![
            Action::MakeDir(path("output")),
            Action::MoveFile { source: path("a"), target: path("temporary") },
            Action::MoveFile {
                source: path("temporary"),
                target: path("output/a"),
            },
            Action::CopyFile { source: path("b"), target: path("output/b") },
            Action::RemoveFile(path("b")),
        ];
        let destinations =
            [path("output/a"), path("output/b")].into_iter().collect();
        assert_eq!(file_move_completions(&actions, &destinations), [
            false, false, true, false, true
        ]);
    }
}
