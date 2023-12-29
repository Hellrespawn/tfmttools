use tfmttools_core::action::Action;
use tfmttools_history::Record;

use super::record_summary::RecordSummary;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct ActionRecord<'ar>(&'ar Record<Action>);

impl<'ar> ActionRecord<'ar> {
    pub fn from_record(record: &'ar Record<Action>) -> Self {
        ActionRecord(record)
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Action> {
        self.0.iter()
    }
}

impl<'ar> std::fmt::Display for ActionRecord<'ar> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let summary = RecordSummary::from_record(self.0);

        if let Some(timestamp) = self.0.timestamp() {
            write!(f, "{} ({summary})", timestamp.format(DATE_FORMAT))
        } else {
            summary.fmt(f)
        }
    }
}
