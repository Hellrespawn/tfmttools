use color_eyre::Result;
use indicatif::{
    ProgressBar as IndicatifProgressBar, ProgressDrawTarget, ProgressFinish,
    ProgressStyle,
};

use crate::config::{Config, DRY_RUN_PREFIX};

pub(crate) mod table;

pub(crate) fn print_error(error: &color_eyre::Report) {
    println!("An error occurred:\n{error}");
}

pub(crate) struct ProgressBarOptions {
    style: ProgressStyle,
    draw_target: ProgressDrawTarget,
    working_message: &'static str,
    finished_message: &'static str,
}

impl ProgressBarOptions {
    pub(crate) fn bar(
        config: &Config,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        Self::new(
            config,
            ProgressStyle::default_bar(),
            &format!("[{{pos}}/{{len}}] {{msg}} {{wide_bar}}"),
            working_message,
            finished_message,
        )
    }

    pub(crate) fn spinner(
        config: &Config,
        found: &str,
        total: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        Self::new(
            config,
            ProgressStyle::default_spinner(),
            &format!(
                "[{{pos}}/{{len}} {found}/{total} files] {{wide_msg}} {{spinner}}",
            ),
            working_message,
            finished_message,
        )
    }

    pub(crate) fn new(
        config: &Config,
        style: ProgressStyle,
        template: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        let prefix = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

        let template = format!("{prefix}{template}");

        let style = style.template(&template)?;

        #[cfg(test)]
        let draw_target = ProgressDrawTarget::stdout();

        #[cfg(not(test))]
        let draw_target = ProgressDrawTarget::stderr();

        Ok(Self { style, draw_target, working_message, finished_message })
    }
}

pub(crate) struct ProgressBar {
    inner: IndicatifProgressBar,
    finished_message: &'static str,
}

impl ProgressBar {
    pub(crate) fn new(options: ProgressBarOptions) -> Self {
        Self::with_length(options, 0)
    }

    pub(crate) fn with_length(
        options: ProgressBarOptions,
        length: u64,
    ) -> Self {
        let inner = IndicatifProgressBar::new(length)
            .with_finish(ProgressFinish::Abandon);

        let ProgressBarOptions {
            style,
            working_message,
            finished_message,
            draw_target,
        } = options;

        inner.set_style(style);
        inner.set_message(working_message);
        inner.set_draw_target(draw_target);

        Self { inner, finished_message }
    }

    pub(crate) fn inc_found(&self) {
        self.inner.inc(1);
    }

    pub(crate) fn inc_total(&self) {
        self.inner.inc_length(1);
        self.inner.tick();
    }

    pub(crate) fn finish(&self) {
        self.inner.set_message(self.finished_message);
        self.inner.finish();
    }
}
