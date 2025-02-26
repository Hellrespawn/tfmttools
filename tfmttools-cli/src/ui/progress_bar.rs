use indicatif::{
    ProgressBar as IndicatifProgressBar, ProgressDrawTarget, ProgressStyle,
};

use crate::TERM;
use crate::cli::show_cursor;

pub struct ProgressBar {
    inner: IndicatifProgressBar,
    finished_message: &'static str,
}

impl ProgressBar {
    pub fn bar(
        working_message: &'static str,
        finished_message: &'static str,
        length: u64,
    ) -> Self {
        Self::new(
            ProgressStyle::default_bar(),
            "[{pos}/{len}] {msg} {wide_bar}",
            working_message,
            finished_message,
            Some(length),
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
                "[{{pos}}/{{len}} {found}/{total}] {{wide_msg}} {{spinner}}",
            ),
            working_message,
            finished_message,
            None,
        )
    }

    // pub fn exact_size(&self) -> bool {
    //     self.length.is_some()
    // }

    fn new(
        style: ProgressStyle,
        template: &str,
        working_message: &'static str,
        finished_message: &'static str,
        length: Option<u64>,
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

        // Need to pass Some(..) to indicatif, otherwise it will substitute pos
        // for len.
        let inner = IndicatifProgressBar::with_draw_target(
            Some(length.unwrap_or(0)),
            draw_target,
        )
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

        show_cursor();
    }
}
