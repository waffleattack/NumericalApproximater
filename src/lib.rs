//! Library crate for the numerical ODE approximator (shared by the TUI binary and benchmarks).
//!
//! # How to read the code
//!
//! 1. **`main`** — terminal setup, event loop, keyboard routing ([`main.rs`](main.rs)).
//! 2. **`app`** — [`App`] state, focus/tab order ([`app::focus`]), methods and curves ([`app::model`]),
//!    [`App::recompute`] and export flow ([`app`]).
//! 3. **`expr`** — parse **F(x, y)** ([`expr::parse`]), evaluate via [`OdeFunction`].
//! 4. **`solver`** — fixed-step integration and plot subsampling.
//! 5. **`ui`** — [`ui::draw`] composes equation bar, sidebar, chart, footer ([`ui`]).
//! 6. **`export`**, **`format`**, **`input`** — file output, number formatting, text fields.

pub mod app;
pub mod export;
pub mod expr;
pub mod format;
pub mod input;
pub mod solver;
pub mod ui;
