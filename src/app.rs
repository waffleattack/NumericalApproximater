use anyhow::Result;

use crate::expr::OdeFunction;
use crate::format::format_sigfigs_with;
use crate::input::TextInput;
use crate::solver::{integrate, linspace_inclusive, Point};

/// Maximum points drawn per curve (subsample if h is very small).
pub const MAX_GRAPH_POINTS: usize = 800;

/// Cap on simultaneous y₀ initial conditions (per method).
pub const MAX_Y0_FAMILY: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Euler,
    ImprovedEuler,
    RungeKutta,
}

impl Method {
    pub const ALL: [Method; 3] = [Method::Euler, Method::ImprovedEuler, Method::RungeKutta];

    pub fn short_label(self) -> &'static str {
        match self {
            Method::Euler => "Euler",
            Method::ImprovedEuler => "Improved Euler",
            Method::RungeKutta => "Runge-Kutta",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodChoice {
    Euler,
    ImprovedEuler,
    RungeKutta,
    All,
}

impl MethodChoice {
    pub const OPTIONS: [MethodChoice; 4] = [
        MethodChoice::Euler,
        MethodChoice::ImprovedEuler,
        MethodChoice::RungeKutta,
        MethodChoice::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MethodChoice::Euler => "Euler",
            MethodChoice::ImprovedEuler => "Improved Euler",
            MethodChoice::RungeKutta => "Runge-Kutta (RK4)",
            MethodChoice::All => "All three methods",
        }
    }

    pub fn index(self) -> usize {
        Self::OPTIONS.iter().position(|&m| m == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Self {
        Self::OPTIONS[i % Self::OPTIONS.len()]
    }

    pub fn methods(self) -> &'static [Method] {
        match self {
            MethodChoice::Euler => &[Method::Euler],
            MethodChoice::ImprovedEuler => &[Method::ImprovedEuler],
            MethodChoice::RungeKutta => &[Method::RungeKutta],
            MethodChoice::All => &Method::ALL,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurveSeries {
    pub method: Method,
    pub label: String,
    pub family_index: usize,
    pub family_total: usize,
    pub points: Vec<Point>,
    pub plot_xy: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Equation,
    MethodDropdown,
    Y0Family,
    X0,
    Y0,
    Y0End,
    Y0Count,
    XEnd,
    H,
    ExportButton,
}

impl Focus {
    pub fn next(self, y0_family: bool) -> Self {
        step(self, y0_family, true)
    }

    pub fn prev(self, y0_family: bool) -> Self {
        step(self, y0_family, false)
    }

    pub fn is_text_input(self) -> bool {
        !matches!(
            self,
            Focus::MethodDropdown | Focus::ExportButton | Focus::Y0Family
        )
    }
}

fn step(from: Focus, y0_family: bool, forward: bool) -> Focus {
    let order: &[Focus] = if y0_family {
        &[
            Focus::Equation,
            Focus::MethodDropdown,
            Focus::Y0Family,
            Focus::X0,
            Focus::Y0,
            Focus::Y0End,
            Focus::Y0Count,
            Focus::XEnd,
            Focus::H,
            Focus::ExportButton,
        ]
    } else {
        &[
            Focus::Equation,
            Focus::MethodDropdown,
            Focus::Y0Family,
            Focus::X0,
            Focus::Y0,
            Focus::XEnd,
            Focus::H,
            Focus::ExportButton,
        ]
    };
    let i = order.iter().position(|&f| f == from).unwrap_or(0);
    let n = order.len();
    let next_i = if forward {
        (i + 1) % n
    } else {
        (i + n - 1) % n
    };
    order[next_i]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPromptFocus {
    Filename,
    H,
    NumPoints,
}

impl ExportPromptFocus {
    pub const ORDER: [ExportPromptFocus; 3] = [
        ExportPromptFocus::Filename,
        ExportPromptFocus::H,
        ExportPromptFocus::NumPoints,
    ];

    pub fn next(self) -> Self {
        let i = Self::ORDER.iter().position(|&f| f == self).unwrap_or(0);
        Self::ORDER[(i + 1) % Self::ORDER.len()]
    }

    pub fn prev(self) -> Self {
        let i = Self::ORDER.iter().position(|&f| f == self).unwrap_or(0);
        Self::ORDER[(i + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }
}

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

/// Format an error for display in the footer status bar.
pub fn user_message(err: impl std::fmt::Display) -> String {
    let msg = err.to_string();
    if msg.trim().is_empty() {
        "Something went wrong".to_string()
    } else {
        msg
    }
}

impl App {
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

    pub fn toggle_y0_family(&mut self) {
        self.y0_family_enabled = !self.y0_family_enabled;
    }

    pub fn focused_input_mut(&mut self) -> Option<&mut TextInput> {
        match self.focus {
            Focus::Equation => Some(&mut self.equation),
            Focus::X0 => Some(&mut self.x0),
            Focus::Y0 => Some(&mut self.y0),
            Focus::Y0End => Some(&mut self.y0_end),
            Focus::Y0Count => Some(&mut self.y0_count),
            Focus::XEnd => Some(&mut self.x_end),
            Focus::H => Some(&mut self.h),
            Focus::Y0Family | Focus::ExportButton | Focus::MethodDropdown => None,
        }
    }

    pub fn open_method_menu(&mut self) {
        self.method_menu_highlight = self.method_choice.index();
        self.method_menu_open = true;
    }

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

    pub fn method_menu_up(&mut self) {
        if self.method_menu_highlight == 0 {
            self.method_menu_highlight = MethodChoice::OPTIONS.len() - 1;
        } else {
            self.method_menu_highlight -= 1;
        }
    }

    pub fn method_menu_down(&mut self) {
        self.method_menu_highlight =
            (self.method_menu_highlight + 1) % MethodChoice::OPTIONS.len();
    }

    pub fn parse_params(&self) -> Result<(f64, f64, f64, f64)> {
        let x0: f64 = self
            .x0
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("x₀ must be a number"))?;
        let y0: f64 = self
            .y0
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("y₀ must be a number"))?;
        let x_end: f64 = self
            .x_end
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("x_end must be a number"))?;
        let h: f64 = self
            .h
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("h must be a positive number"))?;
        Ok((x0, y0, x_end, h))
    }

    pub fn y0_values(&self) -> Result<Vec<f64>> {
        let (_, y0_start, _, _) = self.parse_params()?;
        if !self.y0_family_enabled {
            return Ok(vec![y0_start]);
        }

        let y0_end: f64 = self
            .y0_end
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("y₀ end must be a number"))?;
        let n: usize = self
            .y0_count
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("y₀ count must be a positive integer"))?;
        if n == 0 {
            anyhow::bail!("y₀ count must be at least 1");
        }
        if n > MAX_Y0_FAMILY {
            anyhow::bail!("y₀ count cannot exceed {MAX_Y0_FAMILY}");
        }

        Ok(linspace_inclusive(y0_start, y0_end, n)?)
    }

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

    pub fn open_export_prompt(&mut self) {
        self.export_filename = TextInput::new("ode_export");
        self.export_h = TextInput::new(self.h.as_str());
        self.export_n_points = TextInput::new("100");
        self.export_prompt_focus = ExportPromptFocus::Filename;
        self.export_prompt_open = true;
        self.error = None;
    }

    pub fn export_prompt_input_mut(&mut self) -> &mut TextInput {
        match self.export_prompt_focus {
            ExportPromptFocus::Filename => &mut self.export_filename,
            ExportPromptFocus::H => &mut self.export_h,
            ExportPromptFocus::NumPoints => &mut self.export_n_points,
        }
    }

    pub fn parse_export_options(&self) -> Result<(f64, usize)> {
        let h: f64 = self
            .export_h
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("export h must be a positive number"))?;
        if h <= 0.0 {
            anyhow::bail!("export h must be positive");
        }
        let n: usize = self
            .export_n_points
            .as_str()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("number of points must be a positive integer"))?;
        if n == 0 {
            anyhow::bail!("number of points must be at least 1");
        }
        Ok((h, n))
    }

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
        assert_eq!(Focus::Equation.prev(false), Focus::ExportButton);
        assert_eq!(Focus::ExportButton.next(false), Focus::Equation);
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
