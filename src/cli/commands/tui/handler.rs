use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::App;

pub(crate) fn update(app: &mut App, key_event: KeyEvent) -> Result<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => app.quit(),

        // Exit application on `Ctrl-C`
        KeyCode::Char('c' | 'C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        },
        // KeyCode::Right | KeyCode::Char('j') => app.increment_counter(),
        // KeyCode::Left | KeyCode::Char('k') => app.decrement_counter(),
        _ => {},
    };

    Ok(())
}
