use std::io::{BufRead, Write};

use color_eyre::Result;
use indicatif::{
    ProgressBar as IndicatifProgressBar, ProgressDrawTarget, ProgressStyle,
};

use super::config::DRY_RUN_PREFIX;
use crate::TERM;

pub struct ProgressBarOptions {
    style: ProgressStyle,
    draw_target: ProgressDrawTarget,
    working_message: &'static str,
    finished_message: &'static str,
}

impl ProgressBarOptions {
    pub fn bar(
        dry_run: bool,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        Self::new(
            dry_run,
            ProgressStyle::default_bar(),
            "[{pos}/{len}] {msg} {wide_bar}",
            working_message,
            finished_message,
        )
    }

    pub fn spinner(
        dry_run: bool,
        found: &str,
        total: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        Self::new(
            dry_run,
            ProgressStyle::default_spinner(),
            &format!(
                "[{{pos}}/{{len}} {found}/{total} files] {{wide_msg}} {{spinner}}",
            ),
            working_message,
            finished_message,
        )
    }

    pub fn new(
        dry_run: bool,
        style: ProgressStyle,
        template: &str,
        working_message: &'static str,
        finished_message: &'static str,
    ) -> Result<Self> {
        let prefix = if dry_run { DRY_RUN_PREFIX } else { "" };

        let template = format!("{prefix}{template}");

        let style = style.template(&template)?;

        #[cfg(test)]
        let draw_target = ProgressDrawTarget::stdout();

        #[cfg(not(test))]
        let draw_target = ProgressDrawTarget::stderr();

        Ok(Self { style, draw_target, working_message, finished_message })
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
    }
}

pub struct ConfirmationPrompt<'cp> {
    prompt: &'cp str,
    error_prompt: &'cp str,
    default: bool,
}

impl<'cp> ConfirmationPrompt<'cp> {
    const ERROR_PROMPT: &'static str = "Please enter 'y' or 'n' (y/N)";

    pub fn new(prompt: &'cp str) -> Self {
        Self { prompt, error_prompt: Self::ERROR_PROMPT, default: false }
    }

    pub fn prompt(&self) -> Result<bool> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        print!("{} {} ", self.prompt, self.get_options());
        stdout.flush()?;

        loop {
            let mut input = String::new();

            stdin.lock().read_line(&mut input)?;

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                "" => return Ok(self.default),
                _ => {
                    print!("{} {} ", self.error_prompt, self.get_options());
                    stdout.flush()?;
                },
            }
        }
    }

    fn get_options(&self) -> &'static str {
        if self.default {
            "(Y/n)"
        } else {
            "(y/N)"
        }
    }
}

pub struct PreviewList<S, I>
where
    S: ToString,
    I: Iterator<Item = S>,
{
    iter: I,
    total: usize,
    leading_lines: usize,
    trailing_lines: usize,
}

impl<S, I> PreviewList<S, I>
where
    S: ToString,
    I: Iterator<Item = S>,
{
    const MIN_PREVIEW_AMOUNT: usize = 8;

    pub fn new(
        iter: I,
        total: usize,
        leading_lines: usize,
        trailing_lines: usize,
    ) -> Self {
        Self { iter, total, leading_lines, trailing_lines }
    }

    pub fn print(self) {
        let padding = self.leading_lines + self.trailing_lines;

        let preview_amount = std::cmp::max(
            Self::MIN_PREVIEW_AMOUNT,
            TERM.size().0 as usize
                - self.leading_lines
                - self.trailing_lines
                - padding,
        );

        if self.total > preview_amount {
            println!("Previewing {} of {} items:", preview_amount, self.total);
        } else {
            println!("Previewing {} items:", self.total);
        };

        let step = self.total.div_ceil(preview_amount);

        let iter = self
            .iter
            .enumerate()
            .map(|(index, item)| (index + 1, item))
            .step_by(step)
            .take(preview_amount);

        let enumeration_width = self.total.to_string().len();

        for (index, item) in iter {
            print!("{index:>enumeration_width$}) ");

            println!("{}", item.to_string());
        }
    }
}
