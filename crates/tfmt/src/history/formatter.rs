use std::fmt::Write;

use color_eyre::Result;
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    ActionRecord, ActionRecordMetadata, TemplateMetadata,
};
use tfmttools_history::History;

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
        history: &History<Action, ActionRecordMetadata>,
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

    pub fn format_records(&self, records: &[ActionRecord]) -> Option<String> {
        if records.is_empty() {
            None
        } else {
            let string = records
                .iter()
                .enumerate()
                .map(|(i, record)| {
                    self.format_record_entry(record, i, records.len())
                })
                .collect::<Vec<_>>()
                .join("\n");
            Some(string)
        }
    }

    fn format_record_entry(
        &self,
        record: &ActionRecord,
        index: usize,
        total: usize,
    ) -> String {
        if let Some(prefix) = &self.prefix {
            self.format_record_with_prefix(record, prefix, index, total)
        } else {
            self.format_record(record)
        }
    }

    fn format_record_with_prefix(
        &self,
        record: &ActionRecord,
        prefix: &HistoryPrefix,
        index: usize,
        total: usize,
    ) -> String {
        let formatted_prefix = prefix.format(index, total);
        let formatted_record = self.format_record(record);

        let mut lines = formatted_record.lines();
        let mut string = format!(
            "{formatted_prefix}{}",
            lines
                .next()
                .expect("format_record should not return an empty string")
        );

        for line in lines {
            write!(string, "\n{}{}", " ".repeat(formatted_prefix.len()), line)
                .expect("Using write! to append to String should never fail.");
        }

        string
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
                    Self::format_template_name(record.metadata().template()),
                    record.metadata().arguments().join(" ")
                );

                format!("┌ {base_string}\n└── {metadata_string}")
            },
        }
    }

    pub fn format_template_name(metadata: &TemplateMetadata) -> String {
        match metadata {
            TemplateMetadata::FileOrName(file_or_name) => {
                file_or_name.to_owned()
            },
            TemplateMetadata::Script(_) => "script".to_owned(),
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
                Action::CopyFile { .. } => summary.cp += 1,
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

        push_summary_part(
            &mut strings,
            self.mk_dir,
            "directory created",
            "directories created",
        );
        push_summary_part(&mut strings, self.mv, "file moved", "files moved");
        push_summary_part(&mut strings, self.cp, "file copied", "files copied");
        push_summary_part(
            &mut strings,
            self.rm_file,
            "file removed",
            "files removed",
        );
        push_summary_part(
            &mut strings,
            self.rm_dir,
            "directory removed",
            "directories removed",
        );

        write!(f, "{}", strings.join(", "))
    }
}

fn push_summary_part(
    strings: &mut Vec<String>,
    amount: usize,
    singular: &str,
    plural: &str,
) {
    match amount {
        0 => (),
        1 => strings.push(format!("1 {singular}")),
        amount => strings.push(format!("{amount} {plural}")),
    }
}
