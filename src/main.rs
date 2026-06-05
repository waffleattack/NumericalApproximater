//! Terminal entry point: event loop, keyboard handling, and screen setup.

use std::io;

use anyhow::Result;
use numerical_approximater::app::{user_message, App, Focus};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, terminal::ClearType};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

/// Application entry point.
///
/// Initializes the terminal, runs the main loop, and restores the terminal on exit.
///
/// # Returns
///
/// `Ok(())` when the user quits normally, or an error if terminal I/O fails.
///
/// # Errors
///
/// Returns any error from terminal setup, drawing, or event polling.
fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(terminal)?;
    result
}

/// Put the terminal into raw mode and enter the alternate screen.
///
/// # Returns
///
/// A ratatui `Terminal` backed by stdout.
///
/// # Errors
///
/// Returns an error if raw mode, screen switching, or terminal creation fails.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    stdout.execute(crossterm::terminal::Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Leave alternate screen, disable raw mode, and show the cursor.
///
/// # Arguments
///
/// * `terminal` - Terminal instance to tear down.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if restoring the terminal state fails.
fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Main event loop: draw the UI and dispatch keyboard input until quit.
///
/// # Arguments
///
/// * `terminal` - Active ratatui terminal used for rendering.
///
/// # Returns
///
/// `Ok(())` when the user exits with `q` or Ctrl+C.
///
/// # Errors
///
/// Returns an error if drawing or reading input fails.
fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();

    loop {
        terminal.hide_cursor()?;
        terminal.draw(|f| numerical_approximater::ui::draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                {
                    break;
                }
                if handle_key(&mut app, key.code, key.modifiers) {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Handle a key press in the main (non-modal) UI.
///
/// # Arguments
///
/// * `app` - Mutable application state.
/// * `code` - Key that was pressed.
/// * `_mods` - Modifier keys (currently unused).
///
/// # Returns
///
/// `true` if the application should exit, `false` otherwise.
fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> bool {
    if app.export_prompt_open {
        return handle_export_prompt(app, code);
    }

    if app.method_menu_open {
        return handle_method_menu(app, code);
    }

    match code {
        KeyCode::Char('q') if !app.focus.is_text_input() => return true,
        KeyCode::Char('s') if !app.focus.is_text_input() => app.open_export_prompt(),
        KeyCode::Tab => app.focus = app.focus.next(app.y0_family_enabled),
        KeyCode::BackTab => app.focus = app.focus.prev(app.y0_family_enabled),
        KeyCode::Up => app.focus = app.focus.prev(app.y0_family_enabled),
        KeyCode::Down => app.focus = app.focus.next(app.y0_family_enabled),
        KeyCode::Enter => {
            if handle_activate(app) {
                return true;
            }
        }
        KeyCode::Char(' ') if app.focus.is_text_input() => {
            handle_text_input_key(app, code);
        }
        KeyCode::Char(' ') => {
            if handle_activate(app) {
                return true;
            }
        }
        KeyCode::Esc => {}
        KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End | KeyCode::Delete
        | KeyCode::Backspace | KeyCode::Char(_) => {
            if app.focus.is_text_input() {
                handle_text_input_key(app, code);
            }
        }
        _ => {}
    }
    false
}

/// Activate the focused control (Enter everywhere; Space when not in a text field).
///
/// # Returns
///
/// `true` if the application should exit.
fn handle_activate(app: &mut App) -> bool {
    match app.focus {
        Focus::Equation
        | Focus::X0
        | Focus::Y0
        | Focus::Y0End
        | Focus::Y0Count
        | Focus::XEnd
        | Focus::H => {
            recompute_with_feedback(app);
            false
        }
        Focus::Y0Family => {
            app.toggle_y0_family();
            recompute_with_feedback(app);
            false
        }
        Focus::MethodDropdown => {
            app.open_method_menu();
            false
        }
        Focus::ExportButton => {
            app.open_export_prompt();
            false
        }
        Focus::QuitButton => true,
    }
}

/// Run `recompute` and store any error in the footer status bar.
///
/// # Arguments
///
/// * `app` - Application state to update with curves or an error message.
fn recompute_with_feedback(app: &mut App) {
    if let Err(e) = app.recompute() {
        app.error = Some(user_message(e));
        app.status = None;
    }
}

/// Handle keyboard input while the export dialog is open.
///
/// # Arguments
///
/// * `app` - Mutable application state.
/// * `code` - Key that was pressed.
///
/// # Returns
///
/// `true` if the application should exit, `false` otherwise.
fn handle_export_prompt(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc => {
            let _ = app.close_export_prompt(false);
        }
        KeyCode::Enter => {
            if let Err(e) = app.close_export_prompt(true) {
                app.error = Some(user_message(e));
                app.status = None;
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            app.export_prompt_focus = app.export_prompt_focus.next();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.export_prompt_focus = app.export_prompt_focus.prev();
        }
        KeyCode::Left => app.export_prompt_input_mut().cursor_left(),
        KeyCode::Right => app.export_prompt_input_mut().cursor_right(),
        KeyCode::Home => app.export_prompt_input_mut().cursor_home(),
        KeyCode::End => app.export_prompt_input_mut().cursor_end(),
        KeyCode::Delete => app.export_prompt_input_mut().delete(),
        KeyCode::Backspace => app.export_prompt_input_mut().backspace(),
        KeyCode::Char(c) => app.export_prompt_input_mut().insert(c),
        _ => {}
    }
    false
}

/// Apply an editing key to the currently focused text field.
///
/// # Arguments
///
/// * `app` - Application state; uses `focus` to select the active `TextInput`.
/// * `code` - Key that was pressed.
fn handle_text_input_key(app: &mut App, code: KeyCode) {
    let Some(input) = app.focused_input_mut() else {
        return;
    };
    match code {
        KeyCode::Left => input.cursor_left(),
        KeyCode::Right => input.cursor_right(),
        KeyCode::Home => input.cursor_home(),
        KeyCode::End => input.cursor_end(),
        KeyCode::Delete => input.delete(),
        KeyCode::Backspace => input.backspace(),
        KeyCode::Char(c) => input.insert(c),
        _ => {}
    }
}

/// Handle keyboard input while the method selection menu is open.
///
/// # Arguments
///
/// * `app` - Mutable application state.
/// * `code` - Key that was pressed.
///
/// # Returns
///
/// `true` if the application should exit, `false` otherwise.
fn handle_method_menu(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => app.close_method_menu(false),
        KeyCode::Enter | KeyCode::Char(' ') => app.close_method_menu(true),
        KeyCode::Up => app.method_menu_up(),
        KeyCode::Down => app.method_menu_down(),
        _ => {}
    }
    false
}
