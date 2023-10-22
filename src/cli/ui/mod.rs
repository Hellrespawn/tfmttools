use std::path::Path;

use color_eyre::Result;
use file_history::Action;
use indicatif::{
    ProgressBar as IProgressBar, ProgressDrawTarget, ProgressFinish,
    ProgressStyle,
};

use crate::cli::config::DEFAULT_PREVIEW_AMOUNT;

use super::config::PREVIEW_PREFIX;

pub mod table;

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
    preview: bool,
) -> Result<IProgressBar> {
    let bar = IProgressBar::new(len).with_finish(ProgressFinish::WithMessage(
        std::borrow::Cow::Borrowed(finished_msg),
    ));

    let pp = if preview { PREVIEW_PREFIX } else { "" };

    let template = format!("{pp}[{{pos}}/{{len}}] {{msg}} {{wide_bar}}");

    bar.set_style(ProgressStyle::default_bar().template(&template)?);
    bar.set_draw_target(ProgressDrawTarget::stdout());
    bar.set_message(msg);

    Ok(bar)
}

pub(crate) fn print_actions_preview(actions: &[Action], common_path: &Path) {
    let length = actions.len();

    let step = std::cmp::max(actions.len() / DEFAULT_PREVIEW_AMOUNT, 1);

    let slice = actions
        .iter()
        .step_by(step)
        .map(|a| a.get_src_tgt_unchecked().1)
        .map(|p| p.strip_prefix(common_path).unwrap_or(p))
        .collect::<Vec<_>>();

    if slice.len() <= DEFAULT_PREVIEW_AMOUNT {
        println!("Previewing {} files:", slice.len());
    } else {
        println!("Previewing {} of {} files:", slice.len(), length);
    }

    for path in slice {
        println!("{}", path.display());
    }

    println!();
}
