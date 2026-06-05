//! Sidebar controls, export dialog, and method picker.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, ExportPromptFocus, Focus, MethodChoice};

use super::layout::{centered_rect, field_border, fill_black, modal_block};
use super::widgets::{
    draw_dropdown_trigger, draw_prompt_field, draw_sidebar_field, shortcut_button_label,
};

pub(super) fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let mut constraints = vec![
        Constraint::Length(3), // method
        Constraint::Length(3), // y0 family
        Constraint::Length(3), // x0
        Constraint::Length(3), // y0
    ];
    if app.y0_family_enabled {
        constraints.push(Constraint::Length(3)); // y0 end
        constraints.push(Constraint::Length(3)); // y0 count
    }
    constraints.extend([
        Constraint::Length(3), // x_end
        Constraint::Length(3), // h
        Constraint::Length(1), // legend
        Constraint::Length(3), // export
        Constraint::Length(3), // quit
        Constraint::Min(0),
    ]);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut i = 0usize;
    draw_dropdown_trigger(
        frame,
        rows[i],
        "Method",
        app.method_choice.label(),
        app.focus == Focus::MethodDropdown,
        app.method_menu_open,
    );
    i += 1;
    draw_y0_family_toggle(frame, rows[i], app);
    i += 1;
    draw_sidebar_field(frame, rows[i], "x₀", &app.x0, app.focus == Focus::X0);
    i += 1;
    let y0_label = if app.y0_family_enabled { "y₀ start" } else { "y₀" };
    draw_sidebar_field(frame, rows[i], y0_label, &app.y0, app.focus == Focus::Y0);
    i += 1;
    if app.y0_family_enabled {
        draw_sidebar_field(frame, rows[i], "y₀ end", &app.y0_end, app.focus == Focus::Y0End);
        i += 1;
        draw_sidebar_field(frame, rows[i], "y₀ #", &app.y0_count, app.focus == Focus::Y0Count);
        i += 1;
    }
    draw_sidebar_field(frame, rows[i], "x_end", &app.x_end, app.focus == Focus::XEnd);
    i += 1;
    draw_sidebar_field(frame, rows[i], "h", &app.h, app.focus == Focus::H);
    i += 1;

    let legend = legend_text(app);
    frame.render_widget(
        Paragraph::new(legend)
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: true }),
        rows[i],
    );
    i += 1;
    draw_export_button(frame, rows[i], app.focus == Focus::ExportButton);
    i += 1;
    draw_quit_button(frame, rows[i], app.focus == Focus::QuitButton);
}
/// Draw the y₀ family on/off toggle control.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the toggle.
/// * `app` - Application state (`y0_family_enabled` and focus).
fn draw_y0_family_toggle(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Y0Family;
    let on = app.y0_family_enabled;
    let style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(if on { Color::Cyan } else { Color::DarkGray })
            .add_modifier(Modifier::BOLD)
    } else if on {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border(focused)))
        .title(" y₀ family ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let label = if on { " ON " } else { " OFF " };
    frame.render_widget(Paragraph::new(label).style(style), inner);
}

/// Draw the export action button in the sidebar.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the button.
/// * `focused` - Whether the export button has keyboard focus.
fn draw_export_button(frame: &mut Frame, area: Rect, focused: bool) {
    let (base, accent) = if focused {
        (
            Style::default().fg(Color::Black).bg(Color::Green),
            Color::White,
        )
    } else {
        (Style::default().fg(Color::Green), Color::LightGreen)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border(focused)))
        .title(" Export ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(shortcut_button_label("Save", base, accent)),
        inner,
    );
}

/// Draw the quit action button in the sidebar.
fn draw_quit_button(frame: &mut Frame, area: Rect, focused: bool) {
    let (base, accent) = if focused {
        (
            Style::default().fg(Color::Black).bg(Color::Red),
            Color::White,
        )
    } else {
        (Style::default().fg(Color::Red), Color::LightRed)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(field_border(focused)))
        .title(" Quit ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(shortcut_button_label("Quit", base, accent)),
        inner,
    );
}
/// Draw the centered export dialog (filename, step size, point count).
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `app` - Application state (export fields and focus).
pub(super) fn draw_export_prompt(frame: &mut Frame, app: &App) {
    let area = centered_rect(52, 15, frame.area());
    // Clear only the dialog region so the graph stays visible around it.
    frame.render_widget(Clear, area);

    let block = modal_block(" Export ").border_style(Style::default().fg(Color::Green));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_black(frame, inner);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(1),
        ])
        .split(inner);

    draw_prompt_field(
        frame,
        rows[0],
        "filename",
        &app.export_filename,
        app.export_prompt_focus == ExportPromptFocus::Filename,
    );
    draw_prompt_field(
        frame,
        rows[1],
        "export h",
        &app.export_h,
        app.export_prompt_focus == ExportPromptFocus::H,
    );
    draw_prompt_field(
        frame,
        rows[2],
        "points",
        &app.export_n_points,
        app.export_prompt_focus == ExportPromptFocus::NumPoints,
    );

    frame.render_widget(
        Paragraph::new("Tab/↑/↓: fields | Enter: save | Esc: cancel")
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        rows[3],
    );
}
/// Build sidebar legend text describing visible curves.
///
/// # Arguments
///
/// * `app` - Application state (`curves`, `y0_family_enabled`, `method_choice`).
///
/// # Returns
///
/// A short legend string, or empty when no legend is needed.
fn legend_text(app: &App) -> String {
    if app.y0_family_enabled {
        return format!("{} y₀ curves", app.curves.len());
    }
    if app.method_choice == MethodChoice::All || app.curves.len() <= 1 {
        return String::new();
    }
    app.curves
        .iter()
        .map(|c| format!("● {}", c.label))
        .collect::<Vec<_>>()
        .join("  ")
}
/// Draw the method selection list overlay.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `app` - Application state (`method_menu_highlight`, `method_menu_open`).
pub(super) fn draw_method_dropdown(frame: &mut Frame, app: &App) {
    let area = centered_rect(36, 8, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = MethodChoice::OPTIONS
        .iter()
        .enumerate()
        .map(|(i, &choice)| {
            let prefix = if i == app.method_menu_highlight {
                "› "
            } else {
                "  "
            };
            let style = if i == app.method_menu_highlight {
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::Black)
            };
            ListItem::new(format!("{prefix}{}", choice.label())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        modal_block(" Select method ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(list, area);
}
