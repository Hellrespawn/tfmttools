use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{
    Block, BorderType, Borders, List, ListItem, Paragraph, Wrap,
};
use ratatui::{symbols, Frame};

use super::app::{PreviewApp, PreviewMode};
use crate::cli::util::rows_required_for_string;

const PRESS_Y_NOTIF: &str = "Press 'y' to rename or any other key to cancel.";

const INTERMEDIATE_BORDER_SET: symbols::border::Set = symbols::border::Set {
    top_left: symbols::line::THICK.vertical_right,
    top_right: symbols::line::THICK.vertical_left,
    ..symbols::border::THICK
};

/// Renders the user interface widgets.
pub fn render(app: &mut PreviewApp, frame: &mut Frame) {
    match app.mode() {
        PreviewMode::Rename(_) => render_rename(app, frame),
        PreviewMode::Undo => todo!(),
        PreviewMode::Redo => todo!(),
    }
}

/// Renders the user interface widgets.
pub fn render_rename(app: &mut PreviewApp, frame: &mut Frame) {
    let arguments_string = create_arguments_string(app.as_rename().arguments());
    let arguments_string_rows = calculate_string_rows(frame, &arguments_string);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            // border + blank + heading + arguments + blank
            Constraint::Length(1 + 1 + 1 + arguments_string_rows + 1),
            Constraint::Min(8),
            // border + notif + blank
            Constraint::Length(calculate_string_rows(frame, PRESS_Y_NOTIF) + 2),
        ])
        .split(frame.size());

    let arguments_pane = layout[0];
    let preview_pane = layout[1];
    let notification_pane = layout[2];

    render_title_and_arguments(app, frame, arguments_pane, &arguments_string);
    render_preview(app, frame, preview_pane);
    render_notification(frame, notification_pane);
}

fn render_title_and_arguments(
    app: &PreviewApp,
    frame: &mut Frame,
    pane: Rect,
    arguments_string: &str,
) {
    let p = Paragraph::new(format!("\nArguments:\n{arguments_string}\n"))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                .border_type(BorderType::Thick)
                .title(app.as_rename().title())
                .title_alignment(Alignment::Center),
        );

    frame.render_widget(p, pane);
}

fn render_preview(app: &mut PreviewApp, frame: &mut Frame, pane: Rect) {
    // height - 2 borders - 2 rows
    let amount_of_items = pane.height as usize - 2;

    let step = app.as_rename().move_actions().len() / amount_of_items;

    // TODO Scroll these left-to-right
    let items = app
        .as_rename()
        .move_actions()
        .iter()
        .step_by(step)
        .take(amount_of_items)
        .map(|move_action| {
            move_action
                .target()
                .strip_prefix(app.as_rename().working_directory())
                .unwrap_or(move_action.target())
        })
        .map(|path| ListItem::new(format!(" {path} ")))
        .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .border_set(INTERMEDIATE_BORDER_SET)
            .borders(Borders::ALL)
            .title(format!(
                " Previewing {} of {} ",
                amount_of_items,
                app.as_rename().move_actions().len()
            ))
            .title_alignment(Alignment::Center),
    );

    frame.render_widget(list, pane);
}

fn render_notification(frame: &mut Frame, pane: Rect) {
    let p = Paragraph::new(PRESS_Y_NOTIF)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .border_type(BorderType::Thick)
                .borders(Borders::ALL),
        );

    frame.render_widget(p, pane);
}

fn create_arguments_string(arguments: &[String]) -> String {
    let elements = arguments
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{i} => \"{a}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{elements}]")
}

fn calculate_string_rows(frame: &Frame, string: &str) -> u16 {
    let width = frame.size().width as usize;

    rows_required_for_string(string, width)
        .try_into()
        .expect("String requires more than u17 rows.")
}
