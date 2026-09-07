use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    fmt,
    io::{self, Stdout},
};

#[derive(Debug)]
pub struct LifecycleError;

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("The interactive terminal could not be initialized or restored.")
    }
}

impl std::error::Error for LifecycleError {}

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    raw: bool,
    alternate: bool,
    cursor_hidden: bool,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self, LifecycleError> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(|_| LifecycleError)?;
        let mut guard = Self {
            terminal,
            raw: false,
            alternate: false,
            cursor_hidden: false,
        };
        enable_raw_mode().map_err(|_| LifecycleError)?;
        guard.raw = true;
        execute!(guard.terminal.backend_mut(), EnterAlternateScreen).map_err(|_| LifecycleError)?;
        guard.alternate = true;
        execute!(guard.terminal.backend_mut(), Hide).map_err(|_| LifecycleError)?;
        guard.cursor_hidden = true;
        guard.terminal.clear().map_err(|_| LifecycleError)?;
        Ok(guard)
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    pub fn restore(&mut self) -> Result<(), LifecycleError> {
        let mut failed = false;
        if self.cursor_hidden {
            failed |= execute!(self.terminal.backend_mut(), Show).is_err();
            self.cursor_hidden = false;
        }
        if self.alternate {
            failed |= execute!(self.terminal.backend_mut(), LeaveAlternateScreen).is_err();
            self.alternate = false;
        }
        if self.raw {
            failed |= disable_raw_mode().is_err();
            self.raw = false;
        }
        failed |= self.terminal.show_cursor().is_err();
        if failed { Err(LifecycleError) } else { Ok(()) }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
