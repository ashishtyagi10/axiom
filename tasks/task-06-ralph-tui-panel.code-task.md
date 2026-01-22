---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph TUI Panel

## Description
Create a dedicated TUI panel for Ralph Loop visualization. The panel displays iteration progress, task description, iteration history with status, and provides visual feedback for the autonomous development cycle.

## Background
Axiom TUI uses a panel-based design where all UI panels implement the Panel trait. The Ralph panel provides visibility into the autonomous loop execution, showing what iteration is running, what's been accomplished, and the current status.

## Technical Requirements
1. Create `crates/axiom-tui/src/panels/ralph.rs` implementing the Panel trait
2. Display components:
   - Current iteration number and max iterations (e.g., "Iteration 3/20")
   - Task description (scrollable if long)
   - Current status indicator (Idle, Running, Complete, Error)
   - Real-time iteration progress (spinner/activity indicator when running)
   - Iteration history list with status icons (✓ success, ✗ failed, ⏱ timeout)
   - Last feedback/summary text
   - Elapsed time for current iteration
3. Implement Panel trait methods:
   - `render()`: Draw panel content with ratatui
   - `handle_input()`: Handle panel-specific keys (scroll history, etc.)
   - `update()`: Process Ralph notifications and update state
4. Support scrolling through iteration history
5. Use appropriate colors/styling for different states

## Dependencies
- Task 3: Ralph Notifications (for update handling)
- Existing Panel trait in axiom-tui
- ratatui for rendering
- Existing UI styling patterns

## Implementation Approach
1. Define RalphPanel struct with local state (iteration history, scroll position, etc.)
2. Implement Panel trait with render, handle_input, update methods
3. Create layout with header (status), body (history), footer (summary)
4. Add notification handler to update panel state
5. Implement scroll navigation for history
6. Add visual styling matching existing panels
7. Write rendering tests

## Acceptance Criteria

1. **Panel Rendering**
   - Given Ralph panel is focused
   - When render() is called
   - Then panel displays all required components correctly

2. **Iteration Display**
   - Given active loop at iteration 5 of 20
   - When panel renders
   - Then "Iteration 5/20" is clearly visible

3. **Status Indicator**
   - Given different Ralph states (Idle, Running, Complete, Error)
   - When panel renders
   - Then appropriate status with color coding is shown

4. **Iteration History**
   - Given multiple completed iterations
   - When panel renders
   - Then history shows each iteration with status icon

5. **History Scrolling**
   - Given more iterations than visible area
   - When user scrolls (j/k or arrows)
   - Then history scrolls appropriately

6. **Real-time Updates**
   - Given notifications arriving during execution
   - When update() is called with notification
   - Then panel state updates and re-renders

7. **Task Display**
   - Given a long task description
   - When panel renders
   - Then task is shown with truncation or scrolling

8. **Elapsed Time**
   - Given a running iteration
   - When panel renders
   - Then elapsed time for current iteration is shown

## Metadata
- **Complexity**: Medium
- **Labels**: TUI, Panel, UI, Ralph Loop
- **Required Skills**: Rust, ratatui, TUI design, Panel trait
