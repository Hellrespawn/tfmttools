pub struct RecordEntity {
    id: isize,
    state: isize,
    datetime: String,
    superseded_by_id: Option<isize>,
}

pub struct ActionEntity {
    id: isize,
    action_type: String,
    target: String,
    source: Option<String>,
    record_id: isize,
}
