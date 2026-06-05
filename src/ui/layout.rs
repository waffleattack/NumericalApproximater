//! Shared layout helpers for borders and modal geometry.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

/// Border color for a focusable input field.
///
/// # Arguments
///
/// * `focused` - Whether the field currently has keyboard focus.
///
/// # Returns
///
/// `Color::Yellow` when focused, otherwise `Color::DarkGray`.
pub(super) fn field_border(focused: bool) -> Color {
    if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    }
}

/// Border color for a dropdown trigger or open menu.
///
/// # Arguments
///
/// * `focused` - Whether the dropdown field has keyboard focus.
/// * `open` - Whether the dropdown menu is currently visible.
///
/// # Returns
///
/// `Color::Yellow` when focused or open, otherwise `Color::DarkGray`.
pub(super) fn dropdown_border(focused: bool, open: bool) -> Color {
    if focused || open {
        Color::Yellow
    } else {
        Color::DarkGray
    }
}
/// Styled block for modal dialogs (black background, white text).
///
/// # Arguments
///
/// * `title` - Title text shown in the block border.
///
/// # Returns
///
/// A configured [`Block`] widget.
pub(super) fn modal_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White))
        .title(title)
}

/// Solid black fill for modal interiors (graph remains visible outside).
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Screen region to fill; no-op when width or height is zero.
pub(super) fn fill_black(frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(block, area);
}
/// Compute a rectangle centered within `area`, clamped to its bounds.
///
/// # Arguments
///
/// * `width` - Desired width in terminal cells.
/// * `height` - Desired height in terminal cells.
/// * `area` - Parent region to center within.
///
/// # Returns
///
/// A [`Rect`] no larger than `area`, centered horizontally and vertically.
pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
