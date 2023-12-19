use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{
    Block, BorderType, Borders, List, ListItem, Paragraph, Wrap,
};
use ratatui::{symbols, Frame};
use tracing::debug;

use super::app_data::PreviewData;
use super::app_state::AppState;
use crate::cli::util::rows_required_for_string;
use crate::cli::HistoryMode;

mod rename;
mod undo_redo;

pub const INTERMEDIATE_BORDER_SET: symbols::border::Set =
    symbols::border::Set {
        top_left: symbols::line::THICK.vertical_right,
        top_right: symbols::line::THICK.vertical_left,
        ..symbols::border::THICK
    };

/// Renders the user interface widgets.
pub fn render(_app: &mut AppState, data: &PreviewData, frame: &mut Frame) {
    match data {
        PreviewData::Rename(data) => rename::render_rename(data, frame),
        PreviewData::Undo(data) => {
            undo_redo::render_undo_redo(data, frame, HistoryMode::Undo);
        },
        PreviewData::Redo(data) => {
            undo_redo::render_undo_redo(data, frame, HistoryMode::Redo);
        },
    }
}

fn create_layout(
    frame: &Frame,
    top_rows: u16,
    bottom_rows: u16,
) -> (Rect, Rect, Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            // border + blank + heading + arguments + blank
            Constraint::Length(top_rows),
            Constraint::Min(8),
            // border + notif + blank
            Constraint::Length(bottom_rows),
        ])
        .split(frame.size());

    let top_pane = layout[0];
    let preview_pane = layout[1];
    let bottom_pane = layout[2];

    (top_pane, preview_pane, bottom_pane)
}

fn render_notification(frame: &mut Frame, pane: Rect, text: &str) {
    let p = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .border_type(BorderType::Thick)
                .borders(Borders::ALL),
        );

    frame.render_widget(p, pane);
}

fn render_list<'i, S, I>(
    frame: &mut Frame,
    pane: Rect,
    total_items: usize,
    iter: I,
) where
    S: ToString + 'i,
    I: Iterator<Item = S>,
{
    // height - 2 borders - 2 rows
    let rows = pane.height as usize - 2;

    debug!("total: {}, rows: {}", total_items, rows);

    let step = total_items.div_ceil(rows);

    // TODO Scroll these left-to-right
    let items = iter
        .step_by(step)
        .take(rows)
        .map(|s: S| ListItem::new(format!(" {} ", s.to_string())))
        .collect::<Vec<_>>();

    let title = if total_items > rows {
        format!(" Previewing {rows} of {total_items} items ")
    } else {
        format!(" Previewing {total_items} items ")
    };

    let list = List::new(items).block(
        Block::default()
            .border_set(INTERMEDIATE_BORDER_SET)
            .borders(Borders::ALL)
            .title(title)
            .title_alignment(Alignment::Center),
    );

    frame.render_widget(list, pane);
}

fn add_top_border_to_paragraph(
    paragraph: Paragraph,
    title: String,
) -> Paragraph {
    paragraph.block(
        Block::default()
            .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
            .border_type(BorderType::Thick)
            .title(title)
            .title_alignment(Alignment::Center),
    )
}

fn calculate_string_rows(frame: &Frame, string: &str) -> u16 {
    let width = frame.size().width as usize;

    rows_required_for_string(string, width).try_into().unwrap_or_else(|_| {
        panic!("String requires more than {} (u16::MAX) rows.", u16::MAX)
    })
}
