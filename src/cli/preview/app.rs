use camino::{Utf8Path, Utf8PathBuf};

use crate::template::Template;

#[derive(Debug)]
pub(crate) struct PreviewApp<'templates, 'source, 'move_actions> {
    is_running: bool,
    confirmed: bool,

    // recurse: usize,
    // source_directory: Option<Utf8PathBuf>,
    // target_directory: Option<Utf8PathBuf>,
    template: Option<Template<'templates, 'source>>,
    // arguments: Vec<String>,
    move_actions: &'move_actions [&'move_actions Utf8Path],
}
impl<'templates, 'source, 'move_actions>
    PreviewApp<'templates, 'source, 'move_actions>
{
    /// Constructs a new instance of [`App`].
    pub fn new(move_actions: &'move_actions [&'move_actions Utf8Path]) -> Self {
        Self {
            is_running: true,
            confirmed: false,
            // recurse: 0,
            // source_directory: None,
            // target_directory: None,
            template: None,
            // arguments: Vec::new(),
            move_actions,
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.is_running = false;
    }

    /// Set running to false to quit the application.
    pub fn confirm(&mut self) {
        self.confirmed = true;
    }

    pub(crate) fn title(&self) -> String {
        format!(
            " {} ",
            self.template
                .as_ref()
                .map_or(env!("CARGO_PKG_NAME"), |t| t.name())
                .to_owned()
        )
    }

    pub(crate) fn confirmed(&self) -> bool {
        self.confirmed
    }

    pub(crate) fn move_actions(&self) -> &[&Utf8Path] {
        self.move_actions
    }
}
