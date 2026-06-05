use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::app::MethodChoice;
use crate::expr::OdeFunction;
use crate::format::{format_sigfigs, format_sigfigs_with, SIGFIGS};
use crate::solver::{integrate, subsample, Point};

const EXPORT_DIR: &str = "exported";

pub fn write_text_file(
    equation: &str,
    f: &OdeFunction,
    choice: MethodChoice,
    x0: f64,
    y0_values: &[f64],
    x_end: f64,
    h: f64,
    n_points: usize,
    filename: &str,
    y0_family: bool,
) -> Result<PathBuf> {
    let name = sanitize_filename(filename);
    if name.is_empty() {
        bail!("filename cannot be empty");
    }
    if h <= 0.0 {
        bail!("export step size h must be positive");
    }
    let mut export_dir = env::current_dir()?;
    export_dir.push(EXPORT_DIR);
    create_dir_all(export_dir.clone())?;
    let path = export_dir.join(format!("{name}.txt"));
    let mut file = File::create(&path)?;

    writeln!(file, "y' = {equation}")?;
    if y0_family && y0_values.len() > 1 {
        writeln!(
            file,
            "x0 = {}, y0 = {} .. {}, x_end = {}, h = {}, points = {}",
            format_sigfigs(x0),
            format_sigfigs(y0_values[0]),
            format_sigfigs(y0_values[y0_values.len() - 1]),
            format_sigfigs(x_end),
            format_sigfigs(h),
            n_points
        )?;
    } else {
        writeln!(
            file,
            "x0 = {}, y0 = {}, x_end = {}, h = {}, points = {}",
            format_sigfigs(x0),
            format_sigfigs(y0_values[0]),
            format_sigfigs(x_end),
            format_sigfigs(h),
            n_points
        )?;
    }
    writeln!(file, "# coordinates shown to {SIGFIGS} significant figures")?;
    writeln!(file)?;

    for &method in choice.methods() {
        for &y0 in y0_values {
            let full = integrate(f, method, x0, y0, x_end, h)?;
            let points = subsample(&full, n_points)?;
            let label = if y0_values.len() > 1 {
                format!(
                    "{} y₀={}",
                    method.short_label(),
                    format_sigfigs_with(y0, 4)
                )
            } else {
                method.short_label().to_string()
            };
            writeln!(file, "{}", format_method_line(&label, &points))?;
        }
    }

    Ok(path)
}

fn format_method_line(label: &str, points: &[Point]) -> String {
    let pairs = points
        .iter()
        .map(|p| format!("({}, {})", format_sigfigs(p.x), format_sigfigs(p.y)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{label}: {pairs}")
}

fn sanitize_filename(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_ext = trimmed.strip_suffix(".txt").unwrap_or(trimmed);
    let sanitized: String = without_ext
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    sanitized.trim_matches('_').to_string()
}
