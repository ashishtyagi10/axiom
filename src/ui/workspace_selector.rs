//! Workspace selector modal for managing and switching workspaces

use crate::state::{WorkspaceId, WorkspaceView, WorkspaceType};
use crate::ui::theme::theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::path::PathBuf;

/// Mode of the workspace selector
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectorMode {
    /// Listing workspaces
    List,
    /// Creating a new workspace
    CreateNew,
    /// Confirming deletion
    ConfirmDelete,
    /// Browsing folders to select a path
    BrowseFolders,
}

/// Entry in the folder browser
#[derive(Debug, Clone)]
pub struct FolderEntry {
    /// Display name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Whether this is a directory (always true for folder browser)
    pub is_parent: bool,
}

/// Action returned from workspace selector interactions
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceSelectorAction {
    /// Select and switch to a workspace
    Select(WorkspaceId),
    /// Create a new workspace
    Create { name: String, path: std::path::PathBuf },
    /// Delete a workspace
    Delete(WorkspaceId),
    /// Cancel/close the modal
    Cancel,
    /// No action (continue interaction)
    None,
}

/// Workspace selector modal state
pub struct WorkspaceSelectorModal {
    /// List of workspaces to display
    workspaces: Vec<WorkspaceView>,

    /// Currently selected index
    selected_index: usize,

    /// Current mode (public for external mode checks)
    pub mode: SelectorMode,

    /// New workspace name (when creating)
    new_workspace_name: String,

    /// New workspace path (when creating)
    new_workspace_path: String,

    /// Edit cursor position
    cursor_pos: usize,

    /// Which field is being edited (0 = name, 1 = path)
    edit_field: usize,

    /// Active workspace ID (for highlighting)
    active_workspace_id: Option<WorkspaceId>,

    /// Hit testing
    modal_area: Option<Rect>,
    row_areas: Vec<Rect>,

    // Folder browser state
    /// Current directory being browsed
    folder_browser_path: PathBuf,
    /// Directories in current path
    folder_entries: Vec<FolderEntry>,
    /// Selected folder index
    folder_selected: usize,
    /// Scroll offset for folder list
    folder_scroll: usize,
}

impl WorkspaceSelectorModal {
    /// Create a new workspace selector modal
    pub fn new() -> Self {
        Self {
            workspaces: Vec::new(),
            selected_index: 0,
            mode: SelectorMode::List,
            new_workspace_name: String::new(),
            new_workspace_path: String::new(),
            cursor_pos: 0,
            edit_field: 0,
            active_workspace_id: None,
            modal_area: None,
            row_areas: Vec::new(),
            folder_browser_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            folder_entries: Vec::new(),
            folder_selected: 0,
            folder_scroll: 0,
        }
    }

    /// Set the list of workspaces
    pub fn set_workspaces(&mut self, workspaces: Vec<WorkspaceView>, active_id: Option<WorkspaceId>) {
        self.workspaces = workspaces;
        self.active_workspace_id = active_id;
        self.selected_index = 0;
        self.mode = SelectorMode::List;

        // Try to select the active workspace
        if let Some(active) = active_id {
            if let Some(idx) = self.workspaces.iter().position(|w| w.id == active) {
                self.selected_index = idx;
            }
        }
    }

    /// Get the currently selected workspace
    pub fn selected_workspace(&self) -> Option<&WorkspaceView> {
        self.workspaces.get(self.selected_index)
    }

    /// Navigate up
    pub fn up(&mut self) {
        match self.mode {
            SelectorMode::List => {
                // +1 for "Create New" option at the end
                let total = self.workspaces.len() + 1;
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                } else {
                    self.selected_index = total.saturating_sub(1);
                }
            }
            SelectorMode::CreateNew => {
                // Toggle between name and path fields
                self.edit_field = if self.edit_field == 0 { 1 } else { 0 };
                self.update_cursor_for_field();
            }
            SelectorMode::ConfirmDelete => {}
            SelectorMode::BrowseFolders => {
                // +1 for "Select This Folder" option at the end
                let total = self.folder_entries.len() + 1;
                if self.folder_selected > 0 {
                    self.folder_selected -= 1;
                } else {
                    self.folder_selected = total.saturating_sub(1);
                }
            }
        }
    }

    /// Navigate down
    pub fn down(&mut self) {
        match self.mode {
            SelectorMode::List => {
                let total = self.workspaces.len() + 1;
                if self.selected_index < total.saturating_sub(1) {
                    self.selected_index += 1;
                } else {
                    self.selected_index = 0;
                }
            }
            SelectorMode::CreateNew => {
                self.edit_field = if self.edit_field == 0 { 1 } else { 0 };
                self.update_cursor_for_field();
            }
            SelectorMode::ConfirmDelete => {}
            SelectorMode::BrowseFolders => {
                let total = self.folder_entries.len() + 1;
                if self.folder_selected < total.saturating_sub(1) {
                    self.folder_selected += 1;
                } else {
                    self.folder_selected = 0;
                }
            }
        }
    }

    /// Update cursor position when switching fields
    fn update_cursor_for_field(&mut self) {
        self.cursor_pos = if self.edit_field == 0 {
            self.new_workspace_name.len()
        } else {
            self.new_workspace_path.len()
        };
    }

    /// Handle Enter key
    pub fn enter(&mut self) -> WorkspaceSelectorAction {
        match self.mode {
            SelectorMode::List => {
                if self.selected_index < self.workspaces.len() {
                    // Select workspace
                    let ws = &self.workspaces[self.selected_index];
                    WorkspaceSelectorAction::Select(ws.id)
                } else {
                    // "Create New" option
                    self.mode = SelectorMode::CreateNew;
                    self.new_workspace_name.clear();
                    self.new_workspace_path.clear();
                    self.edit_field = 0;
                    self.cursor_pos = 0;
                    WorkspaceSelectorAction::None
                }
            }
            SelectorMode::CreateNew => {
                if self.new_workspace_name.is_empty() || self.new_workspace_path.is_empty() {
                    WorkspaceSelectorAction::None
                } else {
                    let path = std::path::PathBuf::from(&self.new_workspace_path);
                    WorkspaceSelectorAction::Create {
                        name: self.new_workspace_name.clone(),
                        path,
                    }
                }
            }
            SelectorMode::ConfirmDelete => {
                if let Some(ws) = self.workspaces.get(self.selected_index) {
                    let id = ws.id;
                    self.mode = SelectorMode::List;
                    WorkspaceSelectorAction::Delete(id)
                } else {
                    self.mode = SelectorMode::List;
                    WorkspaceSelectorAction::None
                }
            }
            SelectorMode::BrowseFolders => {
                // Handled by folder_enter() instead
                WorkspaceSelectorAction::None
            }
        }
    }

    /// Handle Escape key
    pub fn escape(&mut self) -> WorkspaceSelectorAction {
        match self.mode {
            SelectorMode::List => WorkspaceSelectorAction::Cancel,
            SelectorMode::CreateNew | SelectorMode::ConfirmDelete => {
                self.mode = SelectorMode::List;
                WorkspaceSelectorAction::None
            }
            SelectorMode::BrowseFolders => {
                // Return to CreateNew mode without selecting
                self.mode = SelectorMode::CreateNew;
                WorkspaceSelectorAction::None
            }
        }
    }

    /// Handle Delete key
    pub fn delete(&mut self) {
        if self.mode == SelectorMode::List && self.selected_index < self.workspaces.len() {
            self.mode = SelectorMode::ConfirmDelete;
        }
    }

    /// Handle 'y' key (confirm delete)
    pub fn confirm(&mut self) -> WorkspaceSelectorAction {
        if self.mode == SelectorMode::ConfirmDelete {
            self.enter()
        } else {
            WorkspaceSelectorAction::None
        }
    }

    /// Handle 'n' key (cancel delete)
    pub fn deny(&mut self) {
        if self.mode == SelectorMode::ConfirmDelete {
            self.mode = SelectorMode::List;
        }
    }

    /// Insert character into current field
    pub fn insert_char(&mut self, c: char) {
        if self.mode == SelectorMode::CreateNew {
            let field = if self.edit_field == 0 {
                &mut self.new_workspace_name
            } else {
                &mut self.new_workspace_path
            };
            field.insert(self.cursor_pos, c);
            self.cursor_pos += c.len_utf8();
        }
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.mode == SelectorMode::CreateNew && self.cursor_pos > 0 {
            let field = if self.edit_field == 0 {
                &mut self.new_workspace_name
            } else {
                &mut self.new_workspace_path
            };
            self.cursor_pos -= 1;
            while self.cursor_pos > 0 && !field.is_char_boundary(self.cursor_pos) {
                self.cursor_pos -= 1;
            }
            field.remove(self.cursor_pos);
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        let len = if self.edit_field == 0 {
            self.new_workspace_name.len()
        } else {
            self.new_workspace_path.len()
        };
        if self.cursor_pos < len {
            self.cursor_pos += 1;
        }
    }

    // ========== Folder Browser Methods ==========

    /// Start folder browsing mode
    pub fn start_folder_browse(&mut self) {
        // If path field has content, try to use it as starting point
        if !self.new_workspace_path.is_empty() {
            let path = PathBuf::from(&self.new_workspace_path);
            if path.is_dir() {
                self.folder_browser_path = path;
            }
        }
        self.mode = SelectorMode::BrowseFolders;
        self.folder_selected = 0;
        self.folder_scroll = 0;
        self.refresh_folder_entries();
    }

    /// Refresh the folder entries list
    fn refresh_folder_entries(&mut self) {
        self.folder_entries.clear();

        // Add parent directory entry if not at root
        if let Some(parent) = self.folder_browser_path.parent() {
            self.folder_entries.push(FolderEntry {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_parent: true,
            });
        }

        // List subdirectories (sorted, no hidden files)
        if let Ok(entries) = std::fs::read_dir(&self.folder_browser_path) {
            let mut dirs: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .collect();
            dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

            for entry in dirs {
                self.folder_entries.push(FolderEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path(),
                    is_parent: false,
                });
            }
        }

        // Ensure selected index is valid
        let total = self.folder_entries.len() + 1; // +1 for "Select This Folder"
        if self.folder_selected >= total {
            self.folder_selected = 0;
        }
    }

    /// Handle Enter key in folder browser
    /// Returns true if a folder was selected (should return to CreateNew)
    pub fn folder_enter(&mut self) -> bool {
        // Check if "Select This Folder" is selected (last option)
        if self.folder_selected == self.folder_entries.len() {
            // Select current folder
            self.new_workspace_path = self.folder_browser_path.to_string_lossy().to_string();
            self.cursor_pos = self.new_workspace_path.len();
            self.mode = SelectorMode::CreateNew;
            self.edit_field = 1; // Stay on path field
            return true;
        }

        // Navigate into selected folder
        if let Some(entry) = self.folder_entries.get(self.folder_selected) {
            if entry.path.is_dir() {
                self.folder_browser_path = entry.path.clone();
                self.folder_selected = 0;
                self.folder_scroll = 0;
                self.refresh_folder_entries();
            }
        }
        false
    }

    /// Go to parent directory
    pub fn folder_parent(&mut self) {
        if let Some(parent) = self.folder_browser_path.parent() {
            self.folder_browser_path = parent.to_path_buf();
            self.folder_selected = 0;
            self.folder_scroll = 0;
            self.refresh_folder_entries();
        }
    }

    /// Cancel folder browsing and return to CreateNew
    pub fn cancel_browse(&mut self) {
        self.mode = SelectorMode::CreateNew;
    }

    /// Check if point is inside modal
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.modal_area
            .map(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
            .unwrap_or(false)
    }

    /// Handle mouse click
    pub fn handle_click(&mut self, x: u16, y: u16) -> WorkspaceSelectorAction {
        if self.mode != SelectorMode::List {
            return WorkspaceSelectorAction::None;
        }

        for (idx, area) in self.row_areas.iter().enumerate() {
            if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
                self.selected_index = idx;
                return self.enter();
            }
        }
        WorkspaceSelectorAction::None
    }

    /// Get type icon for workspace
    fn type_icon(ws_type: &WorkspaceType) -> &'static str {
        match ws_type {
            WorkspaceType::Local => "📁",
            WorkspaceType::Remote { .. } => "🌐",
            WorkspaceType::Container { .. } => "🐳",
        }
    }

    /// Render the modal
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate modal size
        let modal_width = (area.width as f32 * 0.7).max(50.0).min(80.0) as u16;
        let modal_height = (area.height as f32 * 0.6).max(15.0).min(25.0) as u16;

        let x = (area.width.saturating_sub(modal_width)) / 2;
        let y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(x, y, modal_width, modal_height);
        self.modal_area = Some(modal_area);

        // Clear background
        frame.render_widget(Clear, modal_area);

        let t = theme();
        let title = match self.mode {
            SelectorMode::List => " Workspaces ",
            SelectorMode::CreateNew => " Create Workspace ",
            SelectorMode::ConfirmDelete => " Delete Workspace? ",
            SelectorMode::BrowseFolders => " Select Folder ",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border_focused))
            .style(Style::default().bg(t.bg_modal));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        match self.mode {
            SelectorMode::List => self.render_list(frame, inner),
            SelectorMode::CreateNew => self.render_create_form(frame, inner),
            SelectorMode::ConfirmDelete => self.render_delete_confirm(frame, inner),
            SelectorMode::BrowseFolders => self.render_folder_browser(frame, inner),
        }
    }

    /// Render workspace list
    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        self.row_areas.clear();

        let row_height = 2u16;
        let mut current_y = area.y;

        // Render workspaces
        for (idx, ws) in self.workspaces.iter().enumerate() {
            if current_y + row_height > area.y + area.height - 2 {
                break;
            }

            let row_area = Rect::new(area.x, current_y, area.width, row_height);
            self.row_areas.push(row_area);

            let is_selected = idx == self.selected_index;
            let is_active = Some(ws.id) == self.active_workspace_id;

            self.render_workspace_row(frame, row_area, ws, is_selected, is_active);
            current_y += row_height;
        }

        // "Create New" option
        if current_y + row_height <= area.y + area.height {
            let row_area = Rect::new(area.x, current_y, area.width, row_height);
            self.row_areas.push(row_area);

            let is_selected = self.selected_index == self.workspaces.len();
            self.render_create_new_option(frame, row_area, is_selected);
        }

        // Help text at bottom
        let t = theme();
        let help = Line::from(vec![
            Span::styled("Enter", Style::default().fg(t.accent_primary)),
            Span::raw(": select  "),
            Span::styled("Del", Style::default().fg(t.status_error)),
            Span::raw(": delete  "),
            Span::styled("Esc", Style::default().fg(t.text_secondary)),
            Span::raw(": close"),
        ]);
        let help_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        frame.render_widget(Paragraph::new(help).alignment(Alignment::Center), help_area);
    }

    /// Render a single workspace row
    fn render_workspace_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        ws: &WorkspaceView,
        selected: bool,
        active: bool,
    ) {
        let t = theme();
        let bg_color = if selected {
            t.bg_selection
        } else {
            t.bg_modal
        };

        let icon = Self::type_icon(&ws.workspace_type);
        let active_marker = if active { " *" } else { "  " };
        let exists_marker = if ws.exists { "" } else { " (missing)" };

        let name_style = if selected {
            Style::default().fg(t.text_primary).add_modifier(Modifier::BOLD)
        } else if !ws.exists {
            Style::default().fg(t.text_muted)
        } else {
            Style::default().fg(t.text_secondary)
        };

        let path_style = Style::default().fg(t.text_muted);
        let active_style = Style::default().fg(t.status_success);

        let line1 = Line::from(vec![
            Span::raw(format!("  {} ", icon)),
            Span::styled(&ws.name, name_style),
            Span::styled(active_marker, active_style),
            Span::styled(exists_marker, Style::default().fg(t.status_error)),
        ]);

        let path_display = ws.path.to_string_lossy();
        let path_truncated: String = if path_display.len() > area.width as usize - 6 {
            format!("...{}", &path_display[path_display.len().saturating_sub(area.width as usize - 9)..])
        } else {
            path_display.to_string()
        };

        let line2 = Line::from(vec![
            Span::raw("     "),
            Span::styled(path_truncated, path_style),
        ]);

        let paragraph = Paragraph::new(vec![line1, line2])
            .style(Style::default().bg(bg_color));
        frame.render_widget(paragraph, area);
    }

    /// Render "Create New" option
    fn render_create_new_option(&self, frame: &mut Frame, area: Rect, selected: bool) {
        let t = theme();
        let style = if selected {
            Style::default()
                .fg(t.accent_primary)
                .bg(t.bg_selection)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text_secondary).bg(t.bg_modal)
        };

        let line = Line::from(vec![
            Span::styled("  + Create New Workspace", style),
        ]);

        frame.render_widget(Paragraph::new(line).style(style), area);
    }

    /// Render create workspace form
    fn render_create_form(&mut self, frame: &mut Frame, area: Rect) {
        let t = theme();
        let name_style = if self.edit_field == 0 {
            Style::default().fg(t.accent_highlight).bg(t.bg_selection)
        } else {
            Style::default().fg(t.text_secondary)
        };

        let path_style = if self.edit_field == 1 {
            Style::default().fg(t.accent_highlight).bg(t.bg_selection)
        } else {
            Style::default().fg(t.text_secondary)
        };

        // Build path field display with browse hint
        let path_display: String = self.new_workspace_path.chars().take(30).collect();
        let path_field = if self.edit_field == 1 && self.new_workspace_path.is_empty() {
            format!("[{:<30}]", "Ctrl+B to browse...")
        } else {
            format!("[{:<30}]", path_display)
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(t.text_primary)),
                Span::styled(
                    format!("[{:<30}]", self.new_workspace_name.chars().take(30).collect::<String>()),
                    name_style,
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(t.text_primary)),
                Span::styled(path_field, path_style),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(t.accent_primary)),
                Span::raw(": create  "),
                Span::styled("Ctrl+B", Style::default().fg(t.status_success)),
                Span::raw(": browse  "),
                Span::styled("Tab", Style::default().fg(t.text_secondary)),
                Span::raw(": switch  "),
                Span::styled("Esc", Style::default().fg(t.text_secondary)),
                Span::raw(": back"),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines), area);

        // Show cursor
        let cursor_x = area.x + 7 + self.cursor_pos as u16;
        let cursor_y = if self.edit_field == 0 {
            area.y
        } else {
            area.y + 2
        };
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    /// Render delete confirmation
    fn render_delete_confirm(&self, frame: &mut Frame, area: Rect) {
        let t = theme();
        let ws_name = self.workspaces
            .get(self.selected_index)
            .map(|w| w.name.as_str())
            .unwrap_or("Unknown");

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Delete workspace: ", Style::default().fg(t.text_primary)),
                Span::styled(ws_name, Style::default().fg(t.accent_highlight).add_modifier(Modifier::BOLD)),
                Span::raw("?"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "(This will not delete any files, only the workspace entry)",
                Style::default().fg(t.text_muted),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("y", Style::default().fg(t.status_success)),
                Span::raw(": yes  "),
                Span::styled("n", Style::default().fg(t.status_error)),
                Span::raw(": no  "),
                Span::styled("Esc", Style::default().fg(t.text_secondary)),
                Span::raw(": cancel"),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            area,
        );
    }

    /// Render folder browser
    fn render_folder_browser(&self, frame: &mut Frame, area: Rect) {
        let t = theme();

        // Current path display at top
        let path_display = self.folder_browser_path.to_string_lossy();
        let path_truncated: String = if path_display.len() > area.width as usize - 4 {
            format!("...{}", &path_display[path_display.len().saturating_sub(area.width as usize - 7)..])
        } else {
            path_display.to_string()
        };

        let path_line = Line::from(vec![
            Span::styled(&path_truncated, Style::default().fg(t.accent_primary)),
        ]);
        let path_area = Rect::new(area.x, area.y, area.width, 1);
        frame.render_widget(Paragraph::new(path_line), path_area);

        // Folder list
        let list_start_y = area.y + 2;
        let list_height = area.height.saturating_sub(4); // Leave room for path, separator, help

        let mut current_y = list_start_y;

        // Calculate visible range with scroll
        let visible_count = list_height as usize;
        let total_items = self.folder_entries.len() + 1; // +1 for "Select This Folder"

        // Render folder entries
        for (idx, entry) in self.folder_entries.iter().enumerate() {
            if current_y >= list_start_y + list_height {
                break;
            }
            if idx < self.folder_scroll {
                continue;
            }

            let is_selected = idx == self.folder_selected;
            let bg_color = if is_selected {
                t.bg_selection
            } else {
                t.bg_modal
            };

            let icon = if entry.is_parent { " .." } else { " 📁" };
            let name_style = if is_selected {
                Style::default().fg(t.text_primary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.text_secondary)
            };

            let display_name = if entry.is_parent {
                "(parent directory)".to_string()
            } else {
                format!("{}/", entry.name)
            };

            let line = Line::from(vec![
                Span::raw(icon),
                Span::raw(" "),
                Span::styled(display_name, name_style),
            ]);

            let row_area = Rect::new(area.x, current_y, area.width, 1);
            frame.render_widget(Paragraph::new(line).style(Style::default().bg(bg_color)), row_area);
            current_y += 1;
        }

        // "Select This Folder" option
        if current_y < list_start_y + list_height {
            let is_selected = self.folder_selected == self.folder_entries.len();
            let style = if is_selected {
                Style::default()
                    .fg(t.status_success)
                    .bg(t.bg_selection)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.status_success).bg(t.bg_modal)
            };

            let line = Line::from(vec![
                Span::styled("  [ Select This Folder ]", style),
            ]);

            let row_area = Rect::new(area.x, current_y, area.width, 1);
            frame.render_widget(Paragraph::new(line).style(style), row_area);
        }

        // Help text at bottom
        let help = Line::from(vec![
            Span::styled("Enter", Style::default().fg(t.accent_primary)),
            Span::raw(": open/select  "),
            Span::styled("Backspace", Style::default().fg(t.text_secondary)),
            Span::raw(": parent  "),
            Span::styled("Esc", Style::default().fg(t.text_secondary)),
            Span::raw(": cancel"),
        ]);
        let help_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        frame.render_widget(Paragraph::new(help).alignment(Alignment::Center), help_area);
    }
}

impl Default for WorkspaceSelectorModal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_selector_new() {
        let selector = WorkspaceSelectorModal::new();
        assert!(selector.workspaces.is_empty());
        assert_eq!(selector.mode, SelectorMode::List);
    }

    #[test]
    fn test_escape_cancels() {
        let mut selector = WorkspaceSelectorModal::new();
        let action = selector.escape();
        assert_eq!(action, WorkspaceSelectorAction::Cancel);
    }

    #[test]
    fn test_escape_from_create_returns_to_list() {
        let mut selector = WorkspaceSelectorModal::new();
        selector.mode = SelectorMode::CreateNew;
        let action = selector.escape();
        assert_eq!(action, WorkspaceSelectorAction::None);
        assert_eq!(selector.mode, SelectorMode::List);
    }
}
