use tfmttools_core::action::Action;
use tfmttools_history::Record;

#[derive(Default)]
pub struct RecordSummary {
    mv: usize,
    mk_dir: usize,
    rm_dir: usize,
}

impl RecordSummary {
    pub fn from_record(record: &Record<Action>) -> Self {
        let mut summary = Self::default();

        for action in record.items() {
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
