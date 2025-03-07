pub fn normalize_separators(string: &str) -> String {
    string
        .split(['\\', '/'])
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

#[derive(Debug, Default, Copy, Clone)]
pub enum ActionMode {
    #[default]
    Default,
    DryRun,
}
#[derive(Debug, Default, Copy, Clone)]
pub enum MoveMode {
    #[default]
    Auto,
    AlwaysCopy,
}
