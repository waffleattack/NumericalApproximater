//! Solution curves and slope field charts.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::Span,
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph,
    },
    Frame,
};

use crate::app::{App, CurveSeries, GraphDisplay, Method, MethodChoice};
use crate::format::{format_sigfigs_with, y_axis_display, CHART_SIGFIGS};
use crate::slope_field::{SlopeSegment, ViewBounds};
use crate::solver::Point;

const COLOR_EULER: Color = Color::Green;
const COLOR_IMPROVED: Color = Color::Yellow;
const COLOR_RK: Color = Color::Cyan;
const COLOR_SLOPE: Color = Color::DarkGray;

/// Draw the right-hand chart panel.
pub(super) fn draw_chart_panel(frame: &mut Frame, area: Rect, app: &App) {
    let show_solution = app.graph_display.shows_solution();
    let show_slope = app.graph_display.shows_slope_field();

    if show_slope && app.slope_field.is_empty() && (!show_solution || app.curves.is_empty()) {
        let msg = Paragraph::new("Enter a valid F(x,y) and press Enter.")
            .block(Block::default().borders(Borders::ALL).title(" Graph "));
        frame.render_widget(msg, area);
        return;
    }

    if show_solution && app.curves.is_empty() {
        let msg = Paragraph::new("Enter a valid F(x,y) and press Enter to plot.")
            .block(Block::default().borders(Borders::ALL).title(" Graph "));
        frame.render_widget(msg, area);
        return;
    }

    if show_solution
        && app.method_choice == MethodChoice::All
        && app.curves.iter().any(|c| !c.points.is_empty())
    {
        draw_split_by_method(frame, area, app, show_slope);
        return;
    }

    let curves: Vec<&CurveSeries> = app.curves.iter().collect();
    let title = chart_title(app.graph_display);
    let border = border_color_for_curves(&curves);
    render_chart(
        frame,
        area,
        app.view_bounds,
        &curves,
        if show_slope {
            &app.slope_field
        } else {
            &[]
        },
        title,
        border,
    );
}

fn chart_title(mode: GraphDisplay) -> &'static str {
    match mode {
        GraphDisplay::Solution => "y vs x",
        GraphDisplay::SlopeField => "slope field",
        GraphDisplay::Both => "y vs x + slope",
    }
}

/// Draw one stacked chart per integration method (All mode).
fn draw_split_by_method(frame: &mut Frame, area: Rect, app: &App, show_slope: bool) {
    if show_slope {
        render_slope_layer(frame, area, app.view_bounds, &app.slope_field);
    }

    let n = Method::ALL.len() as u32;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints((0..n).map(|_| Constraint::Ratio(1, n)).collect::<Vec<_>>())
        .split(area);

    for (method, row) in Method::ALL.iter().zip(rows.iter()) {
        let method_curves: Vec<&CurveSeries> =
            app.curves.iter().filter(|c| c.method == *method).collect();
        if method_curves.is_empty() {
            continue;
        }
        let bounds = bounds_for_curves(&method_curves, app.view_bounds);
        render_chart(
            frame,
            *row,
            bounds,
            &method_curves,
            &[],
            method.short_label(),
            method_color(*method),
        );
    }
}

fn border_color_for_curves(curves: &[&CurveSeries]) -> Color {
    curves
        .first()
        .map(|c| method_color(c.method))
        .unwrap_or(Color::White)
}

fn bounds_for_curves(curves: &[&CurveSeries], fallback: ViewBounds) -> ViewBounds {
    let all_points: Vec<&Point> = curves.iter().flat_map(|c| c.points.iter()).collect();
    if all_points.is_empty() {
        return fallback;
    }
    let (x_min, x_max, y_min, y_max) = bounds_multi(&all_points);
    let x_pad = ((x_max - x_min) * 0.05).max(0.1);
    let y_pad = ((y_max - y_min) * 0.1).max(0.1);
    ViewBounds {
        x: [x_min - x_pad, x_max + x_pad],
        y: [y_min - y_pad, y_max + y_pad],
    }
}

/// Draw slope segments in a single canvas pass (avoids one chart dataset per segment).
fn render_slope_layer<'a>(
    frame: &mut Frame,
    area: Rect,
    bounds: ViewBounds,
    segments: &'a [SlopeSegment],
) {
    if segments.is_empty() {
        return;
    }

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds(bounds.x)
        .y_bounds(bounds.y)
        .paint(move |ctx| {
            for seg in segments {
                ctx.draw(&CanvasLine::new(
                    seg[0].0,
                    seg[0].1,
                    seg[1].0,
                    seg[1].1,
                    COLOR_SLOPE,
                ));
            }
        });

    frame.render_widget(canvas, area);
}

/// Render optional slope underlay and solution curves with axes.
fn render_chart(
    frame: &mut Frame,
    area: Rect,
    bounds: ViewBounds,
    curves: &[&CurveSeries],
    slope_segments: &[SlopeSegment],
    title: &str,
    border_color: Color,
) {
    let x_bounds = bounds.x;
    let y_bounds_padded = bounds.y;
    let y_min = bounds.y[0];
    let y_max = bounds.y[1];
    let (y_bounds, y_label_digits) = y_axis_display(y_bounds_padded, y_min, y_max);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(border_color));

    if !slope_segments.is_empty() {
        render_slope_layer(frame, block.inner(area), bounds, slope_segments);
    }

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
        .block(block)
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

fn method_color(method: Method) -> Color {
    match method {
        Method::Euler => COLOR_EULER,
        Method::ImprovedEuler => COLOR_IMPROVED,
        Method::RungeKutta => COLOR_RK,
    }
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
