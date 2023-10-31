use camino::Utf8Path;
use console::Term;
use once_cell::sync::Lazy;

static TERM: Lazy<Term> = Lazy::new(Term::stdout);
// TODO? Accept all lines, but step through it like in actions preview?
#[derive(Default)]
pub(crate) struct Table {
    heading: String,
    body: Vec<String>,
    separate_lines: bool,
}

impl Table {
    pub(crate) fn new() -> Self {
        Self { ..Default::default() }
    }

    pub(crate) fn set_heading(&mut self, string: String) {
        self.heading = string;
    }

    pub(crate) fn push_string(&mut self, string: String) {
        self.body.push(string);
    }

    pub(crate) fn push_path(&mut self, path: &Utf8Path) {
        self.push_string(Self::truncate_path(path));
    }

    fn truncate_path(path: &Utf8Path) -> String {
        let width = Self::get_width() - 4;

        let string = path.to_string();

        if console::measure_text_width(string.as_ref()) <= width {
            format!("{string:<width$}")
        } else {
            let ellipsis = format!("...{}", std::path::MAIN_SEPARATOR,);

            let mut truncated = string.to_string();

            while console::measure_text_width(&truncated)
                > width - console::measure_text_width(&ellipsis)
            {
                truncated = truncated
                    .split(std::path::MAIN_SEPARATOR)
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(std::path::MAIN_SEPARATOR_STR);
            }

            format!("{}{truncated:<width$}", ellipsis, width = width - 4)
        }
    }

    fn get_width() -> usize {
        TERM.size().1 as usize
    }
}

impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = Self::get_width();

        let fat_bar = "━".repeat(width - 2);
        let bar = "─".repeat(width - 2);

        writeln!(f, "┏{fat_bar}┓")?;

        for line in textwrap::wrap(&self.heading, width - 4) {
            writeln!(f, "┃ {:^width$} ┃", line, width = width - 4)?;
        }

        if self.body.is_empty() {
            writeln!(f, "┗{fat_bar}┛")?;
        } else {
            writeln!(f, "┡{fat_bar}┩")?;

            let mut body_iter = self.body.iter().peekable();

            while let Some(string) = body_iter.next() {
                for line in textwrap::wrap(string, width - 4) {
                    writeln!(f, "│ {:<width$} │", line, width = width - 4)?;
                }

                if self.separate_lines && body_iter.peek().is_some() {
                    writeln!(f, "├{bar}┤")?;
                }
            }

            writeln!(f, "└{bar}┘")?;
        }

        Ok(())
    }
}
