//! File tree panel for directory navigation

use crate::core::Result;
use crate::events::Event;
use crate::state::{AppState, PanelId};
use crate::ui::theme::theme;
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

    /// Set a new root directory for the file tree
    ///
    /// This is used when switching workspaces to update the file tree
    /// to show the new workspace's directory structure.
    pub fn set_root(&mut self, new_root: &Path) {
        self.root = new_root.to_path_buf();
        self.entries.clear();
        self.selected = 0;
        self.scroll = 0;
        self.pending_open = None;
        self.refresh();
    }

    /// Get the current root path
    pub fn root(&self) -> &Path {
        &self.root
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

    fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let t = theme();
        let border_style = if focused {
            Style::default().fg(t.border_focused)
        } else {
            Style::default().fg(t.border_unfocused)
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
                        ("📂 ", t.file_tree_directory)
                    } else {
                        ("📁 ", t.file_tree_directory)
                    }
                } else {
                    get_file_icon_and_color(&entry.name, &t)
                };

                let is_selected = idx == self.selected;

                // Build style
                let icon_style = if is_selected {
                    Style::default().bg(t.bg_selection)
                } else {
                    Style::default()
                };

                let name_style = if is_selected {
                    Style::default()
                        .fg(t.text_primary)
                        .bg(t.bg_selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(base_color)
                };

                let indent_style = if is_selected {
                    Style::default().bg(t.bg_selection)
                } else {
                    Style::default().fg(t.text_muted)
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
fn get_file_icon_and_color(name: &str, t: &crate::ui::theme::Theme) -> (&'static str, Color) {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        // Rust
        "rs" => ("🦀 ", t.status_error),
        // JavaScript/TypeScript
        "js" => ("📜 ", t.accent_highlight),
        "ts" => ("📘 ", t.accent_primary),
        "jsx" | "tsx" => ("⚛️  ", t.accent_primary),
        // Python
        "py" => ("🐍 ", t.status_success),
        // Web
        "html" | "htm" => ("🌐 ", t.status_error),
        "css" | "scss" | "sass" => ("🎨 ", t.accent_secondary),
        // Config
        "json" => ("📋 ", t.accent_highlight),
        "yaml" | "yml" => ("📋 ", t.status_error),
        "toml" => ("⚙️  ", t.text_secondary),
        "xml" => ("📄 ", t.accent_highlight),
        // Markdown/Text
        "md" | "markdown" => ("📝 ", t.status_info),
        "txt" => ("📄 ", t.text_primary),
        // Shell
        "sh" | "bash" | "zsh" => ("💻 ", t.status_success),
        // Git
        "gitignore" => ("🚫 ", t.text_secondary),
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => ("🖼️  ", t.accent_secondary),
        // Lock files
        "lock" => ("🔒 ", t.text_secondary),
        // Go
        "go" => ("🔵 ", t.accent_primary),
        // C/C++
        "c" | "h" => ("🔧 ", t.status_info),
        "cpp" | "cc" | "hpp" => ("🔧 ", t.status_info),
        // Java
        "java" => ("☕ ", t.status_error),
        // Default
        _ => ("📄 ", t.file_tree_file),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_dir() -> (std::path::PathBuf, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = temp_dir.path().to_path_buf();
        (path, temp_dir)
    }

    #[test]
    fn test_file_tree_new() {
        let (path, _guard) = create_temp_dir();
        let panel = FileTreePanel::new(&path);
        assert_eq!(panel.root, path);
        assert_eq!(panel.selected, 0);
        assert_eq!(panel.scroll, 0);
        assert!(panel.pending_open.is_none());
    }

    #[test]
    fn test_file_tree_refresh() {
        let (path, _guard) = create_temp_dir();

        // Create some files
        fs::write(path.join("test.txt"), "content").unwrap();
        fs::create_dir(path.join("subdir")).unwrap();

        let mut panel = FileTreePanel::new(&path);
        panel.refresh();

        // Should have entries (directory sorted first)
        assert!(!panel.entries.is_empty());
    }

    #[test]
    fn test_file_entry_structure() {
        let entry = FileEntry {
            path: PathBuf::from("/test/file.rs"),
            name: "file.rs".to_string(),
            is_dir: false,
            expanded: false,
            depth: 0,
        };

        assert_eq!(entry.name, "file.rs");
        assert!(!entry.is_dir);
        assert!(!entry.expanded);
        assert_eq!(entry.depth, 0);
    }

    #[test]
    fn test_file_tree_navigation() {
        let (path, _guard) = create_temp_dir();

        // Create test files
        fs::write(path.join("a.txt"), "").unwrap();
        fs::write(path.join("b.txt"), "").unwrap();

        let mut panel = FileTreePanel::new(&path);

        assert_eq!(panel.selected, 0);
        panel.move_down();
        assert_eq!(panel.selected, 1);
        panel.move_up();
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn test_file_tree_navigation_boundaries() {
        let (path, _guard) = create_temp_dir();

        fs::write(path.join("only.txt"), "").unwrap();

        let mut panel = FileTreePanel::new(&path);

        // Can't go above first entry
        panel.move_up();
        assert_eq!(panel.selected, 0);

        // Can't go below last entry
        panel.move_down();
        panel.move_down();
        assert!(panel.selected < panel.entries.len() || panel.entries.is_empty());
    }

    #[test]
    fn test_file_tree_select_file() {
        let (path, _guard) = create_temp_dir();

        fs::write(path.join("test.txt"), "content").unwrap();

        let panel = FileTreePanel::new(&path);

        if !panel.entries.is_empty() {
            let selected = panel.selected_file();
            assert!(selected.is_some());
        }
    }

    #[test]
    fn test_file_tree_select_directory() {
        let (path, _guard) = create_temp_dir();

        fs::create_dir(path.join("subdir")).unwrap();

        let mut panel = FileTreePanel::new(&path);

        // Find the directory entry
        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.is_dir {
                panel.selected = i;
                break;
            }
        }

        // selected_file() should return None for directory
        if let Some(entry) = panel.entries.get(panel.selected) {
            if entry.is_dir {
                assert!(panel.selected_file().is_none());
            }
        }
    }

    #[test]
    fn test_take_pending_open() {
        let (path, _guard) = create_temp_dir();

        fs::write(path.join("test.txt"), "").unwrap();

        let mut panel = FileTreePanel::new(&path);
        panel.pending_open = Some(PathBuf::from("/test/file.rs"));

        let taken = panel.take_pending_open();
        assert!(taken.is_some());
        assert!(panel.pending_open.is_none());
    }

    #[test]
    fn test_file_tree_expand_collapse() {
        let (path, _guard) = create_temp_dir();

        // Create a directory with a file inside
        let subdir = path.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("nested.txt"), "").unwrap();

        let mut panel = FileTreePanel::new(&path);

        // Find directory entry
        let dir_idx = panel.entries.iter().position(|e| e.is_dir);

        if let Some(idx) = dir_idx {
            panel.selected = idx;
            assert!(!panel.entries[idx].expanded);

            // Toggle to expand
            panel.toggle_selected();
            assert!(panel.entries[idx].expanded);

            // Toggle to collapse
            panel.toggle_selected();
            assert!(!panel.entries[idx].expanded);
        }
    }

    #[test]
    fn test_file_tree_hidden_files_filtered() {
        let (path, _guard) = create_temp_dir();

        // Create a hidden file
        fs::write(path.join(".hidden"), "").unwrap();
        fs::write(path.join("visible.txt"), "").unwrap();

        let panel = FileTreePanel::new(&path);

        // Hidden files should be filtered out
        let has_hidden = panel.entries.iter().any(|e| e.name.starts_with('.'));
        assert!(!has_hidden);
    }

    #[test]
    fn test_file_tree_sorting() {
        let (path, _guard) = create_temp_dir();

        // Create files and directories
        fs::write(path.join("z_file.txt"), "").unwrap();
        fs::write(path.join("a_file.txt"), "").unwrap();
        fs::create_dir(path.join("z_dir")).unwrap();
        fs::create_dir(path.join("a_dir")).unwrap();

        let panel = FileTreePanel::new(&path);

        // Directories should come before files
        let first_file_idx = panel.entries.iter().position(|e| !e.is_dir);
        let last_dir_idx = panel.entries.iter().rposition(|e| e.is_dir);

        if let (Some(file_idx), Some(dir_idx)) = (first_file_idx, last_dir_idx) {
            assert!(dir_idx < file_idx);
        }
    }

    #[test]
    fn test_file_tree_scroll_to_bottom() {
        let (path, _guard) = create_temp_dir();

        // Create multiple files
        for i in 0..10 {
            fs::write(path.join(format!("file{}.txt", i)), "").unwrap();
        }

        let mut panel = FileTreePanel::new(&path);
        panel.scroll_to_bottom();

        assert_eq!(panel.selected, panel.entries.len().saturating_sub(1));
    }

    #[test]
    fn test_get_file_icon_and_color_rust() {
        let t = crate::ui::theme::Theme::dark();
        let (icon, _) = get_file_icon_and_color("main.rs", &t);
        assert_eq!(icon, "🦀 ");
    }

    #[test]
    fn test_get_file_icon_and_color_javascript() {
        let t = crate::ui::theme::Theme::dark();
        let (icon, _) = get_file_icon_and_color("app.js", &t);
        assert_eq!(icon, "📜 ");

        let (icon, _) = get_file_icon_and_color("app.ts", &t);
        assert_eq!(icon, "📘 ");
    }

    #[test]
    fn test_get_file_icon_and_color_python() {
        let t = crate::ui::theme::Theme::dark();
        let (icon, _) = get_file_icon_and_color("script.py", &t);
        assert_eq!(icon, "🐍 ");
    }

    #[test]
    fn test_get_file_icon_and_color_unknown() {
        let t = crate::ui::theme::Theme::dark();
        let (icon, _) = get_file_icon_and_color("file.xyz", &t);
        assert_eq!(icon, "📄 ");
    }

    #[test]
    fn test_get_file_icon_case_insensitive() {
        let t = crate::ui::theme::Theme::dark();
        let (icon1, _) = get_file_icon_and_color("file.RS", &t);
        let (icon2, _) = get_file_icon_and_color("file.rs", &t);
        assert_eq!(icon1, icon2);
    }
}
