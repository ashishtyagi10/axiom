//! Output panel - context-aware display area
//!
//! Displays either file content or agent output based on the current OutputContext.
//! Delegates rendering to specialized sub-viewers.

mod agent_viewer;
mod file_viewer;

pub use agent_viewer::AgentViewer;
pub use file_viewer::FileViewer;

use crate::agents::{Agent, AgentRegistry};
use crate::core::Result;
use crate::events::Event;
use crate::panels::Panel;
use crate::state::{AgentId, AppState, OutputContext, PanelId};
use crossterm::event::{KeyCode, KeyModifiers, MouseEventKind};
use parking_lot::RwLock;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
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

    /// Visible height for scrolling calculations
    visible_height: usize,

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
            visible_height: 20,
            content_area: RefCell::new(Rect::default()),
        }
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

    fn on_resize(&mut self, _cols: u16, rows: u16) {
        self.visible_height = rows.saturating_sub(2) as usize;
        self.file_viewer.set_visible_height(self.visible_height);
        self.agent_viewer.set_visible_height(self.visible_height);
    }
}
