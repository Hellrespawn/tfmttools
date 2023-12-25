use once_cell::sync::Lazy;
use tfmttools_history::Record;
use time::format_description::{self, FormatItem};

use crate::action::Action;

static DATE_FORMAT: Lazy<Vec<FormatItem>> = Lazy::new(|| {
    format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .expect("Unable to parse date format.")
});

pub fn normalize_separators(string: &str) -> String {
    string
        .split(['\\', '/'])
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

pub fn format_record(record: &Record<Action>) -> String {
    let no_of_moves = record.items().iter().filter(|a| a.is_move()).count();

    let no_of_mk_dirs = record.items().iter().filter(|a| a.is_mk_dir()).count();

    let no_of_rm_dirs = record.items().iter().filter(|a| a.is_rm_dir()).count();

    let mut string = format!("[{}] ", record.len());

    if no_of_mk_dirs > 1 {
        string += &format!("{no_of_mk_dirs} created directories, ");
    }

    if no_of_moves > 1 {
        string += &format!("{no_of_moves} moved files, ");
    }

    if no_of_rm_dirs > 1 {
        string += &format!("{no_of_rm_dirs} removed directories, ");
    }

    string = string[0..string.len() - 2].to_owned();

    if let Some(timestamp) = record.timestamp() {
        string += &format!(
            " ({})",
            timestamp
                .format(&DATE_FORMAT)
                .expect("Unable to format timestamp.")
        );
    }

    string
}
