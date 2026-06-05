//! Fixed-step ODE integrators and point-sampling helpers.

use anyhow::{bail, Result};

use crate::app::Method;
use crate::expr::OdeFunction;

/// A single `(x, y)` sample along a solution curve.
#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Integrate y' = F(x,y) from `x0` to `x_end` with fixed step size `h`.
///
/// The last step may be shorter so the final point lands exactly on `x_end`.
///
/// # Arguments
///
/// * `f` - Parsed right-hand side F(x, y).
/// * `method` - Numerical method to use.
/// * `x0` - Initial x value.
/// * `y0` - Initial y value.
/// * `x_end` - Target x value (must be greater than `x0`).
/// * `h` - Fixed step size (must be positive).
///
/// # Returns
///
/// Sampled points starting at `(x0, y0)` and ending at `x_end`.
///
/// # Errors
///
/// Returns an error if `h <= 0`, `x_end <= x0`, or evaluation fails during stepping.
pub fn integrate(
    f: &OdeFunction,
    method: Method,
    x0: f64,
    y0: f64,
    x_end: f64,
    h: f64,
) -> Result<Vec<Point>> {
    if h <= 0.0 {
        bail!("step size h must be positive");
    }
    if x_end <= x0 {
        bail!("x_end must be greater than x0");
    }

    let est_steps = ((x_end - x0) / h).ceil() as usize + 1;
    let mut points = Vec::with_capacity(est_steps);
    let mut x = x0;
    let mut y = y0;
    points.push(Point { x, y });

    const EPS: f64 = 1e-12;
    while x < x_end - EPS {
        let step = h.min(x_end - x);
        y = advance(f, method, x, y, step)?;
        x += step;
        points.push(Point { x, y });
    }

    Ok(points)
}

/// Advance one step with the chosen integration method.
///
/// # Arguments
///
/// * `f` - ODE right-hand side.
/// * `method` - Integrator to apply.
/// * `x` - Current x coordinate.
/// * `y` - Current y value.
/// * `h` - Step size for this advance.
///
/// # Returns
///
/// The y value at `x + h`.
///
/// # Errors
///
/// Returns an error if `f.eval` fails during the step.
fn advance(f: &OdeFunction, method: Method, x: f64, y: f64, h: f64) -> Result<f64> {
    match method {
        Method::Euler => euler(f, x, y, h),
        Method::ImprovedEuler => improved_euler(f, x, y, h),
        Method::RungeKutta => runge_kutta4(f, x, y, h),
    }
}

/// Explicit Euler step: y_{n+1} = y_n + h * f(x_n, y_n).
///
/// # Arguments
///
/// * `f` - ODE right-hand side.
/// * `x` - Current x coordinate.
/// * `y` - Current y value.
/// * `h` - Step size.
///
/// # Returns
///
/// The y value after one Euler step.
///
/// # Errors
///
/// Returns an error if `f.eval` fails.
fn euler(f: &OdeFunction, x: f64, y: f64, h: f64) -> Result<f64> {
    Ok(y + h * f.eval(x, y)?)
}

/// Heun / improved Euler step (predictor-corrector average).
///
/// # Arguments
///
/// * `f` - ODE right-hand side.
/// * `x` - Current x coordinate.
/// * `y` - Current y value.
/// * `h` - Step size.
///
/// # Returns
///
/// The y value after one improved Euler step.
///
/// # Errors
///
/// Returns an error if `f.eval` fails.
fn improved_euler(f: &OdeFunction, x: f64, y: f64, h: f64) -> Result<f64> {
    let k1 = f.eval(x, y)?;
    let y_pred = y + h * k1;
    let k2 = f.eval(x + h, y_pred)?;
    Ok(y + h * (k1 + k2) / 2.0)
}

/// Classical fourth-order Runge-Kutta step (RK4).
///
/// # Arguments
///
/// * `f` - ODE right-hand side.
/// * `x` - Current x coordinate.
/// * `y` - Current y value.
/// * `h` - Step size.
///
/// # Returns
///
/// The y value after one RK4 step.
///
/// # Errors
///
/// Returns an error if `f.eval` fails.
fn runge_kutta4(f: &OdeFunction, x: f64, y: f64, h: f64) -> Result<f64> {
    let k1 = f.eval(x, y)?;
    let k2 = f.eval(x + h / 2.0, y + h * k1 / 2.0)?;
    let k3 = f.eval(x + h / 2.0, y + h * k2 / 2.0)?;
    let k4 = f.eval(x + h, y + h * k3)?;
    Ok(y + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0)
}

/// Map `sample_count` evenly spaced indices into `0..=item_count - 1`.
///
/// # Arguments
///
/// * `item_count` - Length of the source sequence (must be > 0).
/// * `sample_count` - Number of indices to produce (must be > 0).
///
/// # Returns
///
/// Rounded indices spanning first to last element, inclusive.
fn evenly_spaced_indices(item_count: usize, sample_count: usize) -> Vec<usize> {
    debug_assert!(item_count > 0);
    debug_assert!(sample_count > 0);
    if sample_count == 1 {
        return vec![0];
    }
    let last = item_count - 1;
    (0..sample_count)
        .map(|i| {
            let t = i as f64 / (sample_count - 1) as f64;
            (t * last as f64).round() as usize
        })
        .collect()
}

/// Pick `n` points evenly spaced along an integrated curve.
///
/// # Arguments
///
/// * `points` - Full integration output (must be non-empty).
/// * `n` - Desired number of output points (must be at least 1).
///
/// # Returns
///
/// A subsequence of `points` with length `min(n, points.len())`.
///
/// # Errors
///
/// Returns an error if `n == 0` or `points` is empty.
pub fn subsample(points: &[Point], n: usize) -> Result<Vec<Point>> {
    if n == 0 {
        bail!("number of points must be at least 1");
    }
    if points.is_empty() {
        bail!("no points to sample");
    }
    if n == 1 {
        return Ok(vec![points[0].clone()]);
    }
    if n >= points.len() {
        return Ok(points.to_vec());
    }

    let mut out = Vec::with_capacity(n);
    for idx in evenly_spaced_indices(points.len(), n) {
        out.push(points[idx].clone());
    }
    Ok(out)
}

/// Produce `n` evenly spaced values from `start` through `end` (inclusive).
///
/// # Arguments
///
/// * `start` - First value in the sequence.
/// * `end` - Last value in the sequence.
/// * `n` - Number of values to produce (must be at least 1).
///
/// # Returns
///
/// `n` evenly spaced floats. When `n == 1`, the result is `[start]`.
///
/// # Errors
///
/// Returns an error if `n == 0`.
pub fn linspace_inclusive(start: f64, end: f64, n: usize) -> Result<Vec<f64>> {
    if n == 0 {
        bail!("need at least one y₀ value");
    }
    if n == 1 {
        return Ok(vec![start]);
    }
    let step = (end - start) / (n - 1) as f64;
    Ok((0..n).map(|i| start + step * i as f64).collect())
}

/// Cap points used for plotting so very small `h` stays responsive.
///
/// # Arguments
///
/// * `points` - Full integration output.
/// * `max` - Maximum number of `(x, y)` pairs to return.
///
/// # Returns
///
/// All points when `points.len() <= max`, otherwise `max` evenly spaced samples.
pub fn subsample_plot(points: &[Point], max: usize) -> Vec<(f64, f64)> {
    if points.len() <= max {
        return points.iter().map(|p| (p.x, p.y)).collect();
    }
    let mut out = Vec::with_capacity(max);
    for idx in evenly_spaced_indices(points.len(), max) {
        let p = &points[idx];
        out.push((p.x, p.y));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::OdeFunction;

    fn unit_slope() -> OdeFunction {
        OdeFunction::parse("1").unwrap()
    }

    fn max_y_error(pts: &[Point]) -> f64 {
        pts.iter()
            .map(|p| (p.y - p.x).abs())
            .fold(0.0_f64, f64::max)
    }

    #[test]
    fn fixed_h_reaches_x_end() {
        let f = unit_slope();
        let pts = integrate(&f, Method::Euler, 0.0, 0.0, 1.0, 0.25).unwrap();
        assert!((pts.last().unwrap().x - 1.0).abs() < 1e-9);
        assert_eq!(pts.len(), 5); // 0, 0.25, 0.5, 0.75, 1.0
    }

    #[test]
    fn euler_y_prime_one_approximates_y_equals_x() {
        let f = unit_slope();
        let pts = integrate(&f, Method::Euler, 0.0, 0.0, 1.0, 0.1).unwrap();
        assert!((pts.last().unwrap().x - 1.0).abs() < 1e-9);
        assert!(max_y_error(&pts) < 0.1);
    }

    #[test]
    fn improved_euler_y_prime_one_approximates_y_equals_x() {
        let f = unit_slope();
        let pts = integrate(&f, Method::ImprovedEuler, 0.0, 0.0, 1.0, 0.1).unwrap();
        assert!((pts.last().unwrap().x - 1.0).abs() < 1e-9);
        assert!(max_y_error(&pts) < 0.01);
    }

    #[test]
    fn rk4_y_prime_one_approximates_y_equals_x() {
        let f = unit_slope();
        let pts = integrate(&f, Method::RungeKutta, 0.0, 0.0, 1.0, 0.1).unwrap();
        assert!((pts.last().unwrap().x - 1.0).abs() < 1e-9);
        assert!(max_y_error(&pts) < 1e-6);
    }

    #[test]
    fn rk4_more_accurate_than_euler_for_same_h() {
        let f = OdeFunction::parse("y").unwrap();
        let h = 0.25;
        let analytic = |x: f64| x.exp();
        let euler_pts = integrate(&f, Method::Euler, 0.0, 1.0, 1.0, h).unwrap();
        let rk4_pts = integrate(&f, Method::RungeKutta, 0.0, 1.0, 1.0, h).unwrap();
        let euler_err = euler_pts
            .iter()
            .map(|p| (p.y - analytic(p.x)).abs())
            .fold(0.0_f64, f64::max);
        let rk4_err = rk4_pts
            .iter()
            .map(|p| (p.y - analytic(p.x)).abs())
            .fold(0.0_f64, f64::max);
        assert!(rk4_err < euler_err);
    }

    #[test]
    fn integrate_rejects_non_positive_h() {
        let f = unit_slope();
        let err = integrate(&f, Method::Euler, 0.0, 0.0, 1.0, 0.0)
            .unwrap_err()
            .to_string();
        assert!(err.contains("step size h must be positive"));
        let err = integrate(&f, Method::Euler, 0.0, 0.0, 1.0, -0.1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("step size h must be positive"));
    }

    #[test]
    fn integrate_rejects_x_end_not_greater_than_x0() {
        let f = unit_slope();
        let err = integrate(&f, Method::Euler, 1.0, 0.0, 1.0, 0.1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("x_end must be greater than x0"));
        let err = integrate(&f, Method::Euler, 2.0, 0.0, 1.0, 0.1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("x_end must be greater than x0"));
    }

    #[test]
    fn non_uniform_last_step_when_x_end_not_divisible_by_h() {
        let f = unit_slope();
        let pts = integrate(&f, Method::Euler, 0.0, 0.0, 1.0, 0.3).unwrap();
        assert_eq!(pts.len(), 5);
        assert!((pts[0].x - 0.0).abs() < 1e-12);
        assert!((pts[1].x - 0.3).abs() < 1e-12);
        assert!((pts[2].x - 0.6).abs() < 1e-12);
        assert!((pts[3].x - 0.9).abs() < 1e-12);
        assert!((pts[4].x - 1.0).abs() < 1e-12);
        assert!((pts[4].x - pts[3].x - 0.1).abs() < 1e-12);
    }

    #[test]
    fn linspace_inclusive_single_point() {
        let v = linspace_inclusive(2.0, 5.0, 1).unwrap();
        assert_eq!(v, vec![2.0]);
    }

    #[test]
    fn linspace_inclusive_endpoints() {
        let v = linspace_inclusive(0.0, 1.0, 3).unwrap();
        assert_eq!(v.len(), 3);
        assert!((v[0] - 0.0).abs() < 1e-12);
        assert!((v[1] - 0.5).abs() < 1e-12);
        assert!((v[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn linspace_inclusive_rejects_zero_count() {
        let err = linspace_inclusive(0.0, 1.0, 0)
            .unwrap_err()
            .to_string();
        assert!(err.contains("need at least one y₀ value"));
    }

    #[test]
    fn subsample_rejects_zero_n() {
        let pts = vec![Point { x: 0.0, y: 0.0 }];
        let err = subsample(&pts, 0).unwrap_err().to_string();
        assert!(err.contains("number of points must be at least 1"));
    }

    #[test]
    fn subsample_rejects_empty_points() {
        let err = subsample(&[], 1).unwrap_err().to_string();
        assert!(err.contains("no points to sample"));
    }

    #[test]
    fn subsample_n_one_returns_first() {
        let pts = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
        ];
        let out = subsample(&pts, 1).unwrap();
        assert_eq!(out.len(), 1);
        assert!((out[0].x - 0.0).abs() < 1e-12);
    }

    #[test]
    fn subsample_n_ge_len_returns_all() {
        let pts = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 2.0, y: 2.0 },
        ];
        let out = subsample(&pts, 5).unwrap();
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn subsample_intermediate_n_evenly_spaced_indices() {
        let pts: Vec<Point> = (0..5)
            .map(|i| {
                let x = i as f64;
                Point { x, y: x }
            })
            .collect();
        let out = subsample(&pts, 3).unwrap();
        assert_eq!(out.len(), 3);
        assert!((out[0].x - 0.0).abs() < 1e-12);
        assert!((out[1].x - 2.0).abs() < 1e-12);
        assert!((out[2].x - 4.0).abs() < 1e-12);
    }

    #[test]
    fn subsample_plot_returns_all_when_len_le_max() {
        let pts = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
        ];
        let out = subsample_plot(&pts, 10);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], (0.0, 0.0));
        assert_eq!(out[1], (1.0, 1.0));
    }

    #[test]
    fn subsample_plot_caps_when_len_gt_max() {
        let pts: Vec<Point> = (0..100)
            .map(|i| {
                let x = i as f64;
                Point { x, y: x }
            })
            .collect();
        let out = subsample_plot(&pts, 10);
        assert_eq!(out.len(), 10);
        assert_eq!(out.first().unwrap(), &(0.0, 0.0));
        assert_eq!(out.last().unwrap(), &(99.0, 99.0));
    }
}
