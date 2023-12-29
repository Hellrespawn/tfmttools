use tfmttools_history::Record;

use crate::action::Action;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn normalize_separators(string: &str) -> String {
    string
        .split(['\\', '/'])
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

pub fn format_record(record: &Record<Action>) -> String {
    let summary = format_action_summary(record);

    if let Some(timestamp) = record.timestamp() {
        format!("{} ({summary}) ", timestamp.format(DATE_FORMAT))
    } else {
        summary
    }
}

fn format_action_summary(record: &Record<Action>) -> String {
    let no_of_moves = record.items().iter().filter(|a| a.is_move()).count();

    let no_of_mk_dirs = record.items().iter().filter(|a| a.is_mk_dir()).count();

    let no_of_rm_dirs = record.items().iter().filter(|a| a.is_rm_dir()).count();

    let mut strings = Vec::new();

    if no_of_mk_dirs > 1 {
        strings.push(format!("{no_of_mk_dirs} created directories"));
    }

    if no_of_moves > 1 {
        strings.push(format!("{no_of_moves} moved files"));
    }

    if no_of_rm_dirs > 1 {
        strings.push(format!("{no_of_rm_dirs} removed directories"));
    }

    strings.join(", ")
}
