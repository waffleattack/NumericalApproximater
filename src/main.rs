mod app;
mod export;
mod expr;
mod format;
mod input;
mod solver;
mod ui;

use std::io;

use anyhow::Result;
use app::{App, Focus};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, terminal::ClearType};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    stdout.execute(crossterm::terminal::Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();

    loop {
        terminal.hide_cursor()?;
        terminal.draw(|f| ui::draw(f, &app))?;

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

fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> bool {
    if app.export_prompt_open {
        return handle_export_prompt(app, code);
    }

    if app.method_menu_open {
        return handle_method_menu(app, code);
    }

    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Tab => app.focus = app.focus.next(app.y0_family_enabled),
        KeyCode::BackTab => app.focus = app.focus.prev(app.y0_family_enabled),
        KeyCode::Up => app.focus = app.focus.prev(app.y0_family_enabled),
        KeyCode::Down => app.focus = app.focus.next(app.y0_family_enabled),
        KeyCode::Enter => match app.focus {
            Focus::Equation
            | Focus::X0
            | Focus::Y0
            | Focus::Y0End
            | Focus::Y0Count
            | Focus::XEnd
            | Focus::H => {
                if let Err(e) = app.recompute() {
                    app.error = Some(e.to_string());
                }
            }
            Focus::Y0Family => {
                app.toggle_y0_family();
                let _ = app.recompute();
            }
            Focus::MethodDropdown => app.open_method_menu(),
            Focus::ExportButton => app.open_export_prompt(),
        },
        KeyCode::Esc => {}
        KeyCode::Char(' ') if app.focus == Focus::MethodDropdown => app.open_method_menu(),
        KeyCode::Char(' ') if app.focus == Focus::Y0Family => {
            app.toggle_y0_family();
            if let Err(e) = app.recompute() {
                app.error = Some(e.to_string());
            }
        }
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

fn handle_export_prompt(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Esc => {
            let _ = app.close_export_prompt(false);
        }
        KeyCode::Enter => {
            if let Err(e) = app.close_export_prompt(true) {
                app.error = Some(e.to_string());
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
