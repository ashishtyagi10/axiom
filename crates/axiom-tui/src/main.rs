//! Axiom TUI - Terminal User Interface for Axiom
//!
//! This is the main entry point for the Ratatui-based terminal interface.

use axiom_core::config::load_config;
use axiom_tui::{AxiomConfig, Result, TuiApp};
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

fn main() -> Result<()> {
    // Get current working directory
    let cwd = std::env::current_dir().unwrap_or_else(|_| {
        dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
    });

    // Load configuration
    let config = load_config(&cwd).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        AxiomConfig::default()
    });

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = TuiApp::new(config, cwd)?;
    let result = app.run(&mut terminal);

    // Restore terminal (ALWAYS, even on error)
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Print any error
    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }

    result
}
