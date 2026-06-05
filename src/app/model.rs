//! Integration methods, curve metadata, and shared limits.

use crate::solver::Point;

/// Maximum points drawn per curve (subsample if h is very small).
pub const MAX_GRAPH_POINTS: usize = 800;

/// Cap on simultaneous y₀ initial conditions (per method).
pub const MAX_Y0_FAMILY: usize = 15;

/// A single numerical integration method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Euler,
    ImprovedEuler,
    RungeKutta,
}

impl Method {
    pub const ALL: [Method; 3] = [Method::Euler, Method::ImprovedEuler, Method::RungeKutta];

    /// Short human-readable name for charts and exports.
    ///
    /// # Returns
    ///
    /// A static label string for this method.
    pub fn short_label(self) -> &'static str {
        match self {
            Method::Euler => "Euler",
            Method::ImprovedEuler => "Improved Euler",
            Method::RungeKutta => "Runge-Kutta",
        }
    }
}

/// User-facing method selection, including an "all methods" option.
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

    /// Label shown in the method dropdown.
    ///
    /// # Returns
    ///
    /// A static menu label for this choice.
    pub fn label(self) -> &'static str {
        match self {
            MethodChoice::Euler => "Euler",
            MethodChoice::ImprovedEuler => "Improved Euler",
            MethodChoice::RungeKutta => "Runge-Kutta (RK4)",
            MethodChoice::All => "All three methods",
        }
    }

    /// Index of this choice in [`Self::OPTIONS`].
    ///
    /// # Returns
    ///
    /// Position in the dropdown list, or `0` if not found.
    pub fn index(self) -> usize {
        Self::OPTIONS.iter().position(|&m| m == self).unwrap_or(0)
    }

    /// Map a dropdown index back to a [`MethodChoice`].
    ///
    /// # Arguments
    ///
    /// * `i` - Menu index (wraps modulo the number of options).
    ///
    /// # Returns
    ///
    /// The corresponding choice.
    pub fn from_index(i: usize) -> Self {
        Self::OPTIONS[i % Self::OPTIONS.len()]
    }

    /// Expand this choice into one or more [`Method`] values to integrate.
    ///
    /// # Returns
    ///
    /// A slice of methods to run during `recompute`.
    pub fn methods(self) -> &'static [Method] {
        match self {
            MethodChoice::Euler => &[Method::Euler],
            MethodChoice::ImprovedEuler => &[Method::ImprovedEuler],
            MethodChoice::RungeKutta => &[Method::RungeKutta],
            MethodChoice::All => &Method::ALL,
        }
    }
}

/// One plotted/exported solution curve with display metadata.
#[derive(Debug, Clone)]
pub struct CurveSeries {
    pub method: Method,
    pub label: String,
    pub family_index: usize,
    pub family_total: usize,
    pub points: Vec<Point>,
    pub plot_xy: Vec<(f64, f64)>,
}
