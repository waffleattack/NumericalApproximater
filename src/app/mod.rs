//! Application state: inputs, integration, and export flow.

use anyhow::Result;

use crate::expr::OdeFunction;
use crate::format::format_sigfigs_with;
use crate::input::TextInput;
use crate::solver::{integrate, linspace_inclusive};
mod focus;
mod model;

pub use focus::{ExportPromptFocus, Focus};
pub use model::{CurveSeries, Method, MethodChoice, MAX_GRAPH_POINTS, MAX_Y0_FAMILY};

/// Full TUI application state.
pub struct App {
    pub focus: Focus,
    pub equation: TextInput,
    pub method_choice: MethodChoice,
    pub method_menu_open: bool,
    pub method_menu_highlight: usize,
    pub y0_family_enabled: bool,
    pub x0: TextInput,
    pub y0: TextInput,
    pub y0_end: TextInput,
    pub y0_count: TextInput,
    pub x_end: TextInput,
    pub h: TextInput,
    pub export_prompt_open: bool,
    pub export_prompt_focus: ExportPromptFocus,
    pub export_filename: TextInput,
    pub export_h: TextInput,
    pub export_n_points: TextInput,
    pub curves: Vec<CurveSeries>,
    pub error: Option<String>,
    pub status: Option<String>,
}

/// Parse a sidebar or dialog field as `f64`.
///
/// # Arguments
///
/// * `input` - Text field to parse.
/// * `label` - Full error message returned on parse failure.
///
/// # Returns
///
/// The parsed floating-point value.
///
/// # Errors
///
/// Returns an error with `label` as the message when parsing fails.
fn parse_input_f64(input: &TextInput, label: &str) -> Result<f64> {
    input
        .as_str()
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("{label} must be a number"))
}

/// Parse a sidebar or dialog field as `usize`.
///
/// # Arguments
///
/// * `input` - Text field to parse.
/// * `label` - Full error message returned on parse failure.
///
/// # Returns
///
/// The parsed unsigned integer.
///
/// # Errors
///
/// Returns an error with `label` as the message when parsing fails.
fn parse_input_usize(input: &TextInput, label: &str) -> Result<usize> {
    input
        .as_str()
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("{label}"))
}

/// Format an error for display in the footer status bar.
///
/// # Arguments
///
/// * `err` - Error value to stringify.
///
/// # Returns
///
/// The error message, or `"Something went wrong"` if it is empty or whitespace.
pub fn user_message(err: impl std::fmt::Display) -> String {
    let msg = err.to_string();
    if msg.trim().is_empty() {
        "Something went wrong".to_string()
    } else {
        msg
    }
}

impl App {
    /// Create the application with default demo values and an initial plot.
    ///
    /// # Returns
    ///
    /// A fully initialized `App`. Integration errors during startup are ignored.
    pub fn new() -> Self {
        let mut app = Self {
            focus: Focus::Equation,
            equation: TextInput::new("y - y^3"),
            method_choice: MethodChoice::RungeKutta,
            method_menu_open: false,
            method_menu_highlight: 0,
            y0_family_enabled: false,
            x0: TextInput::new("0"),
            y0: TextInput::new("0.3"),
            y0_end: TextInput::new("1.2"),
            y0_count: TextInput::new("7"),
            x_end: TextInput::new("3"),
            h: TextInput::new("0.01"),
            export_prompt_open: false,
            export_prompt_focus: ExportPromptFocus::Filename,
            export_filename: TextInput::new("ode_export"),
            export_h: TextInput::new("0.01"),
            export_n_points: TextInput::new("100"),
            curves: Vec::new(),
            error: None,
            status: None,
        };
        let _ = app.recompute();
        app
    }

    /// Toggle the y₀ family mode on or off.
    pub fn toggle_y0_family(&mut self) {
        self.y0_family_enabled = !self.y0_family_enabled;
    }

    /// Return the text field that currently has keyboard focus.
    ///
    /// # Returns
    ///
    /// `None` when focus is on a non-text control (dropdown, toggle, export button).
    pub fn focused_input_mut(&mut self) -> Option<&mut TextInput> {
        match self.focus {
            Focus::Equation => Some(&mut self.equation),
            Focus::X0 => Some(&mut self.x0),
            Focus::Y0 => Some(&mut self.y0),
            Focus::Y0End => Some(&mut self.y0_end),
            Focus::Y0Count => Some(&mut self.y0_count),
            Focus::XEnd => Some(&mut self.x_end),
            Focus::H => Some(&mut self.h),
            Focus::Y0Family
            | Focus::ExportButton
            | Focus::QuitButton
            | Focus::MethodDropdown => None,
        }
    }

    /// Open the method selection dropdown with the current choice highlighted.
    pub fn open_method_menu(&mut self) {
        self.method_menu_highlight = self.method_choice.index();
        self.method_menu_open = true;
    }

    /// Close the method dropdown, optionally applying the highlighted choice.
    ///
    /// # Arguments
    ///
    /// * `apply` - When `true`, set `method_choice` from the highlight and recompute.
    pub fn close_method_menu(&mut self, apply: bool) {
        if apply && self.method_menu_open {
            self.method_choice = MethodChoice::from_index(self.method_menu_highlight);
            if let Err(e) = self.recompute() {
                self.error = Some(user_message(e));
                self.status = None;
            }
        }
        self.method_menu_open = false;
    }

    /// Move the method menu highlight up, wrapping at the top.
    pub fn method_menu_up(&mut self) {
        if self.method_menu_highlight == 0 {
            self.method_menu_highlight = MethodChoice::OPTIONS.len() - 1;
        } else {
            self.method_menu_highlight -= 1;
        }
    }

    /// Move the method menu highlight down, wrapping at the bottom.
    pub fn method_menu_down(&mut self) {
        self.method_menu_highlight =
            (self.method_menu_highlight + 1) % MethodChoice::OPTIONS.len();
    }

    /// Parse sidebar numeric parameters: x₀, y₀, x_end, and h.
    ///
    /// # Returns
    ///
    /// `(x0, y0, x_end, h)` as `f64` values.
    ///
    /// # Errors
    ///
    /// Returns an error if any field is not a valid number.
    pub fn parse_params(&self) -> Result<(f64, f64, f64, f64)> {
        Ok((
            parse_input_f64(&self.x0, "x₀ must be a number")?,
            parse_input_f64(&self.y0, "y₀ must be a number")?,
            parse_input_f64(&self.x_end, "x_end must be a number")?,
            parse_input_f64(&self.h, "h must be a positive number")?,
        ))
    }

    /// Build the list of initial y values to integrate.
    ///
    /// # Returns
    ///
    /// A single-element vector when y₀ family mode is off, otherwise an evenly
    /// spaced inclusive range between y₀ start and y₀ end.
    ///
    /// # Errors
    ///
    /// Returns an error if parameters are invalid or the count exceeds [`MAX_Y0_FAMILY`].
    pub fn y0_values(&self) -> Result<Vec<f64>> {
        let (_, y0_start, _, _) = self.parse_params()?;
        if !self.y0_family_enabled {
            return Ok(vec![y0_start]);
        }

        let y0_end = parse_input_f64(&self.y0_end, "y₀ end must be a number")?;
        let n = parse_input_usize(&self.y0_count, "y₀ count must be a positive integer")?;
        if n == 0 {
            anyhow::bail!("y₀ count must be at least 1");
        }
        if n > MAX_Y0_FAMILY {
            anyhow::bail!("y₀ count cannot exceed {MAX_Y0_FAMILY}");
        }

        Ok(linspace_inclusive(y0_start, y0_end, n)?)
    }

    /// Parse the equation and parameters, then integrate all selected curves.
    ///
    /// Clears `error` and `status` on entry. On success, replaces `curves`.
    ///
    /// # Returns
    ///
    /// `Ok(())` when integration completes for every method and y₀ value.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing, validation, or integration fails.
    pub fn recompute(&mut self) -> Result<()> {
        self.error = None;
        self.status = None;
        let f = OdeFunction::parse(self.equation.as_str())?;
        let (x0, _, x_end, h) = self.parse_params()?;
        let y0_values = self.y0_values()?;
        let family_total = y0_values.len();

        for &y0 in &y0_values {
            f.validate_at(x0, y0)?;
        }

        let mut curves = Vec::new();
        for &method in self.method_choice.methods() {
            for (family_index, &y0_init) in y0_values.iter().enumerate() {
                let points = integrate(&f, method, x0, y0_init, x_end, h)?;
                let plot_xy = crate::solver::subsample_plot(&points, MAX_GRAPH_POINTS);
                let label = if family_total > 1 {
                    format!(
                        "{}  y₀={}",
                        method.short_label(),
                        format_sigfigs_with(y0_init, 4)
                    )
                } else {
                    method.short_label().to_string()
                };
                curves.push(CurveSeries {
                    method,
                    label,
                    family_index,
                    family_total,
                    points,
                    plot_xy,
                });
            }
        }
        self.curves = curves;
        Ok(())
    }

    /// Open the export dialog with defaults copied from the sidebar.
    pub fn open_export_prompt(&mut self) {
        self.export_filename = TextInput::new("ode_export");
        self.export_h = TextInput::new(self.h.as_str());
        self.export_n_points = TextInput::new("100");
        self.export_prompt_focus = ExportPromptFocus::Filename;
        self.export_prompt_open = true;
        self.error = None;
    }

    /// Return the text field that currently has focus in the export dialog.
    ///
    /// # Returns
    ///
    /// Mutable reference to the active export prompt input.
    pub fn export_prompt_input_mut(&mut self) -> &mut TextInput {
        match self.export_prompt_focus {
            ExportPromptFocus::Filename => &mut self.export_filename,
            ExportPromptFocus::H => &mut self.export_h,
            ExportPromptFocus::NumPoints => &mut self.export_n_points,
        }
    }

    /// Parse export step size and point count from the dialog.
    ///
    /// # Returns
    ///
    /// `(h, n_points)` after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if `h` is not positive or `n_points` is zero.
    pub fn parse_export_options(&self) -> Result<(f64, usize)> {
        let h = parse_input_f64(&self.export_h, "export h must be a positive number")?;
        if h <= 0.0 {
            anyhow::bail!("export h must be positive");
        }
        let n = parse_input_usize(
            &self.export_n_points,
            "number of points must be a positive integer",
        )?;
        if n == 0 {
            anyhow::bail!("number of points must be at least 1");
        }
        Ok((h, n))
    }

    /// Close the export dialog, optionally writing a file.
    ///
    /// # Arguments
    ///
    /// * `save` - When `true`, validate inputs and call [`crate::export::write_text_file`].
    ///
    /// # Returns
    ///
    /// `Ok(())` after closing the dialog.
    ///
    /// # Errors
    ///
    /// Returns an error if `save` is `true` and export validation or I/O fails.
    pub fn close_export_prompt(&mut self, save: bool) -> Result<()> {
        if save && self.export_prompt_open {
            let name = self.export_filename.as_str().trim();
            let (export_h, n_points) = self.parse_export_options()?;
            let f = OdeFunction::parse(self.equation.as_str())?;
            let (x0, _, x_end, _) = self.parse_params()?;
            let y0_values = self.y0_values()?;
            let path = crate::export::write_text_file(
                self.equation.as_str(),
                &f,
                self.method_choice,
                x0,
                &y0_values,
                x_end,
                export_h,
                n_points,
                name,
                self.y0_family_enabled,
            )?;
            self.status = Some(format!("Saved to {}", path.display()));
            self.error = None;
        }
        self.export_prompt_open = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new()
    }

    #[test]
    fn parse_params_valid() {
        let app = test_app();
        let (x0, y0, x_end, h) = app.parse_params().unwrap();
        assert!((x0 - 0.0).abs() < 1e-12);
        assert!((y0 - 0.3).abs() < 1e-12);
        assert!((x_end - 3.0).abs() < 1e-12);
        assert!((h - 0.01).abs() < 1e-12);
    }

    #[test]
    fn parse_params_invalid_x0() {
        let mut app = test_app();
        app.x0 = TextInput::new("not-a-number");
        let err = app.parse_params().unwrap_err().to_string();
        assert!(err.contains("x₀ must be a number"));
    }

    #[test]
    fn parse_params_invalid_h() {
        let mut app = test_app();
        app.h = TextInput::new("abc");
        let err = app.parse_params().unwrap_err().to_string();
        assert!(err.contains("h must be a positive number"));
    }

    #[test]
    fn y0_values_single_when_family_off() {
        let app = test_app();
        let vals = app.y0_values().unwrap();
        assert_eq!(vals.len(), 1);
        assert!((vals[0] - 0.3).abs() < 1e-12);
    }

    #[test]
    fn y0_values_family_on_linspace() {
        let mut app = test_app();
        app.y0_family_enabled = true;
        app.y0 = TextInput::new("0");
        app.y0_end = TextInput::new("2");
        app.y0_count = TextInput::new("3");
        let vals = app.y0_values().unwrap();
        assert_eq!(vals.len(), 3);
        assert!((vals[0] - 0.0).abs() < 1e-12);
        assert!((vals[1] - 1.0).abs() < 1e-12);
        assert!((vals[2] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn y0_values_rejects_zero_count() {
        let mut app = test_app();
        app.y0_family_enabled = true;
        app.y0_count = TextInput::new("0");
        let err = app.y0_values().unwrap_err().to_string();
        assert!(err.contains("y₀ count must be at least 1"));
    }

    #[test]
    fn y0_values_rejects_count_over_max() {
        let mut app = test_app();
        app.y0_family_enabled = true;
        app.y0_count = TextInput::new("100");
        let err = app.y0_values().unwrap_err().to_string();
        assert!(err.contains("y₀ count cannot exceed"));
    }

    #[test]
    fn recompute_success_simple_equation() {
        let mut app = test_app();
        app.equation = TextInput::new("1");
        app.method_choice = MethodChoice::Euler;
        app.x0 = TextInput::new("0");
        app.y0 = TextInput::new("0");
        app.x_end = TextInput::new("1");
        app.h = TextInput::new("0.5");
        app.recompute().unwrap();
        assert!(app.error.is_none());
        assert_eq!(app.curves.len(), 1);
        assert!(!app.curves[0].points.is_empty());
    }

    #[test]
    fn recompute_fails_on_bad_equation() {
        let mut app = test_app();
        app.equation = TextInput::new("sin");
        let err = app.recompute().unwrap_err().to_string();
        assert!(err.contains("function 'sin' must be called with parentheses"));
    }

    #[test]
    fn user_message_empty_uses_fallback() {
        assert_eq!(user_message(""), "Something went wrong");
        assert_eq!(user_message("   "), "Something went wrong");
    }

    #[test]
    fn user_message_non_empty_passthrough() {
        assert_eq!(user_message("bad input"), "bad input");
    }

    #[test]
    fn method_choice_from_index_and_index_roundtrip() {
        for (i, choice) in MethodChoice::OPTIONS.iter().enumerate() {
            assert_eq!(MethodChoice::from_index(i), *choice);
            assert_eq!(choice.index(), i);
        }
        assert_eq!(
            MethodChoice::from_index(99),
            MethodChoice::OPTIONS[99 % MethodChoice::OPTIONS.len()]
        );
    }

    #[test]
    fn method_choice_methods() {
        assert_eq!(MethodChoice::Euler.methods(), &[Method::Euler]);
        assert_eq!(
            MethodChoice::ImprovedEuler.methods(),
            &[Method::ImprovedEuler]
        );
        assert_eq!(MethodChoice::RungeKutta.methods(), &[Method::RungeKutta]);
        assert_eq!(MethodChoice::All.methods(), &Method::ALL);
    }

    #[test]
    fn focus_next_without_y0_family() {
        let mut f = Focus::Equation;
        f = f.next(false);
        assert_eq!(f, Focus::MethodDropdown);
        f = f.next(false);
        assert_eq!(f, Focus::Y0Family);
        f = f.next(false);
        assert_eq!(f, Focus::X0);
        f = f.next(false);
        assert_eq!(f, Focus::Y0);
        f = f.next(false);
        assert_eq!(f, Focus::XEnd);
    }

    #[test]
    fn focus_next_with_y0_family_includes_extra_fields() {
        let mut f = Focus::Y0;
        f = f.next(true);
        assert_eq!(f, Focus::Y0End);
        f = f.next(true);
        assert_eq!(f, Focus::Y0Count);
        f = f.next(true);
        assert_eq!(f, Focus::XEnd);
    }

    #[test]
    fn focus_prev_wraps() {
        assert_eq!(Focus::Equation.prev(false), Focus::QuitButton);
        assert_eq!(Focus::ExportButton.next(false), Focus::QuitButton);
        assert_eq!(Focus::QuitButton.next(false), Focus::Equation);
    }

    #[test]
    fn parse_export_options_valid() {
        let app = test_app();
        let (h, n) = app.parse_export_options().unwrap();
        assert!((h - 0.01).abs() < 1e-12);
        assert_eq!(n, 100);
    }

    #[test]
    fn parse_export_options_rejects_non_positive_h() {
        let mut app = test_app();
        app.export_h = TextInput::new("0");
        let err = app.parse_export_options().unwrap_err().to_string();
        assert!(err.contains("export h must be positive"));
    }

    #[test]
    fn parse_export_options_rejects_zero_points() {
        let mut app = test_app();
        app.export_n_points = TextInput::new("0");
        let err = app.parse_export_options().unwrap_err().to_string();
        assert!(err.contains("number of points must be at least 1"));
    }
}
