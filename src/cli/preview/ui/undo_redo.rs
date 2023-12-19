use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{
    add_top_border_to_paragraph, calculate_string_rows, create_layout,
    render_list, render_notification,
};
use crate::cli::preview::UndoRedoData;
use crate::cli::HistoryMode;

/// Renders the user interface widgets.
pub fn render_undo_redo(
    data: &UndoRedoData,
    frame: &mut Frame,
    mode: HistoryMode,
) {
    let amount = data.amount();
    let actual = data.actual();

    let amount_notif_string = if actual == amount {
        format!("{}ing {amount} runs.", mode.verb_capitalized())
    } else {
        format!(
            "Tried to {verb} {amount} runs, but only {actual} can be {verb}ne.",
            verb = mode.verb()
        )
    };

    let amount_notif_string_rows =
        calculate_string_rows(frame, &amount_notif_string);

    let notification =
        format!("Press 'y' to {} or any other key to cancel.", mode.verb());

    let (amount_pane, preview_pane, notification_pane) = create_layout(
        frame,
        // border + blank + string + blank
        1 + 1 + amount_notif_string_rows + 1,
        // border + notif + blank
        calculate_string_rows(frame, &notification) + 2,
    );

    render_title_and_arguments(frame, amount_pane, &amount_notif_string, mode);

    render_preview(data, frame, preview_pane);

    render_notification(frame, notification_pane, &notification);
}

fn render_title_and_arguments(
    frame: &mut Frame,
    pane: Rect,
    amount_string: &str,
    mode: HistoryMode,
) {
    let paragraph = Paragraph::new(format!("\n{amount_string}\n"))
        .alignment(Alignment::Center);

    let p = add_top_border_to_paragraph(
        paragraph,
        mode.verb_capitalized().to_owned(),
    );

    frame.render_widget(p, pane);
}

fn render_preview(data: &UndoRedoData, frame: &mut Frame, pane: Rect) {
    let iter = data.records().iter().map(|record| {
        if let Some(timestamp) = record.timestamp() {
            format!("{} actions ({})", record.len(), timestamp)
        } else {
            format!("{} actions", record.len())
        }
    });

    render_list(frame, pane, data.actual(), iter);
}
