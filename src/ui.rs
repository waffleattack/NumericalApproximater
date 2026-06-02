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

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
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

fn modal_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White))
        .title(title)
}

/// Solid black fill for modal interiors (graph remains visible outside).
fn fill_black(frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(block, area);
}

fn draw_equation_bar(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Equation;
    let border = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(" F(x, y) ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_text_input(
        frame,
        inner,
        &app.equation,
        focused,
        Some(EQUATION_PREFIX),
        if focused {
            Some("  [Enter: update]")
        } else {
            None
        },
        false,
    );
}

/// Renders editable text with a highlighted cursor; sets terminal cursor when `focused`.
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

fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(area);

    draw_sidebar(frame, cols[0], app);
    draw_chart_panel(frame, cols[1], app);
}

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
}

fn draw_y0_family_toggle(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Y0Family;
    let border = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
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
        .border_style(Style::default().fg(border))
        .title(" y₀ family ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let label = if on {
        " ON — Space: off "
    } else {
        " OFF — Space: on "
    };
    frame.render_widget(Paragraph::new(label).style(style), inner);
}

fn draw_export_button(frame: &mut Frame, area: Rect, focused: bool) {
    let border = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(" Export ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let label = if focused {
        " Enter — save to exported/ "
    } else {
        " Save text file "
    };
    frame.render_widget(Paragraph::new(label).style(style), inner);
}

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

fn draw_prompt_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    input: &TextInput,
    focused: bool,
) {
    let border = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(Color::Black))
        .title(format!(" {label} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    fill_black(frame, inner);
    render_text_input(frame, inner, input, focused, None, None, true);
}

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

fn draw_dropdown_trigger(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    focused: bool,
    open: bool,
) {
    let border = if focused || open {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let arrow = if open { "▲" } else { "▼" };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
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

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn draw_sidebar_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    input: &TextInput,
    focused: bool,
) {
    let border = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(format!(" {label} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_text_input(frame, inner, input, focused, None, None, false);
}

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

fn method_color(method: Method) -> Color {
    match method {
        Method::Euler => COLOR_EULER,
        Method::ImprovedEuler => COLOR_IMPROVED,
        Method::RungeKutta => COLOR_RK,
    }
}

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

fn tick_labels(bounds: [f64; 2], digits: usize) -> Vec<Span<'static>> {
    let [a, b] = bounds;
    [a, (a + b) / 2.0, b]
        .into_iter()
        .map(|v| Span::raw(format_sigfigs_with(v, digits)))
        .collect()
}

fn tick_labels_x(bounds: [f64; 2]) -> Vec<Span<'static>> {
    tick_labels(bounds, CHART_SIGFIGS)
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let msg = if let Some(err) = &app.error {
        err.clone()
    } else if let Some(status) = &app.status {
        status.clone()
    } else {
        "y₀ family: Space toggle | Enter: update | Export: save | q: quit"
            .into()
    };
    let color = if app.error.is_some() {
        Color::Red
    } else if app.status.is_some() {
        Color::Green
    } else {
        Color::DarkGray
    };
    let footer = Paragraph::new(msg)
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}
