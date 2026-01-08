//! Focus management for panels
//!
//! Tracks which panel has focus and provides navigation.

/// Unique identifier for a panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub usize);

impl PanelId {
    /// File tree panel (left sidebar)
    pub const FILE_TREE: PanelId = PanelId(0);

    /// Output panel (center-top, shows file content or agent output)
    pub const OUTPUT: PanelId = PanelId(1);

    /// Input panel (center-bottom, unified command input)
    pub const INPUT: PanelId = PanelId(2);

    /// Agents panel (right sidebar, shows spawned agents)
    pub const AGENTS: PanelId = PanelId(3);

    // Legacy aliases for compatibility during transition
    #[deprecated(note = "Use OUTPUT instead")]
    pub const EDITOR: PanelId = PanelId(1);

    #[deprecated(note = "Use INPUT instead")]
    pub const TERMINAL: PanelId = PanelId(2);

    #[deprecated(note = "Use AGENTS instead")]
    pub const CHAT: PanelId = PanelId(3);
}

/// Focus state management
pub struct FocusState {
    /// Currently focused panel
    current: PanelId,

    /// Focus ring (panels in tab order)
    ring: Vec<PanelId>,

    /// Focus history for back navigation
    history: Vec<PanelId>,

    /// Maximum history size
    max_history: usize,
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusState {
    /// Create new focus state with default panel ring
    pub fn new() -> Self {
        Self {
            // Start at INPUT for immediate typing
            current: PanelId::INPUT,
            ring: vec![
                PanelId::FILE_TREE,
                PanelId::OUTPUT,
                PanelId::INPUT,
                PanelId::AGENTS,
            ],
            history: Vec::new(),
            max_history: 10,
        }
    }

    /// Get currently focused panel
    pub fn current(&self) -> PanelId {
        self.current
    }

    /// Check if a panel is focused
    pub fn is_focused(&self, id: PanelId) -> bool {
        self.current == id
    }

    /// Focus a specific panel
    pub fn focus(&mut self, id: PanelId) {
        if self.current != id {
            // Add to history
            self.history.push(self.current);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
            self.current = id;
        }
    }

    /// Cycle to next panel in ring
    pub fn next(&mut self) {
        if let Some(idx) = self.ring.iter().position(|&id| id == self.current) {
            let next_idx = (idx + 1) % self.ring.len();
            self.focus(self.ring[next_idx]);
        }
    }

    /// Cycle to previous panel in ring
    pub fn prev(&mut self) {
        if let Some(idx) = self.ring.iter().position(|&id| id == self.current) {
            let prev_idx = if idx == 0 {
                self.ring.len() - 1
            } else {
                idx - 1
            };
            self.focus(self.ring[prev_idx]);
        }
    }

    /// Go back to previously focused panel
    pub fn back(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.current = prev;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_focus() {
        let focus = FocusState::new();
        assert_eq!(focus.current(), PanelId::INPUT);
    }

    #[test]
    fn test_focus_change() {
        let mut focus = FocusState::new();

        focus.focus(PanelId::OUTPUT);
        assert_eq!(focus.current(), PanelId::OUTPUT);
        assert!(focus.is_focused(PanelId::OUTPUT));
        assert!(!focus.is_focused(PanelId::INPUT));
    }

    #[test]
    fn test_cycle_next() {
        let mut focus = FocusState::new();
        focus.focus(PanelId::FILE_TREE);

        focus.next();
        assert_eq!(focus.current(), PanelId::OUTPUT);

        focus.next();
        assert_eq!(focus.current(), PanelId::INPUT);
    }

    #[test]
    fn test_back_navigation() {
        let mut focus = FocusState::new();

        focus.focus(PanelId::OUTPUT);
        focus.focus(PanelId::AGENTS);

        focus.back();
        assert_eq!(focus.current(), PanelId::OUTPUT);
    }
}
