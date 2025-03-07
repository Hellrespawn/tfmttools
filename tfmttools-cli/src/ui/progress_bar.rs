use indicatif::{
    ProgressBar as IndicatifProgressBar, ProgressDrawTarget, ProgressStyle,
};

use crate::term::{hide_cursor, show_cursor};

pub struct ProgressBar {
    inner: IndicatifProgressBar,
    finished_message: &'static str,
    println_on_finish: bool,
}

impl ProgressBar {
    pub fn bar(
        working_message: &'static str,
        finished_message: &'static str,
        length: u64,
        println_on_finish: bool,
    ) -> Self {
        Self::new(
            ProgressStyle::default_bar(),
            "[{pos}/{len}] {msg} {wide_bar}",
            working_message,
            finished_message,
            Some(length),
            println_on_finish,
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
            false,
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
        println_on_finish: bool,
    ) -> Self {
        let style = match style.template(template) {
            Ok(style) => style,
            Err(err) => {
                panic!(
                    "Unable to parse indicatif template: '{template}'\n{err}",
                )
            },
        };
        let draw_target = ProgressDrawTarget::stderr();

        hide_cursor();

        // Need to pass Some(..) to indicatif, otherwise it will substitute pos
        // for len.
        let inner = IndicatifProgressBar::with_draw_target(
            Some(length.unwrap_or(0)),
            draw_target,
        )
        .with_style(style)
        .with_message(working_message);

        Self { inner, finished_message, println_on_finish }
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

        if self.println_on_finish {
            eprintln!();
            eprintln!();
        }

        show_cursor();
    }
}
