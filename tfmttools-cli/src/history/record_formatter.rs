use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecord;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug)]
pub enum RecordFormat {
    Normal,
    Verbose,
}

#[derive(Debug)]
pub struct RecordFormatter {
    record_format: RecordFormat,
}

impl RecordFormatter {
    pub fn new(record_format: RecordFormat) -> Self {
        Self { record_format }
    }

    pub fn format_record(&self, record: &ActionRecord) -> String {
        let summary = RecordSummary::from_record(record);

        let normal_string =
            format!("{} ({summary})", record.timestamp().format(DATE_FORMAT));

        match self.record_format {
            RecordFormat::Normal => normal_string,
            RecordFormat::Verbose => {
                if let Some(metadata) = record.metadata() {
                    let metadata_string = format!(
                        "'{}' => '{}'",
                        metadata.template(),
                        metadata.arguments().join(" ")
                    );

                    format!("┌ {normal_string}\n└── {metadata_string}")
                } else {
                    normal_string
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
