//! Solution curve charts (single panel or split by method).

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

use crate::app::{App, CurveSeries, Method, MethodChoice};
use crate::format::{format_sigfigs_with, y_axis_display, CHART_SIGFIGS};
use crate::solver::Point;

const COLOR_EULER: Color = Color::Green;
const COLOR_IMPROVED: Color = Color::Yellow;
const COLOR_RK: Color = Color::Cyan;
/// Draw the right-hand chart panel or placeholder messages.
///
/// # Arguments
///
/// * `frame` - Ratatui frame to draw into.
/// * `area` - Layout region for the chart.
/// * `app` - Application state (`curves`, `method_choice`).
pub(super) fn draw_chart_panel(frame: &mut Frame, area: Rect, app: &App) {
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
