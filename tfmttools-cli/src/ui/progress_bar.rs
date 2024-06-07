use indicatif::{
    ProgressBar as IndicatifProgressBar, ProgressDrawTarget, ProgressStyle,
};

use crate::TERM;

pub struct ProgressBarOptions {
    style: ProgressStyle,
    draw_target: ProgressDrawTarget,
    working_message: &'static str,
    finished_message: &'static str,
}

impl ProgressBarOptions {
    pub fn bar(
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Self {
        Self::new(
            ProgressStyle::default_bar(),
            "[{pos}/{len}] {msg} {wide_bar}",
            working_message,
            finished_message,
        )
    }

    pub fn spinner(
        found: &str,
        total: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Self {
        Self::new(
            ProgressStyle::default_spinner(),
            &format!(
                "[{{pos}}/{{len}} {found}/{total} files] {{wide_msg}} {{spinner}}",
            ),
            working_message,
            finished_message,
        )
    }

    pub fn new(
        style: ProgressStyle,
        template: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Self {
        let style = match style.template(template) {
            Ok(style) => style,
            Err(err) => {
                panic!(
                    "Unable to parse indicatif template: '{template}'\n{err}",
                )
            },
        };

        #[cfg(test)]
        let draw_target = ProgressDrawTarget::stdout();

        #[cfg(not(test))]
        let draw_target = ProgressDrawTarget::stderr();

        let _ = TERM.hide_cursor();

        Self { style, draw_target, working_message, finished_message }
    }
}

pub struct ProgressBar {
    inner: IndicatifProgressBar,
    finished_message: &'static str,
}

impl ProgressBar {
    pub fn new(options: ProgressBarOptions) -> Self {
        Self::with_length(options, 0)
    }

    pub fn with_length(options: ProgressBarOptions, length: u64) -> Self {
        let ProgressBarOptions {
            style,
            working_message,
            finished_message,
            draw_target,
        } = options;

        let inner =
            IndicatifProgressBar::with_draw_target(Some(length), draw_target)
                .with_style(style)
                .with_message(working_message);

        Self { inner, finished_message }
    }

    pub fn inc_found(&self) {
        self.inner.inc(1);
    }

    pub fn inc_total(&self) {
        self.inner.inc_length(1);
    }

    pub fn finish(&self) {
        self.inner.set_message(self.finished_message);
        self.inner.abandon();

        let _ = TERM.show_cursor();
    }
}
