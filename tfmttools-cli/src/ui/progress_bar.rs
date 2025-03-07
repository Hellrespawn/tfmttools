use indicatif::{ProgressBar as IndicatifProgressBar, ProgressStyle};

use crate::options::DisplayMode;
use crate::term::{hide_cursor, show_cursor};

pub struct ProgressBar {
    inner: Option<IndicatifProgressBar>,
    finished_message: &'static str,
    println_on_finish: bool,
}

impl ProgressBar {
    pub fn bar(
        display_mode: DisplayMode,
        working_message: &'static str,
        finished_message: &'static str,
        length: u64,
        println_on_finish: bool,
    ) -> Self {
        Self::new(
            display_mode,
            ProgressStyle::default_bar(),
            "[{pos}/{len}] {msg} {wide_bar}",
            working_message,
            finished_message,
            Some(length),
            println_on_finish,
        )
    }

    pub fn spinner(
        display_mode: DisplayMode,
        found: &str,
        total: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Self {
        Self::new(
            display_mode,
            ProgressStyle::default_spinner(),
            &format!(
                "[{{pos}}/{{len}} {found}/{total}] {{wide_msg}} {{spinner}}",
            ),
            working_message,
            finished_message,
            None,
            false,
        )
    }

    // pub fn exact_size(&self) -> bool {
    //     self.length.is_some()
    // }

    fn new(
        display_mode: DisplayMode,
        style: ProgressStyle,
        template: &str,
        working_message: &'static str,
        finished_message: &'static str,
        length: Option<u64>,
        println_on_finish: bool,
    ) -> Self {
        let inner = if matches!(display_mode, DisplayMode::Simple) {
            None
        } else {
            let style = match style.template(template) {
                Ok(style) => style,
                Err(err) => {
                    panic!(
                        "Unable to parse indicatif template: '{template}'\n{err}",
                    )
                },
            };

            hide_cursor();

            Some(
                IndicatifProgressBar::new(length.unwrap_or(0))
                    .with_style(style)
                    .with_message(working_message),
            )
        };

        Self { inner, finished_message, println_on_finish }
    }

    pub fn inc_found(&self) {
        self.inner.as_ref().inspect(|inner| inner.inc(1));
    }

    pub fn inc_total(&self) {
        self.inner.as_ref().inspect(|inner| inner.inc_length(1));
    }

    pub fn finish(self) {
        if let Some(inner) = self.inner {
            inner.set_message(self.finished_message);
            inner.abandon();

            if self.println_on_finish {
                eprintln!();
                eprintln!();
            }

            show_cursor();
        }
    }
}
