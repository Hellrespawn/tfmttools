use camino::Utf8Path;

use crate::action::Move;

#[derive(Debug)]
pub(crate) struct PreviewApp<'pa> {
    is_running: bool,
    confirmed: bool,

    title: &'pa str,
    arguments: &'pa [String],
    move_actions: &'pa [Move],
    working_directory: &'pa Utf8Path,
}
impl<'pa> PreviewApp<'pa> {
    /// Constructs a new instance of [`App`].
    pub(crate) fn new(
        title: &'pa str,
        arguments: &'pa [String],
        move_actions: &'pa [Move],
        working_directory: &'pa Utf8Path,
    ) -> Self {
        Self {
            is_running: true,
            confirmed: false,
            title,
            arguments,
            move_actions,
            working_directory,
        }
    }

    /// Handles the tick event of the terminal.
    // pub fn tick(&self) {}

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
        format!(" {} ", self.title)
    }

    pub(crate) fn arguments(&self) -> &[String] {
        self.arguments
    }

    pub(crate) fn confirmed(&self) -> bool {
        self.confirmed
    }

    pub(crate) fn move_actions(&self) -> &[Move] {
        self.move_actions
    }

    pub(crate) fn working_directory(&self) -> &Utf8Path {
        self.working_directory
    }
}
