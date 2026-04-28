use camino::Utf8Path;
use color_eyre::Result;

use super::RenameSession;
use crate::cli::ConfirmMode;
use crate::ui::ConfirmationPrompt;

pub fn confirm(session: &RenameSession, prompt: &str) -> Result<bool> {
    Ok(matches!(session.app_options().confirm_mode(), ConfirmMode::NoConfirm)
        || ConfirmationPrompt::new(prompt).prompt()?)
}

#[must_use]
pub fn strip_path_prefix(path: &Utf8Path, prefix: &Utf8Path) -> String {
    let path = path.strip_prefix(prefix).unwrap_or(path);

    if path.is_relative() {
        format!(".{}{path}", std::path::MAIN_SEPARATOR)
    } else {
        format!("{path}")
    }
}
