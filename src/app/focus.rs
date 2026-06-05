//! Keyboard focus and tab order in the main UI and export dialog.

/// Tab-order configuration for the main sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusNav {
    pub y0_family: bool,
    pub slope_bounds: bool,
}

/// Keyboard focus target in the main UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Equation,
    MethodDropdown,
    GraphDisplay,
    Y0Family,
    X0,
    Y0,
    Y0End,
    Y0Count,
    XEnd,
    H,
    ViewXMin,
    ViewXMax,
    ViewYMin,
    ViewYMax,
    ExportButton,
    QuitButton,
}

impl Focus {
    /// Move focus to the next field in tab order.
    pub fn next(self, nav: FocusNav) -> Self {
        step(self, nav, true)
    }

    /// Move focus to the previous field in tab order.
    pub fn prev(self, nav: FocusNav) -> Self {
        step(self, nav, false)
    }

    /// `true` for equation and numeric inputs; `false` for buttons and toggles.
    pub fn is_text_input(self) -> bool {
        !matches!(
            self,
            Focus::MethodDropdown
                | Focus::GraphDisplay
                | Focus::ExportButton
                | Focus::QuitButton
                | Focus::Y0Family
        )
    }
}

fn focus_order(nav: FocusNav) -> Vec<Focus> {
    let mut order = vec![
        Focus::Equation,
        Focus::MethodDropdown,
        Focus::GraphDisplay,
        Focus::Y0Family,
        Focus::X0,
        Focus::Y0,
    ];
    if nav.y0_family {
        order.extend([Focus::Y0End, Focus::Y0Count]);
    }
    order.extend([Focus::XEnd, Focus::H]);
    if nav.slope_bounds {
        order.extend([
            Focus::ViewXMin,
            Focus::ViewXMax,
            Focus::ViewYMin,
            Focus::ViewYMax,
        ]);
    }
    order.extend([Focus::ExportButton, Focus::QuitButton]);
    order
}

fn step(from: Focus, nav: FocusNav, forward: bool) -> Focus {
    let order = focus_order(nav);
    let i = order.iter().position(|&f| f == from).unwrap_or(0);
    let n = order.len();
    let next_i = if forward {
        (i + 1) % n
    } else {
        (i + n - 1) % n
    };
    order[next_i]
}

/// Focus target inside the export dialog.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_prompt_focus_cycles() {
        assert_eq!(
            ExportPromptFocus::Filename.next(),
            ExportPromptFocus::H
        );
        assert_eq!(
            ExportPromptFocus::H.next(),
            ExportPromptFocus::NumPoints
        );
        assert_eq!(
            ExportPromptFocus::NumPoints.next(),
            ExportPromptFocus::Filename
        );
        assert_eq!(ExportPromptFocus::Filename.prev(), ExportPromptFocus::NumPoints);
    }

    #[test]
    fn focus_is_text_input() {
        assert!(Focus::Equation.is_text_input());
        assert!(Focus::ViewYMax.is_text_input());
        assert!(!Focus::GraphDisplay.is_text_input());
        assert!(!Focus::ExportButton.is_text_input());
    }

    #[test]
    fn focus_nav_with_y0_family_and_slope_bounds() {
        let nav = FocusNav {
            y0_family: true,
            slope_bounds: true,
        };
        let mut f = Focus::H;
        f = f.next(nav);
        assert_eq!(f, Focus::ViewXMin);
        f = f.prev(nav);
        assert_eq!(f, Focus::H);
        f = f.prev(nav);
        assert_eq!(f, Focus::XEnd);
        f = Focus::Y0Count.next(nav);
        assert_eq!(f, Focus::XEnd);
    }

    #[test]
    fn focus_prev_full_ring_without_extras() {
        let nav = FocusNav {
            y0_family: false,
            slope_bounds: false,
        };
        assert_eq!(Focus::ExportButton.prev(nav), Focus::H);
        assert_eq!(Focus::QuitButton.prev(nav), Focus::ExportButton);
    }
}
