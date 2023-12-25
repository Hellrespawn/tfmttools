use crate::TERM;

#[derive(Default)]
pub struct Table {
    heading: String,
    body: Vec<String>,
    separate_lines: bool,
}

impl Table {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn set_heading(&mut self, string: String) {
        self.heading = string;
    }

    pub fn push_string(&mut self, string: String) {
        self.body.push(string);
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
