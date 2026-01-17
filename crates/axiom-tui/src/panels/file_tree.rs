//! File tree panel for directory navigation

use super::Panel;
use crate::events::TuiEvent;
use crate::state::{AppState, PanelId};
use axiom_core::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::{Path, PathBuf};

/// File or directory entry
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    expanded: bool,
    depth: usize,
}

/// File tree panel
pub struct FileTreePanel {
    root: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    scroll: usize,
    pub pending_open: Option<PathBuf>,
    inner_area: Rect,
}

impl FileTreePanel {
    pub fn new(root: &Path) -> Self {
        let mut panel = Self {
            root: root.to_path_buf(),
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
            pending_open: None,
            inner_area: Rect::default(),
        };
        panel.refresh();
        panel
    }

    pub fn take_pending_open(&mut self) -> Option<PathBuf> {
        self.pending_open.take()
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.load_dir(&self.root.clone(), 0);
    }

    fn load_dir(&mut self, path: &Path, depth: usize) {
        let mut entries: Vec<_> = match std::fs::read_dir(path) {
            Ok(dir) => dir
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .collect(),
            Err(_) => return,
        };

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
            self.entries.push(FileEntry {
                path: entry.path(),
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir,
                expanded: false,
                depth,
            });
        }
    }

    fn toggle_selected(&mut self) {
        if let Some(entry) = self.entries.get_mut(self.selected) {
            if entry.is_dir {
                entry.expanded = !entry.expanded;
                if entry.expanded {
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
                    for (i, child) in children.into_iter().enumerate() {
                        self.entries.insert(insert_at + i, child);
                    }
                } else {
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

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.auto_open_file();
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            self.auto_open_file();
        }
    }

    fn auto_open_file(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if !entry.is_dir {
                self.pending_open = Some(entry.path.clone());
            }
        }
    }

    fn ensure_visible(&mut self, height: usize) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + height {
            self.scroll = self.selected.saturating_sub(height - 1);
        }
    }
}

impl Panel for FileTreePanel {
    fn id(&self) -> PanelId {
        PanelId::FILE_TREE
    }

    fn name(&self) -> &str {
        "Files"
    }

    fn handle_input(&mut self, event: &TuiEvent, _state: &mut AppState) -> Result<bool> {
        match event {
            TuiEvent::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_up();
                    Ok(true)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_down();
                    Ok(true)
                }
                KeyCode::Enter | KeyCode::Right => {
                    if let Some(entry) = self.entries.get(self.selected) {
                        if entry.is_dir {
                            self.toggle_selected();
                        } else {
                            self.pending_open = Some(entry.path.clone());
                        }
                    }
                    Ok(true)
                }
                KeyCode::Left => {
                    if let Some(entry) = self.entries.get(self.selected) {
                        if entry.is_dir && entry.expanded {
                            self.toggle_selected();
                        }
                    }
                    Ok(true)
                }
                _ => Ok(false),
            },
            TuiEvent::Mouse(mouse) => {
                let inner = self.inner_area;
                if mouse.column >= inner.x
                    && mouse.column < inner.x + inner.width
                    && mouse.row >= inner.y
                    && mouse.row < inner.y + inner.height
                {
                    match mouse.kind {
                        MouseEventKind::Down(_) => {
                            let idx = self.scroll + (mouse.row - inner.y) as usize;
                            if idx < self.entries.len() {
                                self.selected = idx;
                                self.auto_open_file();
                            }
                            Ok(true)
                        }
                        MouseEventKind::ScrollUp => {
                            self.move_up();
                            Ok(true)
                        }
                        MouseEventKind::ScrollDown => {
                            self.move_down();
                            Ok(true)
                        }
                        _ => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let block = Block::default()
            .title(" Files ")
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let inner = block.inner(area);
        self.inner_area = inner;

        let visible = inner.height as usize;
        self.ensure_visible(visible);

        let lines: Vec<Line> = self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible)
            .map(|(idx, entry)| {
                let selected = idx == self.selected;
                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir {
                    if entry.expanded { "v " } else { "> " }
                } else {
                    "  "
                };

                let style = if selected {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else if entry.is_dir {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };

                Line::from(Span::styled(format!("{}{}{}", indent, icon, entry.name), style))
            })
            .collect();

        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(lines), inner);
    }
}
