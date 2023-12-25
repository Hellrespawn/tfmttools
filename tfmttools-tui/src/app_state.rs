#[derive(Debug)]
pub struct AppState {
    is_running: bool,
    confirmed: bool,
}
impl AppState {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self { is_running: true, confirmed: false }
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

    pub fn confirmed(&self) -> bool {
        self.confirmed
    }
}
