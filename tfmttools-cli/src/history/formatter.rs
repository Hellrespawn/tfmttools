use std::fmt::Write;

use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecord, ActionRecordMetadata};
use tfmttools_history_core::History;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug)]
pub enum HistoryFormat {
    Normal,
    Verbose,
}

#[derive(Debug)]
pub enum HistoryPrefix {
    Ordered(char),
}

impl HistoryPrefix {
    fn format(&self, index: usize, total: usize) -> String {
        match self {
            // HistoryPrefix::Unordered(list_style) => format!("{list_style} "),
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

    pub fn format_history(
        &self,
        history: &mut impl History<Action, ActionRecordMetadata>,
    ) -> Result<String> {
        let undo = history.get_all_records_to_undo()?;

        let undo_string = self.format_records(&undo);

        let redo = history.get_all_records_to_redo()?;

        let redo_string = self.format_records(&redo);

        if undo_string.is_none() && redo_string.is_none() {
            Ok("There is nothing to undo or redo.".to_owned())
        } else {
            let mut buffer = String::new();

            if let Some(string) = undo_string {
                buffer.push_str("Undo history:\n");
                buffer.push_str(&string);
                buffer.push_str("\n\n");
            } else {
                buffer.push_str("There is nothing to undo.\n\n");
            }

            if let Some(string) = redo_string {
                buffer.push_str("Redo history:\n");
                buffer.push_str(&string);
                buffer.push('\n');
            } else {
                buffer.push_str("There is nothing to redo.\n");
            }

            Ok(buffer)
        }
    }

    pub fn format_records(&self, records: &[&ActionRecord]) -> Option<String> {
        if records.is_empty() {
            None
        } else {
            let string = records
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
                        write!(
                            string,
                            "\n{}{}",
                            " ".repeat(formatted_prefix.len()),
                            line
                        ).expect("Using write! to append to String should never fail.");
                    }

                    string
                } else {
                    self.format_record(record)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
            Some(string)
        }
    }

    pub fn format_record(&self, record: &ActionRecord) -> String {
        let summary = RecordSummary::from_record(record);

        let base_string = format!(
            "{} [#{}] ({})",
            record.timestamp().format(DATE_FORMAT),
            summary.run_id,
            summary
        );

        match self.format {
            HistoryFormat::Normal => base_string,
            HistoryFormat::Verbose => {
                let metadata_string = format!(
                    "'{}' => '{}'",
                    record.metadata().template(),
                    record.metadata().arguments().join(" ")
                );

                format!("┌ {base_string}\n└── {metadata_string}")
            },
        }
    }
}

#[derive(Default)]
pub struct RecordSummary {
    mv: usize,
    cp: usize,
    rm_file: usize,
    mk_dir: usize,
    rm_dir: usize,
    run_id: String,
}

impl RecordSummary {
    pub fn from_record(record: &ActionRecord) -> Self {
        let mut summary = Self {
            run_id: record.metadata().run_id().to_owned(),
            ..Default::default()
        };

        for action in record.iter() {
            match action {
                Action::MoveFile { .. } => summary.mv += 1,
                Action::CopyFile(_) => summary.cp += 1,
                Action::RemoveFile(_) => summary.rm_file += 1,
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

        match self.cp.cmp(&1) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                strings.push("1 file copied".to_string());
            },
            std::cmp::Ordering::Greater => {
                strings.push(format!("{} files copied", self.cp));
            },
        }

        match self.rm_file.cmp(&1) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => {
                strings.push("1 file removed".to_string());
            },
            std::cmp::Ordering::Greater => {
                strings.push(format!("{} files removed", self.rm_file));
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
