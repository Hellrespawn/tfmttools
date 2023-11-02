use color_eyre::Result;
use indicatif::{
    ProgressBar, ProgressDrawTarget, ProgressFinish, ProgressStyle,
};

use crate::config::DRY_RUN_PREFIX;

pub(crate) mod table;

pub(crate) fn print_error(error: &color_eyre::Report) {
    println!("An error occurred:\n{error}");
}

pub(crate) struct PathFilterSpinner {
    spinner: ProgressBar,
    finished_message: &'static str,
}

impl PathFilterSpinner {
    pub(crate) fn new(
        found: &str,
        total: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        let spinner = ProgressBar::new(0).with_finish(ProgressFinish::Abandon);

        let template = format!(
            "[{{pos}}/{{len}} {found}/{total} files] {{wide_msg}} {{spinner}}",
        );

        let style = ProgressStyle::default_spinner().template(&template)?;

        spinner.set_style(style);
        spinner.set_draw_target(ProgressDrawTarget::stdout());
        spinner.set_message(working_message);

        Ok(Self { spinner, finished_message })
    }

    pub(crate) fn inc_found(&self) {
        self.spinner.inc(1);
    }

    pub(crate) fn inc_total(&self) {
        // std::thread::sleep(std::time::Duration::from_millis(100));
        self.spinner.inc_length(1);
        self.spinner.tick();
    }

    pub(crate) fn finish(&self) {
        self.spinner.finish_using_style();
        self.spinner.set_message(self.finished_message);
    }
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
