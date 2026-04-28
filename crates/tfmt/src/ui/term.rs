use std::sync::LazyLock;

use camino::Utf8PathBuf;
use color_eyre::Result;
use console::Term;
use tfmttools_core::util::Utf8Directory;

static TERM: LazyLock<Term> = LazyLock::new(Term::stdout);

pub fn terminal_width() -> usize {
    TERM.size().0 as usize
}

pub fn terminal_height() -> usize {
    TERM.size().1 as usize
}

/// Make the cursor visible again, ignoring the result.
pub fn show_cursor() {
    let _ = TERM.show_cursor();
}

/// Hide the cursor, ignoring the result.
pub fn hide_cursor() {
    let _ = TERM.hide_cursor();
}

pub fn current_dir_utf8() -> Result<Utf8Directory> {
    let path = std::env::current_dir()?;

    let utf8_path = Utf8PathBuf::try_from(path)?;

    Ok(Utf8Directory::new(utf8_path)?)
}
