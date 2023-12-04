use ratatui::layout::{Alignment, Direction, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Borders, List, ListItem, Paragraph, Wrap,
};
use ratatui::Frame;

use super::app::PreviewApp;

/// Renders the user interface widgets.
pub(crate) fn render(app: &mut PreviewApp, frame: &mut Frame) {
    frame.render_widget(
        Block::default()
            .title(app.title())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default()),
        frame.size(),
    );

    let inner = frame.size().inner(&Margin::new(2, 2));

    let items = app
        .move_actions()
        .iter()
        .map(|p| ListItem::new(p.to_string()))
        .collect::<Vec<_>>();

    let list = List::new(items)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");

    frame.render_widget(list, inner);

    // frame.render_widget(
    //     Paragraph::new("Press 'y', to move files.")
    //         .style(Style::default())
    //         .alignment(Alignment::Center)
    //         .wrap(Wrap { trim: true }),
    //     inner,
    // );

    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
}
