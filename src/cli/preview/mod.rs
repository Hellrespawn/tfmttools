use color_eyre::Result;
use ratatui::prelude::{CrosstermBackend, Terminal as CrosstermTerminal};
use tracing::trace;

use self::app_state::AppState;
use self::event::{Event, EventHandler};
use self::terminal::Tui;

mod app_data;
mod app_state;
mod event;
mod handler;
mod terminal;
mod ui;

pub use app_data::{PreviewData, RenameData, UndoRedoData};

pub fn preview(data: &PreviewData) -> Result<bool> {
    trace!("Running preview TUI:\n{:#?}", data);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal: CrosstermTerminal<CrosstermBackend<std::io::Stderr>> =
        CrosstermTerminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    let mut state = AppState::new();

    // Start the main loop.
    while state.is_running() {
        // Render the user interface.
        tui.draw(&mut state, data)?;
        // Handle events.
        match tui.events.next()? {
            Event::Key(key_event) => handler::update(&mut state, key_event),
            Event::Tick
            | Event::Mouse(_)
            | Event::Paste(_)
            | Event::Resize(_, _) => {},
        };
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(state.confirmed())
}
