use std::path::Path;

use color_eyre::Result;
use file_history::Change;
use indicatif::{
    ProgressBar, ProgressDrawTarget, ProgressFinish, ProgressStyle,
};

use crate::cli::ui::table::Table;
use crate::config::{Config, DRY_RUN_PREFIX};

pub(crate) mod table;

pub(crate) fn print_error(error: &color_eyre::Report) {
    println!("An error occurred:\n{error}");
}

pub(crate) fn create_spinner(
    found: &str,
    total: &str,
    msg: &'static str,
    finished_msg: &'static str,
) -> Result<ProgressBar> {
    let spinner = ProgressBar::new(0)
        .with_finish(ProgressFinish::AbandonWithMessage(finished_msg.into()));

    let template =
        format!("[{{pos}}/{{len}} {found}/{total}] {{wide_msg}} {{spinner}}",);

    let style = ProgressStyle::default_spinner().template(&template)?;

    spinner.set_style(style);
    spinner.set_draw_target(ProgressDrawTarget::stdout());
    spinner.set_message(msg);

    Ok(spinner)
}

pub(crate) fn create_progressbar(
    len: u64,
    msg: &'static str,
    finished_msg: &'static str,
    dry_run: bool,
) -> Result<ProgressBar> {
    let bar = ProgressBar::new(len).with_finish(ProgressFinish::WithMessage(
        std::borrow::Cow::Borrowed(finished_msg),
    ));

    let prefix = if dry_run { DRY_RUN_PREFIX } else { "" };

    let template = format!("{prefix}[{{pos}}/{{len}}] {{msg}} {{wide_bar}}");

    bar.set_style(ProgressStyle::default_bar().template(&template)?);
    bar.set_draw_target(ProgressDrawTarget::stdout());
    bar.set_message(msg);

    Ok(bar)
}

pub(crate) fn print_changes_preview(
    config: &Config,
    changes: &[Change],
    common_path: &Path,
) {
    let length = changes.len();

    let step = std::cmp::max(changes.len() / config.preview_amount(), 1);

    let slice = changes
        .iter()
        .step_by(step)
        .map(file_history::Change::target)
        .map(|path| path.strip_prefix(common_path).unwrap_or(path))
        .collect::<Vec<_>>();

    let mut table = Table::new();

    table.set_heading(if slice.len() <= config.preview_amount() {
        format!("Previewing {} files", slice.len())
    } else {
        format!("Previewing {} of {} files", slice.len(), length)
    });

    for path in slice {
        table.push_path(path);
    }

    println!("{table}");
}
