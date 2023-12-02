use ratatui::layout::{Alignment, Direction, Layout, Margin};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::app::App;

/// Renders the user interface widgets.
pub(crate) fn render(app: &mut App, frame: &mut Frame) {
    frame.render_widget(
        Block::default()
            .title(app.screen().title())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(Color::Cyan).bg(Color::Black)),
        frame.size(),
    );

    let inner = frame.size().inner(&Margin::new(2, 2));

    frame.render_widget(
        Paragraph::new("Press 'Esc', 'q', or 'Ctrl-c' to quit.")
            .style(Style::default().fg(Color::Red).bg(Color::Black))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        inner,
    );

    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
}
