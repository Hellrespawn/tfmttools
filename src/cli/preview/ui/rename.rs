use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{
    add_top_border_to_paragraph, calculate_string_rows, create_layout,
    render_list, render_notification,
};
use crate::cli::preview::RenameData;

const PRESS_Y_NOTIF: &str =
    "Press Enter/return or 'y' to rename, or any other key to cancel.";

/// Renders the user interface widgets.
pub fn render_rename(data: &RenameData, frame: &mut Frame) {
    let arguments_string = create_arguments_string(data.arguments());
    let arguments_string_rows = calculate_string_rows(frame, &arguments_string);

    let (arguments_pane, preview_pane, notification_pane) = create_layout(
        frame,
        // border + blank + heading + arguments + blank
        1 + 1 + 1 + arguments_string_rows + 1,
        // border + notif + blank
        calculate_string_rows(frame, PRESS_Y_NOTIF) + 2,
    );

    render_title_and_arguments(data, frame, arguments_pane, &arguments_string);
    render_preview(data, frame, preview_pane);
    render_notification(frame, notification_pane, PRESS_Y_NOTIF);
}

fn render_title_and_arguments(
    data: &RenameData,
    frame: &mut Frame,
    pane: Rect,
    arguments_string: &str,
) {
    let paragraph =
        Paragraph::new(format!("\nArguments:\n{arguments_string}\n"))
            .alignment(Alignment::Center);

    let p = add_top_border_to_paragraph(paragraph, data.title());

    frame.render_widget(p, pane);
}

fn render_preview(data: &RenameData, frame: &mut Frame, pane: Rect) {
    let total = data.move_actions().len();

    let iter = data.move_actions().iter().map(|move_action| {
        move_action
            .target()
            .strip_prefix(data.working_directory())
            .unwrap_or(move_action.target())
    });

    render_list(frame, pane, total, iter);
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
