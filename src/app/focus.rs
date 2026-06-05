//! Keyboard focus and tab order in the main UI and export dialog.

/// Keyboard focus target in the main UI.
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
    QuitButton,
}

impl Focus {
    /// Move focus to the next field in tab order.
    ///
    /// # Arguments
    ///
    /// * `y0_family` - Whether the extended y₀ family fields are visible.
    ///
    /// # Returns
    ///
    /// The next focus target, wrapping at the end.
    pub fn next(self, y0_family: bool) -> Self {
        step(self, y0_family, true)
    }

    /// Move focus to the previous field in tab order.
    ///
    /// # Arguments
    ///
    /// * `y0_family` - Whether the extended y₀ family fields are visible.
    ///
    /// # Returns
    ///
    /// The previous focus target, wrapping at the start.
    pub fn prev(self, y0_family: bool) -> Self {
        step(self, y0_family, false)
    }

    /// Return whether this focus target is an editable text field.
    ///
    /// # Returns
    ///
    /// `true` for equation and numeric inputs; `false` for buttons and toggles.
    pub fn is_text_input(self) -> bool {
        !matches!(
            self,
            Focus::MethodDropdown | Focus::ExportButton | Focus::QuitButton | Focus::Y0Family
        )
    }
}

/// Advance or retreat one step in the focus ring.
///
/// # Arguments
///
/// * `from` - Current focus.
/// * `y0_family` - Whether y₀ end/count fields are in the tab order.
/// * `forward` - `true` for next, `false` for previous.
///
/// # Returns
///
/// The new focus value, wrapping at the ends of the ring.
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
            Focus::QuitButton,
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
            Focus::QuitButton,
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

    /// Next field in the export dialog tab order.
    ///
    /// # Returns
    ///
    /// The following [`ExportPromptFocus`], wrapping at the end.
    pub fn next(self) -> Self {
        let i = Self::ORDER.iter().position(|&f| f == self).unwrap_or(0);
        Self::ORDER[(i + 1) % Self::ORDER.len()]
    }

    /// Previous field in the export dialog tab order.
    ///
    /// # Returns
    ///
    /// The preceding [`ExportPromptFocus`], wrapping at the start.
    pub fn prev(self) -> Self {
        let i = Self::ORDER.iter().position(|&f| f == self).unwrap_or(0);
        Self::ORDER[(i + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }
}
