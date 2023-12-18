use color_eyre::Result;
use ratatui::prelude::{CrosstermBackend, Terminal as CrosstermTerminal};

use self::app::PreviewApp;
use self::event::{Event, EventHandler};
use self::terminal::Tui;

mod event;
mod handler;
mod terminal;
mod ui;

pub mod app;

pub fn preview(mut app: PreviewApp<'_>) -> Result<bool> {
    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal: CrosstermTerminal<CrosstermBackend<std::io::Stderr>> =
        CrosstermTerminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.is_running() {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Key(key_event) => handler::update(&mut app, key_event),
            Event::Tick
            | Event::Mouse(_)
            | Event::Paste(_)
            | Event::Resize(_, _) => {},
        };
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(app.confirmed())
}
