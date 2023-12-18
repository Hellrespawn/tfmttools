use crossterm::event::{KeyCode, KeyEvent};

use super::app_state::AppState;

pub fn update(app: &mut AppState, key_event: KeyEvent) {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('y' | 'Y') => app.confirm(),
        _ => {},
    };

    app.quit();
}
