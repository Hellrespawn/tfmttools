use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionHistory, ActionRecord};

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug)]
pub enum HistoryFormat {
    Normal,
    Verbose,
}

#[derive(Debug)]
pub enum HistoryPrefix {
    Unordered(char),
    Ordered(char),
}

impl HistoryPrefix {
    fn format(&self, index: usize, total: usize) -> String {
        match self {
            HistoryPrefix::Unordered(list_style) => format!("{list_style} "),
            HistoryPrefix::Ordered(list_style) => {
                let width = total.to_string().len();

                format!("{:>width$}{} ", index + 1, list_style)
            },
        }
    }
}

#[derive(Debug)]
pub struct HistoryFormatter {
    format: HistoryFormat,
    prefix: Option<HistoryPrefix>,
}

impl HistoryFormatter {
    pub fn new() -> Self {
        Self { format: HistoryFormat::Normal, prefix: None }
    }

    pub fn with_format(mut self, format: HistoryFormat) -> Self {
        self.format = format;

        self
    }

    pub fn with_prefix(mut self, prefix: HistoryPrefix) -> Self {
        self.prefix = Some(prefix);

        self
    }

    pub fn format_history(&self, history: &ActionHistory) -> String {
        let undo = history.get_records_to_undo().collect::<Vec<_>>();

        let redo = history.get_records_to_redo().collect::<Vec<_>>();

        if undo.is_empty() && redo.is_empty() {
            "There is nothing to undo or redo".to_owned()
        } else {
            let mut buffer = String::new();

            buffer.push_str("Undo history:\n");

            if undo.is_empty() {
                buffer.push_str("There is nothing to undo.");
            } else {
                buffer.push_str(&self.format_records(&undo));
            }

            buffer.push_str("\n\nRedo history:\n");

            if redo.is_empty() {
                buffer.push_str("There is nothing to redo.");
            } else {
                buffer.push_str(&self.format_records(&redo));
            }

            buffer
        }
    }

    pub fn format_records(&self, records: &[&ActionRecord]) -> String {
        records
            .iter()
            .enumerate()
            .map(|(i, record)| {
                if let Some(prefix) = &self.prefix {
                    let formatted_prefix = prefix.format(i, records.len());

                    let formatted_record = self.format_record(record);

                    let mut iter = formatted_record.lines();

                    let mut string =
                        format!("{formatted_prefix}{}", iter.next().unwrap());

                    for line in iter {
                        string += &format!(
                            "\n{}{}",
                            " ".repeat(formatted_prefix.len()),
                            line
                        );
                    }

                    string
                } else {
                    self.format_record(record)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn format_record(&self, record: &ActionRecord) -> String {
        let summary = RecordSummary::from_record(record);

        let base_string =
            format!("{} ({summary})", record.timestamp().format(DATE_FORMAT));

        match self.format {
            HistoryFormat::Normal => base_string,
            HistoryFormat::Verbose => {
                if let Some(metadata) = record.metadata() {
                    let metadata_string = format!(
                        "'{}' => '{}'",
                        metadata.template(),
                        metadata.arguments().join(" ")
                    );

                    format!("┌ {base_string}\n└── {metadata_string}")
                } else {
                    base_string
                }
            },
        }
    }
}

#[derive(Default)]
pub struct RecordSummary {
    mv: usize,
    mk_dir: usize,
    rm_dir: usize,
}

impl RecordSummary {
    pub fn from_record(record: &ActionRecord) -> Self {
        let mut summary = Self::default();

        for action in record.iter() {
            match action {
                Action::Move { .. } => summary.mv += 1,
                Action::MakeDir(_) => summary.mk_dir += 1,
                Action::RemoveDir(_) => summary.rm_dir += 1,
            }
        }

        summary
    }
}

impl std::fmt::Display for RecordSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut strings = Vec::new();

        match self.mk_dir.cmp(&1) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                strings.push("1 directory created".to_string());
            },
            std::cmp::Ordering::Greater => {
                strings.push(format!("{} directories created", self.mk_dir));
            },
        }

        match self.mv.cmp(&1) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                strings.push("1 file moved".to_string());
            },
            std::cmp::Ordering::Greater => {
                strings.push(format!("{} files moved", self.mv));
            },
        }

        match self.rm_dir.cmp(&1) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                strings.push("1 directory removed".to_string());
            },
            std::cmp::Ordering::Greater => {
                strings.push(format!("{} directories removed", self.rm_dir));
            },
        }

        write!(f, "{}", strings.join(", "))
    }
}
