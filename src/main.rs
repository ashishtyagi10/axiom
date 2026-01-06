//! Axiom - Terminal IDE with AI integration
//!
//! Entry point with proper terminal setup and cleanup.

use axiom::{
    core::Result,
    events::{Event, EventBus},
    panels::{Panel, PanelRegistry},
    state::AppState,
    ui,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

fn main() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    // Run app with panic recovery
    let result = run_app(&mut term);

    // Restore terminal (ALWAYS, even on error)
    terminal::disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    // Print any error
    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }

    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    // Create event bus with bounded channel
    let event_bus = EventBus::new(1024);

    // Create application state
    let mut state = AppState::new();

    // Create panels
    let mut panels = PanelRegistry::new(event_bus.sender(), &state.cwd)?;

    // Spawn input reader thread
    spawn_input_reader(event_bus.sender());

    // Get initial terminal size and notify panels
    let size = terminal.size()?;
    let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
    let layout = ui::get_layout_with_focus(area, Some(state.focus.current()));
    panels.notify_resize_all(&layout);

    // Track screen area for layout calculations
    let mut screen_area = ratatui::layout::Rect::new(0, 0, size.width, size.height);

    // Main event loop
    loop {
        // Render
        terminal.draw(|frame| {
            ui::render(frame, &state, &mut panels);
        })?;

        // Process events with timeout (50ms for responsive UI)
        if let Some(event) = event_bus.recv_timeout(Duration::from_millis(50)) {
            if handle_event(&event, &mut state, &mut panels, screen_area)? {
                break; // Quit requested
            }

            // Handle resize events
            if let Event::Resize(w, h) = event {
                screen_area = ratatui::layout::Rect::new(0, 0, w, h);
                let layout = ui::get_layout_with_focus(screen_area, Some(state.focus.current()));
                panels.notify_resize_all(&layout);
            }
        }

        // Drain additional events to prevent lag
        for event in event_bus.drain(50) {
            if handle_event(&event, &mut state, &mut panels, screen_area)? {
                break;
            }
        }

        // Check if file tree wants to open a file (auto-open on selection)
        if let Some(path) = panels.file_tree.take_pending_open() {
            if let Err(e) = panels.editor.open(&path) {
                state.error(format!("Failed to open: {}", e));
            }
            // Don't switch focus - let user keep navigating file tree
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

/// Handle a single event
///
/// Returns Ok(true) if quit was requested.
fn handle_event(
    event: &Event,
    state: &mut AppState,
    panels: &mut PanelRegistry,
    screen_area: ratatui::layout::Rect,
) -> Result<bool> {
    match event {
        // Global key bindings (checked first)
        Event::Key(key) => {
            // Handle model selector modal
            if state.input_mode.is_modal_open("model_selector") {
                match key.code {
                    KeyCode::Esc => {
                        state.input_mode.to_normal();
                        return Ok(false);
                    }
                    KeyCode::Up => {
                        panels.model_selector.up();
                        return Ok(false);
                    }
                    KeyCode::Down => {
                        panels.model_selector.down();
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        if let Some(model) = panels.apply_selected_model() {
                            state.info(format!("Model changed to: {}", model));
                        }
                        state.input_mode.to_normal();
                        return Ok(false);
                    }
                    _ => return Ok(false),
                }
            }

            // 'q' in Normal mode: Quit (vim-style, avoids Ctrl+Q terminal conflict)
            if key.code == KeyCode::Char('q') && !state.input_mode.is_editing() && !state.input_mode.is_modal() {
                state.quit();
                return Ok(true);
            }

            // Ctrl+M: Open model selector
            if key.code == KeyCode::Char('m') && key.modifiers.contains(KeyModifiers::CONTROL) {
                panels.open_model_selector();
                state.input_mode.open_modal("model_selector");
                return Ok(false);
            }

            // Tab: Cycle focus
            if key.code == KeyCode::Tab && !state.input_mode.is_editing() && !state.input_mode.is_modal() {
                state.focus.next();
                panels.handle_focus_change(state.focus.current(), screen_area);
                return Ok(false);
            }

            // Backtab (Shift+Tab): Cycle focus backwards
            if key.code == KeyCode::BackTab && !state.input_mode.is_editing() && !state.input_mode.is_modal() {
                state.focus.prev();
                panels.handle_focus_change(state.focus.current(), screen_area);
                return Ok(false);
            }

            // Escape: Return to normal mode
            if key.code == KeyCode::Esc {
                state.input_mode.to_normal();
                return Ok(false);
            }

            // 'i' in Normal mode: Enter insert mode
            if key.code == KeyCode::Char('i') && !state.input_mode.is_editing() && !state.input_mode.is_modal() {
                state.input_mode.to_insert();
                return Ok(false);
            }

            // Forward to focused panel
            let focused = state.focus.current();
            panels.get_mut(focused).handle_input(event, state)?;
        }

        // Mouse events
        Event::Mouse(mouse) => {
            let x = mouse.column;
            let y = mouse.row;

            // Handle model selector modal mouse events
            if state.input_mode.is_modal_open("model_selector") {
                match mouse.kind {
                    event::MouseEventKind::Down(event::MouseButton::Left) => {
                        // Check if click is inside modal
                        if panels.model_selector.contains(x, y) {
                            // Check if click is on a model item
                            if panels.model_selector.handle_click(x, y) {
                                // Item was clicked, apply selection
                                if let Some(model) = panels.apply_selected_model() {
                                    state.info(format!("Model: {}", model));
                                }
                                state.input_mode.to_normal();
                            }
                        } else {
                            // Click outside modal - close it
                            state.input_mode.to_normal();
                        }
                    }
                    event::MouseEventKind::ScrollUp => {
                        if panels.model_selector.contains(x, y) {
                            panels.model_selector.handle_scroll(false);
                        }
                    }
                    event::MouseEventKind::ScrollDown => {
                        if panels.model_selector.contains(x, y) {
                            panels.model_selector.handle_scroll(true);
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }

            // Handle click on model badge in status bar
            if let event::MouseEventKind::Down(event::MouseButton::Left) = mouse.kind {
                if panels.is_model_badge_click(x, y) {
                    panels.open_model_selector();
                    state.input_mode.open_modal("model_selector");
                    return Ok(false);
                }
            }
        }

        // PTY output - route to terminal panel
        Event::PtyOutput(_) | Event::PtyExit(_) => {
            panels.terminal.handle_input(event, state)?;
        }

        // LLM events - route to chat panel
        Event::LlmChunk(_) | Event::LlmDone | Event::LlmError(_) => {
            panels.chat.handle_input(event, state)?;
        }

        // Resize is handled in main loop
        Event::Resize(_, _) => {}

        // Tick - could be used for animations
        Event::Tick => {}

        // Other events
        _ => {}
    }

    Ok(false)
}

/// Spawn thread to read input events
fn spawn_input_reader(tx: crossbeam_channel::Sender<Event>) {
    std::thread::spawn(move || {
        loop {
            // Poll with timeout to allow thread shutdown
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                match event::read() {
                    Ok(event::Event::Key(key)) => {
                        if tx.send(Event::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(event::Event::Mouse(mouse)) => {
                        if tx.send(Event::Mouse(mouse)).is_err() {
                            break;
                        }
                    }
                    Ok(event::Event::Resize(w, h)) => {
                        if tx.send(Event::Resize(w, h)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    });
}
