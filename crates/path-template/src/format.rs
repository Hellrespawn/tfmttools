use std::sync::LazyLock;

use regex::Regex;

pub(crate) fn year(text: &str) -> Result<String, String> {
    static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        [r"(\d{4})-\d{2}-\d{2}", r"\d{2}-\d{2}-(\d{4})", r"(\d{4})"]
            .iter()
            .map(|pattern| Regex::new(pattern).expect("Built-in date pattern is valid"))
            .collect()
    });
    for pattern in PATTERNS.iter() {
        if let Some(captures) = pattern.captures(text) {
            return Ok(captures[1].to_owned());
        }
    }
    Err(format!("Unable to extract a year from {text:?}"))
}

pub(crate) fn pad(text: &str, width: usize) -> String {
    format!("{text:0>width$}")
}
