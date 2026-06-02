use anyhow::{bail, Result};

use crate::app::Method;
use crate::expr::OdeFunction;

#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Integrate y' = F(x,y) from `x0` to `x_end` with fixed step size `h`.
/// The last step may be shorter so the final point lands on `x_end`.
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

    let mut points = Vec::new();
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

fn advance(f: &OdeFunction, method: Method, x: f64, y: f64, h: f64) -> Result<f64> {
    match method {
        Method::Euler => euler(f, x, y, h),
        Method::ImprovedEuler => improved_euler(f, x, y, h),
        Method::RungeKutta => runge_kutta4(f, x, y, h),
    }
}

/// y_{n+1} = y_n + h * f(x_n, y_n)
fn euler(f: &OdeFunction, x: f64, y: f64, h: f64) -> Result<f64> {
    Ok(y + h * f.eval(x, y)?)
}

/// Heun's method (predictor-corrector average)
fn improved_euler(f: &OdeFunction, x: f64, y: f64, h: f64) -> Result<f64> {
    let k1 = f.eval(x, y)?;
    let y_pred = y + h * k1;
    let k2 = f.eval(x + h, y_pred)?;
    Ok(y + h * (k1 + k2) / 2.0)
}

/// Classical fourth-order Runge-Kutta
fn runge_kutta4(f: &OdeFunction, x: f64, y: f64, h: f64) -> Result<f64> {
    let k1 = f.eval(x, y)?;
    let k2 = f.eval(x + h / 2.0, y + h * k1 / 2.0)?;
    let k3 = f.eval(x + h / 2.0, y + h * k2 / 2.0)?;
    let k4 = f.eval(x + h, y + h * k3)?;
    Ok(y + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0)
}

/// Pick `n` points evenly spaced along an integrated curve.
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

    let last = points.len() - 1;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let idx = (t * last as f64).round() as usize;
        out.push(points[idx].clone());
    }
    Ok(out)
}

/// `n` evenly spaced values from `start` through `end` (inclusive).
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
pub fn subsample_plot(points: &[Point], max: usize) -> Vec<(f64, f64)> {
    if points.len() <= max {
        return points.iter().map(|p| (p.x, p.y)).collect();
    }
    let last = points.len() - 1;
    (0..max)
        .map(|i| {
            let t = i as f64 / (max - 1) as f64;
            let idx = (t * last as f64).round() as usize;
            let p = &points[idx];
            (p.x, p.y)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::OdeFunction;

    #[test]
    fn fixed_h_reaches_x_end() {
        let f = OdeFunction::parse("1").unwrap();
        let pts = integrate(&f, Method::Euler, 0.0, 0.0, 1.0, 0.25).unwrap();
        assert!((pts.last().unwrap().x - 1.0).abs() < 1e-9);
        assert_eq!(pts.len(), 5); // 0, 0.25, 0.5, 0.75, 1.0
    }
}
