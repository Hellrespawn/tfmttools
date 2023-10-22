use super::config::DRY_RUN_PREFIX;
use super::Config;
use crate::cli::ui::table::Table;
use color_eyre::Result;
use file_history::Action;
use indicatif::{
    ProgressBar as IProgressBar, ProgressDrawTarget, ProgressFinish,
    ProgressStyle,
};
use std::path::Path;

pub(crate) mod table;

pub(crate) fn print_error(error: &color_eyre::Report) {
    println!("An error occurred:\n{error}");
}

pub(crate) struct AudioFileSpinner {
    spinner: IProgressBar,
}

impl AudioFileSpinner {
    pub(crate) fn new(
        found: &str,
        total: &str,
        message: &'static str,
    ) -> Result<AudioFileSpinner> {
        let spinner = IProgressBar::new(0).with_finish(ProgressFinish::Abandon);

        let template = format!(
            "[{{pos}}/{{len}} {found}/{total}] {{wide_msg}} {{spinner}}",
        );

        let style = ProgressStyle::default_spinner().template(&template)?;

        spinner.set_style(style);
        spinner.set_draw_target(ProgressDrawTarget::stdout());
        spinner.set_message(message);

        Ok(AudioFileSpinner { spinner })
    }

    pub(crate) fn inc_found(&self) {
        self.spinner.inc(1);
    }

    pub(crate) fn inc_total(&self) {
        // std::thread::sleep(std::time::Duration::from_millis(100));
        self.spinner.inc_length(1);
        self.spinner.tick();
    }

    pub(crate) fn finish(&self, message: &'static str) {
        self.spinner.finish_using_style();
        self.spinner.set_message(message);
    }
}

pub(crate) fn create_progressbar(
    len: u64,
    msg: &'static str,
    finished_msg: &'static str,
    dry_run: bool,
) -> Result<IProgressBar> {
    let bar = IProgressBar::new(len).with_finish(ProgressFinish::WithMessage(
        std::borrow::Cow::Borrowed(finished_msg),
    ));

    let prefix = if dry_run { DRY_RUN_PREFIX } else { "" };

    let template = format!("{prefix}[{{pos}}/{{len}}] {{msg}} {{wide_bar}}");

    bar.set_style(ProgressStyle::default_bar().template(&template)?);
    bar.set_draw_target(ProgressDrawTarget::stdout());
    bar.set_message(msg);

    Ok(bar)
}

pub(crate) fn print_actions_preview(
    config: &Config,
    actions: &[Action],
    common_path: &Path,
) {
    let length = actions.len();

    let step = std::cmp::max(actions.len() / config.preview_amount(), 1);

    let slice = actions
        .iter()
        .step_by(step)
        .map(|a| a.get_src_tgt_unchecked().1)
        .map(|p| p.strip_prefix(common_path).unwrap_or(p))
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
