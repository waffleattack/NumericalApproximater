//! Write integrated solution curves to text files under `exported/`.

use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::app::MethodChoice;
use crate::expr::OdeFunction;
use crate::format::{format_sigfigs, format_sigfigs_with, SIGFIGS};
use crate::solver::{integrate, subsample, Point};

const EXPORT_DIR: &str = "exported";

/// Integrate and write solution data to `exported/{filename}.txt`.
///
/// # Arguments
///
/// * `equation` - Raw equation string shown in the file header.
/// * `f` - Parsed ODE right-hand side.
/// * `choice` - Which numerical method(s) to export.
/// * `x0` - Initial x value.
/// * `y0_values` - Initial y values (one curve per value).
/// * `x_end` - Integration endpoint.
/// * `h` - Export step size (must be positive).
/// * `n_points` - Number of points to subsample per curve.
/// * `filename` - Base filename without path (sanitized before use).
/// * `y0_family` - Whether the header should show a y₀ range.
///
/// # Returns
///
/// Absolute path to the created `.txt` file.
///
/// # Errors
///
/// Returns an error if the filename is empty after sanitization, `h <= 0`,
/// directory creation fails, or integration/subsampling fails.
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
    create_dir_all(&export_dir)?;
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

/// Format one exported line: `label: (x, y), (x, y), ...`.
///
/// # Arguments
///
/// * `label` - Method name (and optional y₀ annotation).
/// * `points` - Subsampled points to include.
///
/// # Returns
///
/// A single line of comma-separated coordinate pairs.
fn format_method_line(label: &str, points: &[Point]) -> String {
    let pairs = points
        .iter()
        .map(|p| format!("({}, {})", format_sigfigs(p.x), format_sigfigs(p.y)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{label}: {pairs}")
}

/// Reduce a user-provided name to safe ASCII for use as a filename stem.
///
/// # Arguments
///
/// * `raw` - User input, optionally ending in `.txt`.
///
/// # Returns
///
/// Alphanumeric characters, underscores, and hyphens only. May be empty.
pub(crate) fn sanitize_filename(raw: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MethodChoice;
    use std::io::Read;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    struct TempWorkDir {
        path: PathBuf,
        previous: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TempWorkDir {
        fn new() -> Self {
            let lock = CWD_LOCK.lock().unwrap();
            let n = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "numerical_approximater_test_{}_{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).unwrap();
            let previous = std::env::current_dir().unwrap();
            std::env::set_current_dir(&path).unwrap();
            Self {
                path,
                previous,
                _lock: lock,
            }
        }
    }

    impl Drop for TempWorkDir {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.previous);
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn sanitize_filename_strips_invalid_chars() {
        assert_eq!(sanitize_filename("  my file!.txt  "), "my_file");
        assert_eq!(sanitize_filename("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_filename("___hello___"), "hello");
    }

    #[test]
    fn sanitize_filename_keeps_allowed_chars() {
        assert_eq!(sanitize_filename("ode_export-1"), "ode_export-1");
    }

    #[test]
    fn sanitize_filename_empty_after_sanitize() {
        assert_eq!(sanitize_filename("!!!"), "");
        assert_eq!(sanitize_filename("   "), "");
    }

    #[test]
    fn write_text_file_creates_export_with_content() {
        let _dir = TempWorkDir::new();
        let f = OdeFunction::parse("1").unwrap();
        let path = write_text_file(
            "1",
            &f,
            MethodChoice::Euler,
            0.0,
            &[0.0],
            1.0,
            0.5,
            3,
            "test_export",
            false,
        )
        .unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().contains("exported"));
        assert!(path.to_string_lossy().ends_with("test_export.txt"));

        let mut contents = String::new();
        File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(contents.contains("y' = 1"));
        assert!(contents.contains("x0 = 0"));
        assert!(contents.contains("Euler:"));
    }

    #[test]
    fn write_text_file_rejects_empty_filename() {
        let _dir = TempWorkDir::new();
        let f = OdeFunction::parse("1").unwrap();
        let err = write_text_file(
            "1",
            &f,
            MethodChoice::Euler,
            0.0,
            &[0.0],
            1.0,
            0.5,
            3,
            "!!!",
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("filename cannot be empty"));
    }

    #[test]
    fn write_text_file_family_header() {
        let _dir = TempWorkDir::new();
        let f = OdeFunction::parse("x").unwrap();
        let path = write_text_file(
            "x",
            &f,
            MethodChoice::All,
            0.0,
            &[0.0, 1.0],
            1.0,
            0.5,
            2,
            "family_export",
            true,
        )
        .unwrap();

        let mut contents = String::new();
        File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(contents.contains("y0 = 0 .. 1"));
        assert!(contents.contains("Euler"));
        assert!(contents.contains("Improved Euler"));
        assert!(contents.contains("Runge-Kutta"));
    }
}
