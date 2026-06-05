//! Ratatui rendering for the equation bar, sidebar, chart, modals, and footer.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, GraphType, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

use crate::app::{App, CurveSeries, ExportPromptFocus, Focus, Method, MethodChoice};
use crate::format::{format_sigfigs_with, y_axis_display, CHART_SIGFIGS};
use crate::input::TextInput;
use crate::solver::Point;

const EQUATION_PREFIX: &str = "y' = ";
const COLOR_EULER: Color = Color::Green;
const COLOR_IMPROVED: Color = Color::Yellow;
const COLOR_RK: Color = Color::Cyan;

/// Border color for a focusable input field.
///
/// # Arguments
///
/// * `focused` - Whether the field currently has keyboard focus.
///
/// # Returns
///
/// `Color::Yellow` when focused, otherwise `Color::DarkGray`.
fn field_border(focused: bool) -> Color {
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
fn dropdown_border(focused: bool, open: bool) -> Color {
    if focused || open {
        Color::Yellow
    } else {
        Color::DarkGray
    }
}

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
        draw_method_dropdown(frame, app);
    }

    if app.export_prompt_open {
        draw_export_prompt(frame, app);
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
fn modal_block(title: &str) -> Block<'_> {
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
fn fill_black(frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(block, area);
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
fn render_text_input(
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

    draw_sidebar(frame, cols[0], app);
    draw_chart_panel(frame, cols[1], app);
}

/// Draw the left sidebar with method, ICs, step size, legend, and export.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Sidebar layout region.
/// * `app` - Application state.
fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
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

/// Label for sidebar action buttons: leading space, then a bright first letter (shortcut key).
///
/// Terminals cannot change per-character font size; the bright palette reads slightly larger.
fn shortcut_button_label(label: &str, base: Style, accent: Color) -> Line<'_> {
    let mut chars = label.chars();
    let first = chars.next().unwrap_or(' ');
    let rest: String = chars.collect();
    Line::from(vec![
        Span::styled(" ", base),
        Span::styled(first.to_string(), base.fg(accent)),
        Span::styled(rest, base),
    ])
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
fn draw_export_prompt(frame: &mut Frame, app: &App) {
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

/// Draw a labeled text field inside the export dialog.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for this field.
/// * `label` - Field title shown in the border.
/// * `input` - Editable text buffer.
/// * `focused` - Whether this field is the active export prompt field.
fn draw_prompt_field(
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
fn draw_dropdown_trigger(
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

/// Draw the method selection list overlay.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `app` - Application state (`method_menu_highlight`, `method_menu_open`).
fn draw_method_dropdown(frame: &mut Frame, app: &App) {
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
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
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
fn draw_sidebar_field(
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

/// Draw the right-hand chart panel or placeholder messages.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the chart.
/// * `app` - Application state (`curves`, `method_choice`).
fn draw_chart_panel(frame: &mut Frame, area: Rect, app: &App) {
    if app.curves.is_empty() {
        let msg = Paragraph::new("Enter a valid F(x,y) and press Enter to plot.")
            .block(Block::default().borders(Borders::ALL).title(" Graph "));
        frame.render_widget(msg, area);
        return;
    }

    if app.curves.iter().all(|c| c.points.is_empty()) {
        let msg = Paragraph::new("No data.").block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, area);
        return;
    }

    if app.method_choice == MethodChoice::All {
        draw_split_by_method(frame, area, &app.curves);
    } else {
        let curves: Vec<&CurveSeries> = app.curves.iter().collect();
        render_curves_chart(frame, area, &curves, "y vs x");
    }
}

/// Draw one stacked chart per integration method (All mode).
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region split vertically by method.
/// * `curves` - Curve series to partition by [`Method`].
fn draw_split_by_method(frame: &mut Frame, area: Rect, curves: &[CurveSeries]) {
    let n = Method::ALL.len() as u32;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..n).map(|_| Constraint::Ratio(1, n)).collect::<Vec<_>>())
        .split(area);

    for (method, row) in Method::ALL.iter().zip(rows.iter()) {
        let method_curves: Vec<&CurveSeries> =
            curves.iter().filter(|c| c.method == *method).collect();
        if method_curves.is_empty() {
            continue;
        }
        render_curves_chart(frame, *row, &method_curves, method.short_label());
    }
}

/// Primary chart color for a single integration method.
///
/// # Arguments
///
/// * `method` - Euler, improved Euler, or Runge–Kutta.
///
/// # Returns
///
/// The method's default line color.
fn method_color(method: Method) -> Color {
    match method {
        Method::Euler => COLOR_EULER,
        Method::ImprovedEuler => COLOR_IMPROVED,
        Method::RungeKutta => COLOR_RK,
    }
}

/// Render a Braille line chart for one or more curve series.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the chart.
/// * `curves` - Series to plot (shared axes from all points).
/// * `title` - Chart title in the border.
fn render_curves_chart(frame: &mut Frame, area: Rect, curves: &[&CurveSeries], title: &str) {
    let all_points: Vec<&Point> = curves.iter().flat_map(|c| c.points.iter()).collect();
    let (x_min, x_max, y_min, y_max) = bounds_multi(&all_points);
    let x_pad = ((x_max - x_min) * 0.05).max(0.1);
    let y_pad = ((y_max - y_min) * 0.1).max(0.1);
    let x_bounds = [x_min - x_pad, x_max + x_pad];
    let y_bounds_padded = [y_min - y_pad, y_max + y_pad];
    let (y_bounds, y_label_digits) = y_axis_display(y_bounds_padded, y_min, y_max);

    let border_color = curves
        .first()
        .map(|c| method_color(c.method))
        .unwrap_or(Color::White);

    let mut datasets = Vec::new();
    for curve in curves {
        datasets.push(
            Dataset::default()
                .name(curve.label.as_str())
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(family_color(
                    curve.method,
                    curve.family_index,
                    curve.family_total,
                )))
                .data(&curve.plot_xy),
        );
    }

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} "))
                .border_style(Style::default().fg(border_color)),
        )
        .x_axis(
            Axis::default()
                .title("x")
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds)
                .labels(tick_labels_x(x_bounds)),
        )
        .y_axis(
            Axis::default()
                .title("y")
                .style(Style::default().fg(Color::Gray))
                .bounds(y_bounds)
                .labels(tick_labels(y_bounds, y_label_digits)),
        );

    frame.render_widget(chart, area);
}

/// Line color for a curve in a y₀ family (palette by method and index).
///
/// # Arguments
///
/// * `method` - Integration method for the curve.
/// * `index` - Zero-based index within the family.
/// * `total` - Total curves in the family; `1` uses the method's primary color.
///
/// # Returns
///
/// A distinct color from the method's palette.
fn family_color(method: Method, index: usize, total: usize) -> Color {
    if total <= 1 {
        return method_color(method);
    }
    let palette: &[Color] = match method {
        Method::Euler => &[
            Color::Green,
            Color::LightGreen,
            Color::Cyan,
            Color::Blue,
            Color::Magenta,
        ],
        Method::ImprovedEuler => &[
            Color::Yellow,
            Color::LightYellow,
            Color::Rgb(255, 200, 0),
            Color::Rgb(200, 160, 0),
            Color::Rgb(255, 220, 100),
        ],
        Method::RungeKutta => &[
            Color::Cyan,
            Color::LightCyan,
            Color::Blue,
            Color::Rgb(100, 200, 255),
            Color::Rgb(0, 180, 220),
        ],
    };
    palette[index % palette.len()]
}

/// Compute axis-aligned min/max bounds over multiple points.
///
/// # Arguments
///
/// * `points` - Non-empty slice of point references.
///
/// # Returns
///
/// `(x_min, x_max, y_min, y_max)`.
fn bounds_multi(points: &[&Point]) -> (f64, f64, f64, f64) {
    let first = points[0];
    let mut x_min = first.x;
    let mut x_max = first.x;
    let mut y_min = first.y;
    let mut y_max = first.y;
    for p in points.iter().skip(1) {
        x_min = x_min.min(p.x);
        x_max = x_max.max(p.x);
        y_min = y_min.min(p.y);
        y_max = y_max.max(p.y);
    }
    (x_min, x_max, y_min, y_max)
}

/// Build three axis tick labels (min, mid, max) with fixed significant figures.
///
/// # Arguments
///
/// * `bounds` - Axis range `[min, max]`.
/// * `digits` - Significant figures for each label.
///
/// # Returns
///
/// Three styled [`Span`] labels for the chart axis.
fn tick_labels(bounds: [f64; 2], digits: usize) -> Vec<Span<'static>> {
    let [a, b] = bounds;
    [a, (a + b) / 2.0, b]
        .into_iter()
        .map(|v| Span::raw(format_sigfigs_with(v, digits)))
        .collect()
}

/// Build x-axis tick labels using the default chart precision.
///
/// # Arguments
///
/// * `bounds` - x-axis range `[min, max]`.
///
/// # Returns
///
/// Three tick labels at [`CHART_SIGFIGS`] precision.
fn tick_labels_x(bounds: [f64; 2]) -> Vec<Span<'static>> {
    tick_labels(bounds, CHART_SIGFIGS)
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
