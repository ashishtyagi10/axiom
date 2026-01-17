//! TuiApp - Main application struct using AxiomService
//!
//! This is the bridge between the TUI layer and the backend.
//! It handles terminal events, sends Commands, and reacts to Notifications.

use axiom_core::{AxiomConfig, AxiomService, Command, Notification, OutputContext, Result};
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph};
use std::path::PathBuf;
use std::time::Duration;

use crate::events::TuiEvent;
use crate::panels::{AgentsPanel, FileTreePanel, InputPanel, OutputPanel, Panel};
use crate::state::{AppState, MessageLevel, PanelId};

/// Main TUI application
///
/// Owns the AxiomService and manages the TUI event loop.
pub struct TuiApp {
    /// Backend service
    service: AxiomService,

    /// TUI-specific state
    state: AppState,

    /// Panel instances
    file_tree: FileTreePanel,
    output: OutputPanel,
    input: InputPanel,
    agents: AgentsPanel,
}

impl TuiApp {
    /// Create a new TuiApp
    pub fn new(config: AxiomConfig, cwd: PathBuf) -> Result<Self> {
        let service = AxiomService::new(config, cwd.clone())?;

        Ok(Self {
            service,
            state: AppState::new(),
            file_tree: FileTreePanel::new(&cwd),
            output: OutputPanel::new(),
            input: InputPanel::new(),
            agents: AgentsPanel::new(),
        })
    }

    /// Run the main event loop
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Process backend events
            self.service.process_events()?;

            // Update panels from service
            self.output.update_from_service(&self.service);
            self.agents.update_from_service(&self.service);

            // Process pending actions from panels
            self.process_panel_actions()?;

            // Poll notifications from backend
            while let Some(notification) = self.service.poll_notification() {
                self.handle_notification(notification);
            }

            // Render
            terminal.draw(|frame| self.render(frame))?;

            // Poll terminal events with timeout
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    CrosstermEvent::Key(key) => {
                        if self.handle_key(key)? {
                            break; // Quit requested
                        }
                    }
                    CrosstermEvent::Mouse(mouse) => {
                        self.handle_mouse(mouse)?;
                    }
                    CrosstermEvent::Resize(cols, rows) => {
                        self.handle_resize(cols, rows)?;
                    }
                    _ => {}
                }
            }

            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Process pending actions from panels
    fn process_panel_actions(&mut self) -> Result<()> {
        // Check for file tree file open
        if let Some(path) = self.file_tree.take_pending_open() {
            self.output.set_context(OutputContext::File { path });
            self.state.focus.focus(PanelId::OUTPUT);
        }

        // Check for input panel commands
        if let Some(command) = self.input.take_pending_command() {
            self.service.send(command)?;
        }

        // Check for agents panel context switch
        if let Some(context) = self.agents.take_pending_context() {
            self.output.set_context(context);
        }

        Ok(())
    }

    /// Handle a key event
    fn handle_key(&mut self, key: event::KeyEvent) -> Result<bool> {
        // Global quit: 'q' in normal mode or Ctrl+C
        if (key.code == KeyCode::Char('q') && !self.state.input_mode.is_editing())
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.service.send(Command::Shutdown)?;
            return Ok(true);
        }

        // Tab to cycle focus
        if key.code == KeyCode::Tab && !self.state.input_mode.is_editing() {
            self.state.focus.next();
            return Ok(false);
        }

        // Shift+Tab to cycle focus backward
        if key.code == KeyCode::BackTab {
            self.state.focus.prev();
            return Ok(false);
        }

        // Escape: return to normal mode
        if key.code == KeyCode::Esc {
            self.state.input_mode.to_normal();
            return Ok(false);
        }

        // Create TuiEvent for panel handling
        let tui_event = TuiEvent::Key(key);

        // Dispatch to focused panel
        let handled = match self.state.focus.current() {
            PanelId::INPUT => self.input.handle_input(&tui_event, &mut self.state)?,
            PanelId::OUTPUT => {
                // Check for PTY input forwarding
                if let Some(id) = self.output.context().agent_id() {
                    let agents = self.service.agents();
                    if let Some(agent) = agents.iter().find(|a| a.id == id) {
                        if agent.agent_type.is_cli_agent() {
                            // Forward key to PTY
                            match key.code {
                                KeyCode::Char(c) => {
                                    self.service.send(Command::SendPtyInput {
                                        agent_id: id,
                                        data: vec![c as u8],
                                    })?;
                                    return Ok(false);
                                }
                                KeyCode::Enter => {
                                    self.service.send(Command::SendPtyInput {
                                        agent_id: id,
                                        data: vec![b'\r'],
                                    })?;
                                    return Ok(false);
                                }
                                KeyCode::Backspace => {
                                    self.service.send(Command::SendPtyInput {
                                        agent_id: id,
                                        data: vec![0x7f], // DEL
                                    })?;
                                    return Ok(false);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                self.output.handle_input(&tui_event, &mut self.state)?
            }
            PanelId::AGENTS => self.agents.handle_input(&tui_event, &mut self.state)?,
            PanelId::FILE_TREE => self.file_tree.handle_input(&tui_event, &mut self.state)?,
            _ => false,
        };

        let _ = handled; // Silence unused warning
        Ok(false)
    }

    /// Handle a mouse event
    fn handle_mouse(&mut self, mouse: event::MouseEvent) -> Result<()> {
        let tui_event = TuiEvent::Mouse(mouse);

        // Try each panel to see if it handles the mouse event
        if self.file_tree.handle_input(&tui_event, &mut self.state)? {
            self.state.focus.focus(PanelId::FILE_TREE);
        } else if self.output.handle_input(&tui_event, &mut self.state)? {
            self.state.focus.focus(PanelId::OUTPUT);
        } else if self.agents.handle_input(&tui_event, &mut self.state)? {
            self.state.focus.focus(PanelId::AGENTS);
        }

        Ok(())
    }

    /// Handle terminal resize
    fn handle_resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.file_tree.on_resize(cols, rows);
        self.output.on_resize(cols, rows);
        self.input.on_resize(cols, rows);
        self.agents.on_resize(cols, rows);
        Ok(())
    }

    /// Handle a notification from the backend
    fn handle_notification(&mut self, notification: Notification) {
        match notification {
            Notification::AgentSpawned { id, name, .. } => {
                self.state.info(format!("Started: {}", name));
                // Switch to view the new agent
                self.output.set_context(OutputContext::Agent { agent_id: id });
            }
            Notification::AgentStatusChanged { id, status } => {
                if status.is_terminal() {
                    if let Some(agent) = self.service.agents().iter().find(|a| a.id == id) {
                        self.state.info(format!("Completed: {}", agent.name));
                    }
                }
            }
            Notification::AgentOutput { .. } => {
                // Output is streaming, just trigger redraw
            }
            Notification::PtyOutput { .. } => {
                // PTY output, just trigger redraw
            }
            Notification::PtyExited { id, exit_code } => {
                if exit_code == 0 {
                    self.state.info("Agent completed");
                } else {
                    self.state
                        .error(format!("Agent exited with code {}", exit_code));
                }
                let _ = id; // Silence warning
            }
            Notification::FileModified { path } => {
                self.state.info(format!(
                    "Modified: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
            Notification::Error { message } => {
                self.state.error(message);
            }
            Notification::Info { message } => {
                self.state.info(message);
            }
            _ => {}
        }
    }

    /// Render the TUI
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Simple layout: left sidebar, center content, right sidebar
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // File tree
                Constraint::Percentage(60), // Output + Input
                Constraint::Percentage(20), // Agents
            ])
            .split(area);

        // File tree (left)
        self.file_tree.render(
            frame,
            chunks[0],
            self.state.focus.is_focused(PanelId::FILE_TREE),
        );

        // Center area: Output (top) + Input (bottom)
        let center_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Output
                Constraint::Length(3), // Input
                Constraint::Length(1), // Status bar
            ])
            .split(chunks[1]);

        self.output
            .render(frame, center_chunks[0], self.state.focus.is_focused(PanelId::OUTPUT));
        self.input
            .render(frame, center_chunks[1], self.state.focus.is_focused(PanelId::INPUT));
        self.render_status(frame, center_chunks[2]);

        // Agents list (right)
        self.agents
            .render(frame, chunks[2], self.state.focus.is_focused(PanelId::AGENTS));
    }

    /// Render status bar
    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let status_text = if let Some(ref msg) = self.state.status_message {
            let style = match msg.level {
                MessageLevel::Info => Style::default().fg(Color::Green),
                MessageLevel::Warning => Style::default().fg(Color::Yellow),
                MessageLevel::Error => Style::default().fg(Color::Red),
            };
            Span::styled(&msg.text, style)
        } else {
            let mode = match self.state.input_mode {
                crate::state::InputMode::Normal => "NORMAL",
                crate::state::InputMode::Insert => "INSERT",
                crate::state::InputMode::Command { .. } => "COMMAND",
                crate::state::InputMode::Search { .. } => "SEARCH",
                crate::state::InputMode::Modal { .. } => "MODAL",
            };
            Span::styled(
                format!(" {} | Tab to switch panels | q to quit ", mode),
                Style::default().fg(Color::Gray),
            )
        };

        let status = Paragraph::new(Line::from(status_text))
            .style(Style::default().bg(Color::DarkGray));

        frame.render_widget(status, area);
    }
}
