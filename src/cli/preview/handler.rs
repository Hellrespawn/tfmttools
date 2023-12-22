use crossterm::event::{KeyCode, KeyEvent};

use super::app_state::AppState;

pub fn update(app: &mut AppState, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Enter | KeyCode::Char('y' | 'Y') => app.confirm(),
        _ => {},
    };

    app.quit();
}
