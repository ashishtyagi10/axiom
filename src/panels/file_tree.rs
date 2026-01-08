//! File tree panel for directory navigation

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crate::ui::ScrollBar;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::cell::Cell;
use std::path::{Path, PathBuf};

/// File or directory entry in the tree
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub depth: usize,
}

/// File tree panel
pub struct FileTreePanel {
    /// Root directory
    root: PathBuf,

    /// Flattened list of visible entries
    entries: Vec<FileEntry>,

    /// Currently selected index
    selected: usize,

    /// Scroll offset
    scroll: usize,

    /// File to open (set when Enter pressed on file)
    pub pending_open: Option<PathBuf>,

    /// Inner area for mouse click detection (updated during render)
    inner_area: Cell<Rect>,

    /// Current scroll offset for mouse click detection (updated during render)
    render_scroll: Cell<usize>,
}

impl FileTreePanel {
    /// Create new file tree panel
    pub fn new(root: &Path) -> Self {
        let mut panel = Self {
            root: root.to_path_buf(),
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            pending_open: None,
            inner_area: Cell::new(Rect::default()),
            render_scroll: Cell::new(0),
        };
        panel.refresh();
        panel
    }

    /// Take pending file to open (returns and clears it)
    pub fn take_pending_open(&mut self) -> Option<PathBuf> {
        self.pending_open.take()
    }

    /// Refresh the file tree
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.load_dir(&self.root.clone(), 0, true);
    }

    /// Load directory entries
    fn load_dir(&mut self, path: &Path, depth: usize, expanded: bool) {
        if !expanded {
            return;
        }

        let mut entries: Vec<_> = match std::fs::read_dir(path) {
            Ok(dir) => dir
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .collect(),
            Err(_) => return,
        };

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in entries {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            self.entries.push(FileEntry {
                path: entry_path.clone(),
                name,
                is_dir,
                expanded: false,
                depth,
            });
        }
    }

    /// Toggle expand/collapse of selected directory
    fn toggle_selected(&mut self) {
        if let Some(entry) = self.entries.get_mut(self.selected) {
            if entry.is_dir {
                entry.expanded = !entry.expanded;

                if entry.expanded {
                    // Insert children after current entry
                    let path = entry.path.clone();
                    let depth = entry.depth + 1;
                    let insert_at = self.selected + 1;

                    let mut children = Vec::new();
                    if let Ok(dir) = std::fs::read_dir(&path) {
                        let mut entries: Vec<_> = dir
                            .filter_map(|e| e.ok())
                            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                            .collect();

                        entries.sort_by(|a, b| {
                            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                            match (a_is_dir, b_is_dir) {
                                (true, false) => std::cmp::Ordering::Less,
                                (false, true) => std::cmp::Ordering::Greater,
                                _ => a.file_name().cmp(&b.file_name()),
                            }
                        });

                        for e in entries {
                            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                            children.push(FileEntry {
                                path: e.path(),
                                name: e.file_name().to_string_lossy().to_string(),
                                is_dir,
                                expanded: false,
                                depth,
                            });
                        }
                    }

                    // Insert children
                    for (i, child) in children.into_iter().enumerate() {
                        self.entries.insert(insert_at + i, child);
                    }
                } else {
                    // Remove children
                    let depth = entry.depth;
                    let mut remove_count = 0;
                    for i in (self.selected + 1)..self.entries.len() {
                        if self.entries[i].depth > depth {
                            remove_count += 1;
                        } else {
                            break;
                        }
                    }
                    for _ in 0..remove_count {
                        self.entries.remove(self.selected + 1);
                    }
                }
            }
        }
    }

    /// Get selected file path (if file, not directory)
    pub fn selected_file(&self) -> Option<&Path> {
        self.entries.get(self.selected).and_then(|e| {
            if e.is_dir {
                None
            } else {
                Some(e.path.as_path())
            }
        })
    }

    /// Move selection up
    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.auto_open_if_file();
        }
    }

    /// Move selection down
    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            self.auto_open_if_file();
        }
    }

    /// Automatically open file when selected (not directories)
    fn auto_open_if_file(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if !entry.is_dir {
                self.pending_open = Some(entry.path.clone());
            }
        }
    }

    /// Scroll to bottom (select last entry)
    fn scroll_to_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
            self.auto_open_if_file();
        }
    }
}

impl super::Panel for FileTreePanel {
    fn id(&self) -> PanelId {
        PanelId::FILE_TREE
    }

    fn name(&self) -> &str {
        "Files"
    }

    fn handle_input(&mut self, event: &Event, state: &mut AppState) -> Result<bool> {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_up();
                    Ok(true)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_down();
                    Ok(true)
                }
                KeyCode::Enter => {
                    // Enter: open file or toggle directory
                    if let Some(entry) = self.entries.get(self.selected) {
                        if entry.is_dir {
                            self.toggle_selected();
                        } else {
                            // Set pending file to open
                            self.pending_open = Some(entry.path.clone());
                        }
                    }
                    Ok(true)
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    // Right: expand directory only
                    if let Some(entry) = self.entries.get(self.selected) {
                        if entry.is_dir && !entry.expanded {
                            self.toggle_selected();
                        }
                    }
                    Ok(true)
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    // Collapse if expanded, otherwise no-op
                    if let Some(entry) = self.entries.get(self.selected) {
                        if entry.is_dir && entry.expanded {
                            self.toggle_selected();
                        }
                    }
                    Ok(true)
                }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.refresh();
                    Ok(true)
                }
                // Ctrl+C: Copy selected file/directory path to clipboard
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(entry) = self.entries.get(self.selected) {
                        let path_str = entry.path.to_string_lossy().to_string();
                        if crate::clipboard::copy(&path_str).is_ok() {
                            state.info(format!("Copied: {}", path_str));
                        }
                    }
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else if let Event::Mouse(mouse) = event {
            match mouse.kind {
                // Handle mouse click to select entry
                crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    let x = mouse.column;
                    let y = mouse.row;
                    let inner = self.inner_area.get();
                    let scroll = self.render_scroll.get();

                    // Check for scroll bar clicks first
                    let scrollbar = ScrollBar::new(scroll, inner.height as usize, self.entries.len());
                    if scrollbar.is_arrow_click(x, y, inner) {
                        // Down arrow clicked - scroll to bottom
                        self.scroll_to_bottom();
                        return Ok(true);
                    }
                    if let Some(page_up) = scrollbar.track_click(x, y, inner) {
                        // Track clicked - page up or down
                        let page_size = inner.height as usize;
                        if page_up {
                            for _ in 0..page_size {
                                self.move_up();
                            }
                        } else {
                            for _ in 0..page_size {
                                self.move_down();
                            }
                        }
                        return Ok(true);
                    }

                    // Check if click is inside the inner area (excluding scroll bar column)
                    if x >= inner.x && x < inner.x + inner.width.saturating_sub(1)
                        && y >= inner.y && y < inner.y + inner.height
                    {
                        // Calculate which entry was clicked
                        let row = (y - inner.y) as usize;
                        let clicked_idx = scroll + row;

                        if clicked_idx < self.entries.len() {
                            self.selected = clicked_idx;

                            // Handle the selection (open file or toggle directory)
                            if let Some(entry) = self.entries.get(self.selected) {
                                if entry.is_dir {
                                    self.toggle_selected();
                                } else {
                                    self.pending_open = Some(entry.path.clone());
                                }
                            }
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                // Handle mouse scroll
                crossterm::event::MouseEventKind::ScrollUp => {
                    for _ in 0..3 {
                        self.move_up();
                    }
                    Ok(true)
                }
                crossterm::event::MouseEventKind::ScrollDown => {
                    for _ in 0..3 {
                        self.move_down();
                    }
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Calculate inner area first to get visible height
        let temp_block = Block::default().borders(Borders::ALL);
        let inner = temp_block.inner(area);
        let visible_height = inner.height as usize;

        // Adjust scroll to keep selected visible
        let mut scroll = self.scroll;
        if self.selected < scroll {
            scroll = self.selected;
        } else if self.selected >= scroll + visible_height {
            scroll = self.selected - visible_height + 1;
        }

        // Store for mouse click detection
        self.inner_area.set(inner);
        self.render_scroll.set(scroll);

        // Generate scroll indicator
        let scroll_info = crate::ui::scroll::scroll_indicator(scroll, visible_height, self.entries.len());
        let title = format!(" Files{} ", scroll_info);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        // Build lines
        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(idx, entry)| {
                let indent = "  ".repeat(entry.depth);

                // Get icon and color based on file type
                let (icon, base_color) = if entry.is_dir {
                    if entry.expanded {
                        ("📂 ", Color::Yellow)
                    } else {
                        ("📁 ", Color::Yellow)
                    }
                } else {
                    get_file_icon_and_color(&entry.name)
                };

                let is_selected = idx == self.selected;

                // Build style
                let icon_style = if is_selected {
                    Style::default().bg(Color::Rgb(60, 60, 80))
                } else {
                    Style::default()
                };

                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(60, 60, 80))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(base_color)
                };

                let indent_style = if is_selected {
                    Style::default().bg(Color::Rgb(60, 60, 80))
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                Line::from(vec![
                    Span::styled(indent, indent_style),
                    Span::styled(icon, icon_style),
                    Span::styled(&entry.name, name_style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);

        // Render scroll bar
        let scrollbar = ScrollBar::new(scroll, visible_height, self.entries.len());
        scrollbar.render(frame, inner, focused);
    }
}

/// Get icon and color for a file based on extension
fn get_file_icon_and_color(name: &str) -> (&'static str, Color) {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        // Rust
        "rs" => ("🦀 ", Color::Rgb(255, 100, 50)),
        // JavaScript/TypeScript
        "js" => ("📜 ", Color::Yellow),
        "ts" => ("📘 ", Color::Cyan),
        "jsx" | "tsx" => ("⚛️  ", Color::Cyan),
        // Python
        "py" => ("🐍 ", Color::Green),
        // Web
        "html" | "htm" => ("🌐 ", Color::Rgb(255, 100, 50)),
        "css" | "scss" | "sass" => ("🎨 ", Color::Magenta),
        // Config
        "json" => ("📋 ", Color::Yellow),
        "yaml" | "yml" => ("📋 ", Color::LightRed),
        "toml" => ("⚙️  ", Color::Gray),
        "xml" => ("📄 ", Color::Rgb(255, 150, 50)),
        // Markdown/Text
        "md" | "markdown" => ("📝 ", Color::LightBlue),
        "txt" => ("📄 ", Color::White),
        // Shell
        "sh" | "bash" | "zsh" => ("💻 ", Color::Green),
        // Git
        "gitignore" => ("🚫 ", Color::Gray),
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => ("🖼️  ", Color::Magenta),
        // Lock files
        "lock" => ("🔒 ", Color::Gray),
        // Go
        "go" => ("🔵 ", Color::Cyan),
        // C/C++
        "c" | "h" => ("🔧 ", Color::Blue),
        "cpp" | "cc" | "hpp" => ("🔧 ", Color::Blue),
        // Java
        "java" => ("☕ ", Color::Rgb(255, 100, 50)),
        // Default
        _ => ("📄 ", Color::White),
    }
}
