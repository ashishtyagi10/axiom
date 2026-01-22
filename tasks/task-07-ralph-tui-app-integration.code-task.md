---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph TUI App Integration

## Description
Integrate the Ralph Loop panel into the TUI application. This includes registering the panel, adjusting layout to accommodate it, handling Ralph-specific notifications, and wiring up the panel lifecycle.

## Background
TuiApp manages the panel registry, layout, and event loop. The Ralph panel needs to be integrated into this system, potentially as an optional/toggleable panel that can be shown when Ralph Loop is active.

## Technical Requirements
1. Register RalphPanel in panel registry (`crates/axiom-tui/src/app.rs`)
2. Layout adjustment options:
   - Option A: Replace Agents panel when Ralph is active
   - Option B: Add as fifth panel with layout recalculation
   - Option C: Toggleable overlay/modal panel
3. Add Ralph panel to focus ring for Tab cycling
4. Process Ralph notifications in event loop:
   - Route RalphIterationStarted, RalphIterationComplete, etc. to Ralph panel
   - Update panel state from notifications
5. Handle panel visibility toggle (show when loop starts, option to hide)
6. Add status indicator in header/status bar when Ralph is running
7. Ensure proper cleanup when Ralph panel is hidden/removed

## Dependencies
- Task 6: Ralph TUI Panel
- Existing TuiApp and panel registry
- Existing layout system in axiom-tui

## Implementation Approach
1. Import and instantiate RalphPanel in TuiApp
2. Add to panel registry with appropriate panel ID
3. Modify layout calculation to include Ralph panel (choose layout strategy)
4. Add Ralph panel to FocusState ring
5. Add notification routing in event loop for Ralph notifications
6. Implement panel toggle command/keybinding
7. Add Ralph status to status bar
8. Test panel integration with mock notifications

## Acceptance Criteria

1. **Panel Registration**
   - Given TuiApp initialization
   - When app starts
   - Then RalphPanel is registered and available

2. **Layout Integration**
   - Given Ralph panel is visible
   - When layout is calculated
   - Then Ralph panel has appropriate space allocation

3. **Focus Cycling**
   - Given Ralph panel in registry
   - When Tab is pressed
   - Then focus cycles through including Ralph panel

4. **Notification Routing**
   - Given Ralph notifications from backend
   - When event loop processes them
   - Then notifications are forwarded to Ralph panel

5. **Panel Toggle**
   - Given Ralph panel visibility toggle
   - When toggle is activated
   - Then panel shows/hides appropriately

6. **Status Bar Indicator**
   - Given active Ralph Loop
   - When status bar renders
   - Then Ralph running indicator is visible

7. **Auto-show on Start**
   - Given StartRalphLoop command sent
   - When loop begins
   - Then Ralph panel becomes visible (if configured)

8. **Cleanup on Exit**
   - Given Ralph panel is removed/hidden
   - When visibility changes
   - Then resources are cleaned up properly

## Metadata
- **Complexity**: Medium
- **Labels**: TUI, Integration, Layout, Ralph Loop
- **Required Skills**: Rust, TUI architecture, event handling, layout management
