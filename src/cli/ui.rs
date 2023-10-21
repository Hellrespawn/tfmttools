use crate::cli::Config;
use file_history::Action;
use indicatif::{
    ProgressBar as IProgressBar, ProgressDrawTarget, ProgressFinish,
    ProgressStyle,
};

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
    ) -> AudioFileSpinner {
        let spinner = IProgressBar::new(0);

        let template = format!(
            "[{{pos}}/{{len}} {found}/{total}] {{wide_msg}} {{spinner}}",
        );

        let style = ProgressStyle::default_spinner()
            .template(&template)
            .on_finish(ProgressFinish::AtCurrentPos);

        spinner.set_style(style);
        spinner.set_draw_target(ProgressDrawTarget::stdout());
        spinner.set_message(message);

        AudioFileSpinner { spinner }
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
        self.spinner.finish_at_current_pos();
        self.spinner.set_message(message);
    }
}

pub(crate) fn create_progressbar(
    len: u64,
    msg: &'static str,
    finished_msg: &'static str,
    preview: bool,
) -> IProgressBar {
    let bar = IProgressBar::new(len);

    let pp = if preview { Config::PREVIEW_PREFIX } else { "" };

    let template = format!("{pp}[{{pos}}/{{len}}] {{msg}} {{wide_bar}}");

    bar.set_style(ProgressStyle::default_bar().template(&template).on_finish(
        ProgressFinish::WithMessage(std::borrow::Cow::Borrowed(finished_msg)),
    ));
    bar.set_draw_target(ProgressDrawTarget::stdout());
    bar.set_message(msg);

    bar
}

pub(crate) fn print_actions_preview(actions: &[Action], preview_amount: usize) {
    let length = actions.len();

    println!(
        "\nPreviewing {} files:",
        if length <= preview_amount {
            length.to_string()
        } else {
            format!(
                "{}/{}",
                std::cmp::min(preview_amount, actions.len()),
                length
            )
        }
    );

    let step = std::cmp::max(length / preview_amount, 1);

    for action in actions.iter().step_by(step) {
        // FIXME Check actual amount previewed
        let (_, target) = action.get_src_tgt_unchecked();
        println!("{}", target.display());
    }

    println!();
}
