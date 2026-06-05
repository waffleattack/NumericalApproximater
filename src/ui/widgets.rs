//! Reusable field and text-input widgets.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::input::TextInput;

use super::layout::{dropdown_border, field_border, fill_black};

/// Label for sidebar action buttons: leading space, then a bright first letter (shortcut key).
///
/// Terminals cannot change per-character font size; the bright palette reads slightly larger.
pub(super) fn shortcut_button_label(label: &str, base: Style, accent: Color) -> Line<'_> {
    let mut chars = label.chars();
    let first = chars.next().unwrap_or(' ');
    let rest: String = chars.collect();
    Line::from(vec![
        Span::styled(" ", base),
        Span::styled(first.to_string(), base.fg(accent)),
        Span::styled(rest, base),
    ])
}
/// Render editable text with a highlighted cursor; sets terminal cursor when `focused`.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Inner region for the text.
/// * `input` - Text buffer and cursor position.
/// * `focused` - Whether this field is active (cursor highlight and terminal cursor).
/// * `prefix` - Optional static prefix (e.g. `y' = `) shown before the value.
/// * `suffix` - Optional hint text shown after the value in dark gray.
/// * `on_black` - Use white-on-black styling when not focused (modal fields).
pub(super) fn render_text_input(
    frame: &mut Frame,
    area: Rect,
    input: &TextInput,
    focused: bool,
    prefix: Option<&str>,
    suffix: Option<&str>,
    on_black: bool,
) {
    let mut spans: Vec<Span> = Vec::new();
    let prefix_len = if let Some(p) = prefix {
        spans.push(Span::raw(p));
        p.chars().count()
    } else {
        0
    };

    let chars: Vec<char> = input.value.chars().collect();
    let cursor = input.cursor.min(chars.len());

    for (i, &ch) in chars.iter().enumerate() {
        let style = if focused && i == cursor {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if focused {
            Style::default().add_modifier(Modifier::BOLD)
        } else if on_black {
            Style::default().fg(Color::White).bg(Color::Black)
        } else {
            Style::default()
        };
        spans.push(Span::styled(ch.to_string(), style));
    }

    if focused && cursor == chars.len() {
        spans.push(Span::styled(
            " ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));
    }

    if let Some(suf) = suffix {
        let suf_style = if on_black {
            Style::default().fg(Color::DarkGray).bg(Color::Black)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(suf, suf_style));
    }

    if focused {
        let cursor_x = area.x + (prefix_len + cursor) as u16;
        frame.set_cursor_position((cursor_x, area.y));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
/// Draw a closed dropdown showing the current value and arrow.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the trigger.
/// * `title` - Field label in the border.
/// * `value` - Currently selected option text.
/// * `focused` - Whether the dropdown has keyboard focus.
/// * `open` - Whether the menu is expanded (shows ▲ instead of ▼).
pub(super) fn draw_dropdown_trigger(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    focused: bool,
    open: bool,
) {
    let arrow = if open { "▲" } else { "▼" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dropdown_border(focused, open)))
        .title(format!(" {title} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text = format!("{value} {arrow}");
    let style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new(text).style(style), inner);
}
/// Draw a labeled numeric/text sidebar input field.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the field.
/// * `label` - Field title in the border.
/// * `input` - Editable text buffer.
/// * `focused` - Whether this field has keyboard focus.
pub(super) fn draw_sidebar_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    input: &TextInput,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border(focused)))
        .title(format!(" {label} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_text_input(frame, inner, input, focused, None, None, false);
}
/// Draw a labeled text field inside the export dialog.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for this field.
/// * `label` - Field title shown in the border.
/// * `input` - Editable text buffer.
/// * `focused` - Whether this field is the active export prompt field.
pub(super) fn draw_prompt_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    input: &TextInput,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border(focused)))
        .style(Style::default().bg(Color::Black))
        .title(format!(" {label} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_black(frame, inner);
    render_text_input(frame, inner, input, focused, None, None, true);
}
