//! Slope field segments for y' = F(x, y).

use crate::expr::OdeFunction;

/// Grid resolution for the slope field (columns × rows sample points).
pub const SLOPE_FIELD_COLS: usize = 22;
pub const SLOPE_FIELD_ROWS: usize = 14;

/// One short line segment indicating slope direction at a grid point.
pub type SlopeSegment = [(f64, f64); 2];

/// Axis-aligned view window used by the chart and slope field sampler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBounds {
    pub x: [f64; 2],
    pub y: [f64; 2],
}

/// Build slope-field segments over a rectangular window.
///
/// At each grid point `(x, y)` the segment follows the direction `(1, F(x, y))`,
/// normalized to a fixed length in data coordinates. Points where `F` is
/// non-finite or evaluation fails are skipped.
///
/// # Arguments
///
/// * `f` - Compiled right-hand side.
/// * `bounds` - Plot window.
/// * `cols` - Grid columns (≥ 2).
/// * `rows` - Grid rows (≥ 2).
///
/// # Returns
///
/// Pairs of endpoints, one segment per successful grid sample.
pub fn build_slope_field(
    f: &OdeFunction,
    bounds: ViewBounds,
    cols: usize,
    rows: usize,
) -> Vec<SlopeSegment> {
    if cols < 2 || rows < 2 {
        return Vec::new();
    }

    let [x_min, x_max] = bounds.x;
    let [y_min, y_max] = bounds.y;
    let dx = (x_max - x_min) / (cols - 1) as f64;
    let dy = (y_max - y_min) / (rows - 1) as f64;
    if dx <= 0.0 || dy <= 0.0 {
        return Vec::new();
    }

    let half = 0.35 * dx.min(dy);
    let mut segments = Vec::with_capacity(cols * rows);

    for j in 0..rows {
        for i in 0..cols {
            let x = x_min + i as f64 * dx;
            let y = y_min + j as f64 * dy;
            let slope = match f.eval(x, y) {
                Ok(v) if v.is_finite() => v,
                _ => continue,
            };
            let len = (1.0 + slope * slope).sqrt();
            let ux = 1.0 / len;
            let uy = slope / len;
            segments.push([
                (x - ux * half, y - uy * half),
                (x + ux * half, y + uy * half),
            ]);
        }
    }

    segments
}

/// Compute chart bounds from integrated curves, or fall back to the parameter box.
///
/// # Arguments
///
/// * `x0` - Integration start.
/// * `x_end` - Integration end.
/// * `y0_values` - Initial y values (single or family).
/// * `curve_points` - Optional `(x, y)` samples from solution curves.
pub fn view_bounds(
    x0: f64,
    x_end: f64,
    y0_values: &[f64],
    curve_points: &[(f64, f64)],
) -> ViewBounds {
    if !curve_points.is_empty() {
        let mut x_min = curve_points[0].0;
        let mut x_max = curve_points[0].0;
        let mut y_min = curve_points[0].1;
        let mut y_max = curve_points[0].1;
        for &(x, y) in curve_points.iter().skip(1) {
            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);
        }
        let x_pad = ((x_max - x_min) * 0.05).max(0.1);
        let y_pad = ((y_max - y_min) * 0.1).max(0.1);
        return ViewBounds {
            x: [x_min - x_pad, x_max + x_pad],
            y: [y_min - y_pad, y_max + y_pad],
        };
    }

    let x_lo = x0.min(x_end);
    let x_hi = x0.max(x_end);
    let x_span = (x_hi - x_lo).max(0.5);
    let y_min = y0_values.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max = y0_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_mid = (y_min + y_max) / 2.0;
    let y_span = (y_max - y_min).max(1.0).max(x_span * 0.6);
    let x_pad = x_span * 0.05;
    let y_pad = y_span * 0.1;
    ViewBounds {
        x: [x_lo - x_pad, x_hi + x_pad],
        y: [y_mid - y_span / 2.0 - y_pad, y_mid + y_span / 2.0 + y_pad],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::OdeFunction;

    #[test]
    fn unit_slope_segments_are_horizontal_steps_in_x() {
        let f = OdeFunction::parse("0").unwrap();
        let bounds = ViewBounds {
            x: [0.0, 2.0],
            y: [0.0, 2.0],
        };
        let segs = build_slope_field(&f, bounds, 3, 3);
        assert!(!segs.is_empty());
        for seg in segs {
            assert!((seg[0].1 - seg[1].1).abs() < 1e-9, "zero slope should be flat");
            assert!(seg[1].0 > seg[0].0, "segment should point in +x");
        }
    }

    #[test]
    fn view_bounds_from_curve_points() {
        let bounds = view_bounds(0.0, 3.0, &[0.3], &[(0.0, 0.3), (3.0, 1.2)]);
        assert!(bounds.x[0] < 0.0);
        assert!(bounds.x[1] > 3.0);
        assert!(bounds.y[0] < 0.3);
        assert!(bounds.y[1] > 1.2);
    }

    #[test]
    fn view_bounds_without_curves_uses_parameter_box() {
        let bounds = view_bounds(0.0, 3.0, &[0.3], &[]);
        assert!(bounds.x[0] <= 0.0);
        assert!(bounds.x[1] >= 3.0);
        assert!(bounds.y[0] < 0.3);
        assert!(bounds.y[1] > 0.3);
    }
}
