use console::Term;
use once_cell::sync::Lazy;

static TERM: Lazy<Term> = Lazy::new(Term::stdout);
// TODO? Accept all lines, but step through it like in preview?
#[derive(Default)]
pub struct Table {
    heading: String,
    body: Vec<String>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn set_heading(&mut self, string: String) {
        self.heading = string;
    }

    pub fn push_string(&mut self, string: &str) {
        self.body
            .extend(string.split('\n').map(std::borrow::ToOwned::to_owned));
    }
}

impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = get_width();

        let fat_bar = "━".repeat(width - 2);
        let bar = "─".repeat(width - 2);

        writeln!(f, "┏{fat_bar}┓")?;

        writeln!(f, "┃ {:<width$} ┃", &self.heading, width = width - 4)?;

        writeln!(f, "┡{fat_bar}┩")?;

        for line in &self.body {
            writeln!(f, "│ {} │", truncate(line, width - 4))?;
        }

        writeln!(f, "└{bar}┘")?;

        Ok(())
    }
}

fn get_width() -> usize {
    TERM.size().1 as usize
}

fn truncate(string: &str, width: usize) -> String {
    if console::measure_text_width(string) <= width {
        format!("{string:<width$}")
    } else {
        let mut truncated = string.to_owned();

        while console::measure_text_width(&truncated) > width - 4 {
            truncated = truncated
                .split(std::path::MAIN_SEPARATOR)
                .skip(1)
                .collect::<Vec<_>>()
                .join(std::path::MAIN_SEPARATOR_STR);
        }

        format!(
            "...{}{truncated:<width$}",
            std::path::MAIN_SEPARATOR,
            width = width - 4
        )
    }
}
