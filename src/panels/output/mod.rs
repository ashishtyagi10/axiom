//! Output panel - context-aware display area
//!
//! Displays either file content or agent output based on the current OutputContext.
//! Delegates rendering to specialized sub-viewers.
//! For CLI agents, renders an interactive terminal with full vt100 support.

mod agent_viewer;
mod file_viewer;

pub use agent_viewer::AgentViewer;
pub use file_viewer::FileViewer;

use crate::agents::{Agent, AgentRegistry, AgentType, PtyAgentManager};
use crate::core::Result;
use crate::events::Event;
use crate::panels::Panel;
use crate::state::{AgentId, AppState, OutputContext, PanelId};
use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind};
use parking_lot::RwLock;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Scroll state for a context
#[derive(Debug, Clone, Default)]
struct ScrollState {
    offset: usize,
}

/// Central output panel - displays content based on context
pub struct OutputPanel {
    /// Current output context
    context: OutputContext,

    /// File viewer for displaying file content
    file_viewer: FileViewer,

    /// Agent viewer for displaying agent output
    agent_viewer: AgentViewer,

    /// Scroll states preserved per context
    scroll_states: HashMap<String, ScrollState>,

    /// Reference to agent registry for viewing agent output
    agent_registry: Arc<RwLock<AgentRegistry>>,

    /// Reference to PTY agent manager for CLI agent rendering
    pty_manager: Option<Arc<RwLock<PtyAgentManager>>>,

    /// Event sender for routing keyboard input to CLI agents
    event_tx: Option<Sender<Event>>,

    /// Visible height for scrolling calculations
    visible_height: usize,

    /// Current panel dimensions (for PTY sizing)
    panel_cols: u16,
    panel_rows: u16,

    /// Content area for scroll detection
    content_area: RefCell<Rect>,
}

impl OutputPanel {
    /// Create a new output panel
    pub fn new(agent_registry: Arc<RwLock<AgentRegistry>>) -> Self {
        Self {
            context: OutputContext::Empty,
            file_viewer: FileViewer::new(),
            agent_viewer: AgentViewer::new(),
            scroll_states: HashMap::new(),
            agent_registry,
            pty_manager: None,
            event_tx: None,
            visible_height: 20,
            panel_cols: 80,
            panel_rows: 24,
            content_area: RefCell::new(Rect::default()),
        }
    }

    /// Set the PTY agent manager for CLI agent rendering
    pub fn set_pty_manager(&mut self, manager: Arc<RwLock<PtyAgentManager>>, event_tx: Sender<Event>) {
        // Set the default PTY size based on current panel dimensions
        let pty_cols = self.panel_cols.saturating_sub(2);
        let pty_rows = self.panel_rows.saturating_sub(2);
        manager.write().set_default_size(pty_cols, pty_rows);

        self.pty_manager = Some(manager);
        self.event_tx = Some(event_tx);
    }

    /// Check if the current context is a CLI agent
    fn is_cli_agent(&self) -> bool {
        if let OutputContext::Agent { agent_id } = &self.context {
            let registry = self.agent_registry.read();
            if let Some(agent) = registry.get(*agent_id) {
                return agent.agent_type.is_cli_agent();
            }
        }
        false
    }

    /// Get the current CLI agent ID if viewing one
    fn current_cli_agent_id(&self) -> Option<AgentId> {
        if let OutputContext::Agent { agent_id } = &self.context {
            let registry = self.agent_registry.read();
            if let Some(agent) = registry.get(*agent_id) {
                if agent.agent_type.is_cli_agent() {
                    return Some(*agent_id);
                }
            }
        }
        None
    }

    /// Get the current context
    pub fn context(&self) -> &OutputContext {
        &self.context
    }

    /// Set the output context
    pub fn set_context(&mut self, context: OutputContext) {
        // Save current scroll state
        let key = self.context_key(&self.context);
        if let Some(state) = self.get_current_scroll() {
            self.scroll_states.insert(key, state);
        }

        self.context = context;

        // Load file content if switching to file context
        if let OutputContext::File { ref path } = self.context {
            self.file_viewer.load_file(path);
        }

        // Restore scroll state for new context
        let new_key = self.context_key(&self.context);
        if let Some(state) = self.scroll_states.get(&new_key) {
            self.apply_scroll(state.clone());
        }
    }

    /// Generate a key for the scroll state map
    fn context_key(&self, ctx: &OutputContext) -> String {
        match ctx {
            OutputContext::File { path } => format!("file:{}", path.display()),
            OutputContext::Agent { agent_id } => format!("agent:{}", agent_id),
            OutputContext::Empty => "empty".to_string(),
        }
    }

    /// Get current scroll state
    fn get_current_scroll(&self) -> Option<ScrollState> {
        match &self.context {
            OutputContext::File { .. } => Some(ScrollState {
                offset: self.file_viewer.scroll_offset(),
            }),
            OutputContext::Agent { .. } => Some(ScrollState {
                offset: self.agent_viewer.scroll_offset(),
            }),
            OutputContext::Empty => None,
        }
    }

    /// Apply scroll state
    fn apply_scroll(&mut self, state: ScrollState) {
        match &self.context {
            OutputContext::File { .. } => self.file_viewer.set_scroll_offset(state.offset),
            OutputContext::Agent { .. } => self.agent_viewer.set_scroll_offset(state.offset),
            OutputContext::Empty => {}
        }
    }

    /// Scroll up by given number of lines
    fn scroll_up(&mut self, lines: usize) {
        match &self.context {
            OutputContext::File { .. } => self.file_viewer.scroll_up(lines),
            OutputContext::Agent { .. } => self.agent_viewer.scroll_up(lines),
            OutputContext::Empty => {}
        }
    }

    /// Scroll down by given number of lines
    fn scroll_down(&mut self, lines: usize) {
        match &self.context {
            OutputContext::File { .. } => self.file_viewer.scroll_down(lines),
            OutputContext::Agent { .. } => self.agent_viewer.scroll_down(lines),
            OutputContext::Empty => {}
        }
    }

    /// Get the title for the panel based on context
    fn title(&self) -> String {
        match &self.context {
            OutputContext::File { path } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                format!(" Output: {} ", name)
            }
            OutputContext::Agent { agent_id } => {
                let registry = self.agent_registry.read();
                if let Some(agent) = registry.get(*agent_id) {
                    format!(" {} {} ", agent.agent_type.icon(), agent.name)
                } else {
                    format!(" Agent {} ", agent_id)
                }
            }
            OutputContext::Empty => " Output ".to_string(),
        }
    }
}

impl Panel for OutputPanel {
    fn id(&self) -> PanelId {
        PanelId::OUTPUT
    }

    fn name(&self) -> &str {
        "Output"
    }

    fn handle_input(&mut self, event: &Event, _state: &mut AppState) -> Result<bool> {
        // If viewing a CLI agent, route keyboard input to the PTY
        if let Some(agent_id) = self.current_cli_agent_id() {
            if let Event::Key(key) = event {
                // Convert key to bytes and send to PTY
                let bytes = key_to_bytes(key.code, key.modifiers);
                if !bytes.is_empty() {
                    if let Some(ref event_tx) = self.event_tx {
                        let _ = event_tx.send(Event::CliAgentInput {
                            id: agent_id,
                            data: bytes,
                        });
                    }
                    return Ok(true);
                }
            }
        }

        match event {
            Event::Key(key) => {
                match (key.code, key.modifiers) {
                    // Scroll with arrow keys
                    (KeyCode::Up, KeyModifiers::NONE) => {
                        self.scroll_up(1);
                        Ok(true)
                    }
                    (KeyCode::Down, KeyModifiers::NONE) => {
                        self.scroll_down(1);
                        Ok(true)
                    }
                    // Page up/down
                    (KeyCode::PageUp, _) => {
                        self.scroll_up(self.visible_height.saturating_sub(2));
                        Ok(true)
                    }
                    (KeyCode::PageDown, _) => {
                        self.scroll_down(self.visible_height.saturating_sub(2));
                        Ok(true)
                    }
                    // Home/End
                    (KeyCode::Home, KeyModifiers::CONTROL) => {
                        self.apply_scroll(ScrollState { offset: 0 });
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_up(3);
                    Ok(true)
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_down(3);
                    Ok(true)
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        // Store content area for input handling
        *self.content_area.borrow_mut() = area;

        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = self.title();
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render based on context
        match &self.context {
            OutputContext::File { .. } => {
                self.file_viewer.render(frame, inner);
            }
            OutputContext::Agent { agent_id } => {
                let agent_id = *agent_id;
                let registry = self.agent_registry.read();
                if let Some(agent) = registry.get(agent_id) {
                    // Check if this is a CLI agent - render using vt100
                    if agent.agent_type.is_cli_agent() {
                        let agent_name = agent.name.clone();
                        let agent_status = agent.status.clone();
                        drop(registry);

                        // Get screen lines from PTY manager
                        if let Some(ref pty_manager) = self.pty_manager {
                            let manager = pty_manager.read();
                            if let Some(lines) = manager.get_screen_lines(agent_id) {
                                drop(manager);
                                // Render the terminal screen
                                let paragraph = Paragraph::new(lines);
                                frame.render_widget(paragraph, inner);
                            } else {
                                drop(manager);
                                // PTY not found - show waiting message
                                let msg = Paragraph::new(vec![
                                    Line::from(""),
                                    Line::from(Span::styled(
                                        format!("Starting {}...", agent_name),
                                        Style::default().fg(Color::DarkGray),
                                    )),
                                ]);
                                frame.render_widget(msg, inner);
                            }
                        } else {
                            // No PTY manager - show error
                            let msg = Paragraph::new("CLI agent rendering not available");
                            frame.render_widget(msg, inner);
                        }
                    } else {
                        // Regular agent - use standard viewer
                        // Clone agent data to avoid holding lock during render
                        let agent_clone = Agent {
                            id: agent.id,
                            agent_type: agent.agent_type.clone(),
                            name: agent.name.clone(),
                            description: agent.description.clone(),
                            status: agent.status.clone(),
                            output: agent.output.clone(),
                            created_at: agent.created_at,
                            completed_at: agent.completed_at,
                            token_count: agent.token_count,
                            line_count: agent.line_count,
                            progress: agent.progress,
                            parent_id: agent.parent_id,
                        };

                        // Get children for aggregated output (only for Conductor)
                        let children: Vec<Agent> = registry
                            .children(agent_id)
                            .into_iter()
                            .map(|c| Agent {
                                id: c.id,
                                agent_type: c.agent_type.clone(),
                                name: c.name.clone(),
                                description: c.description.clone(),
                                status: c.status.clone(),
                                output: c.output.clone(),
                                created_at: c.created_at,
                                completed_at: c.completed_at,
                                token_count: c.token_count,
                                line_count: c.line_count,
                                progress: c.progress,
                                parent_id: c.parent_id,
                            })
                            .collect();

                        drop(registry);
                        self.agent_viewer.render(frame, inner, &agent_clone, &children);
                    }
                } else {
                    drop(registry);
                    let msg = Paragraph::new("Agent not found");
                    frame.render_widget(msg, inner);
                }
            }
            OutputContext::Empty => {
                let lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "No content selected",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Select a file from the tree or an agent from the right panel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let msg = Paragraph::new(lines);
                frame.render_widget(msg, inner);
            }
        }
    }

    fn on_resize(&mut self, cols: u16, rows: u16) {
        // Store panel dimensions for future PTY creation
        self.panel_cols = cols;
        self.panel_rows = rows;

        self.visible_height = rows.saturating_sub(2) as usize;
        self.file_viewer.set_visible_height(self.visible_height);
        self.agent_viewer.set_visible_height(self.visible_height);

        // Resize PTY agents to match panel size (account for borders)
        let pty_cols = cols.saturating_sub(2);
        let pty_rows = rows.saturating_sub(2);
        if let Some(ref pty_manager) = self.pty_manager {
            // Update default size for new agents and resize existing ones
            let mut manager = pty_manager.write();
            manager.set_default_size(pty_cols, pty_rows);
            let _ = manager.resize_all(pty_cols, pty_rows);
        }
    }
}

/// Convert a key event to bytes for PTY input
fn key_to_bytes(code: KeyCode, modifiers: KeyModifiers) -> Vec<u8> {
    match code {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                // Control characters (Ctrl+A = 1, Ctrl+B = 2, etc.)
                let ctrl_char = (c.to_ascii_lowercase() as u8).wrapping_sub(b'a' - 1);
                vec![ctrl_char]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(n) => match n {
            1 => b"\x1bOP".to_vec(),
            2 => b"\x1bOQ".to_vec(),
            3 => b"\x1bOR".to_vec(),
            4 => b"\x1bOS".to_vec(),
            5 => b"\x1b[15~".to_vec(),
            6 => b"\x1b[17~".to_vec(),
            7 => b"\x1b[18~".to_vec(),
            8 => b"\x1b[19~".to_vec(),
            9 => b"\x1b[20~".to_vec(),
            10 => b"\x1b[21~".to_vec(),
            11 => b"\x1b[23~".to_vec(),
            12 => b"\x1b[24~".to_vec(),
            _ => vec![],
        },
        _ => vec![],
    }
}
