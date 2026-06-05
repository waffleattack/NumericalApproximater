//! Ratatui rendering for the equation bar, sidebar, chart, modals, and footer.
//!
//! Start with [`draw`]: top equation bar, sidebar + chart body, footer, then overlays.

mod chart;
mod layout;
mod sidebar;
mod widgets;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus};

use widgets::render_text_input;
use layout::field_border;

const EQUATION_PREFIX: &str = "y' = ";

/// Render the full application layout into the terminal frame.
///
/// # Arguments
///
/// * `frame` - Ratatui frame for the current draw pass.
/// * `app` - Application state to display.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    draw_equation_bar(frame, root[0], app);
    draw_body(frame, root[1], app);
    draw_footer(frame, root[2], app);

    if app.method_menu_open {
        sidebar::draw_method_dropdown(frame, app);
    }

    if app.export_prompt_open {
        sidebar::draw_export_prompt(frame, app);
    }
}
/// Draw the top equation input bar (`y' = …`).
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the bar.
/// * `app` - Application state (equation text and focus).
fn draw_equation_bar(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Equation;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border(focused)))
        .title(" F(x, y) ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_text_input(
        frame,
        inner,
        &app.equation,
        focused,
        Some(EQUATION_PREFIX),
        None,
        false,
    );
}
/// Draw the main body: sidebar controls and chart panel.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region below the equation bar.
/// * `app` - Application state.
fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(area);

    sidebar::draw_sidebar(frame, cols[0], app);
    chart::draw_chart_panel(frame, cols[1], app);
}
/// Draw the bottom status bar (errors, success, or default key hints).
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the footer.
/// * `app` - Application state (`error`, `status`).
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let (msg, color, title) = if let Some(err) = &app.error {
        (
            err.clone(),
            Color::Red,
            Some(" Error ".to_string()),
        )
    } else if let Some(status) = &app.status {
        (
            status.clone(),
            Color::Green,
            Some(" OK ".to_string()),
        )
    } else {
        (
            String::new(),
            Color::DarkGray,
            None,
        )
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));
    if let Some(t) = title {
        block = block.title(t).title_style(Style::default().fg(color).add_modifier(Modifier::BOLD));
    }

    let footer = Paragraph::new(msg)
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: false })
        .block(block);
    frame.render_widget(footer, area);
}
