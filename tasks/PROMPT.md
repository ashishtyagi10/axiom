# Ralph Loop Implementation

## Objective

Implement the Ralph Loop feature for Axiom - an autonomous iterative loop mechanism that continuously works on tasks until completion. The feature spans backend (axiom-core), TUI (axiom-tui), and Web (Next.js) layers.

## Code Tasks

Execute these tasks **in sequence** (dependencies flow from earlier to later tasks):

### Backend Core (Tasks 1-5)
1. `task-01-ralph-state-management.code-task.md` - RalphState, RalphConfig, file persistence
2. `task-02-ralph-commands.code-task.md` - Command enum extensions
3. `task-03-ralph-notifications.code-task.md` - Notification enum extensions
4. `task-04-ralph-executor.code-task.md` - Core loop logic and iteration orchestration
5. `task-05-ralph-service-integration.code-task.md` - AxiomService integration

### TUI (Tasks 6-8)
6. `task-06-ralph-tui-panel.code-task.md` - Ralph visualization panel
7. `task-07-ralph-tui-app-integration.code-task.md` - TuiApp panel registration
8. `task-08-ralph-tui-input-commands.code-task.md` - #ralph command parsing

### Web (Tasks 9-12)
9. `task-09-ralph-web-component.code-task.md` - React component
10. `task-10-ralph-web-hook.code-task.md` - useRalphLoop hook
11. `task-11-ralph-web-actions.code-task.md` - Server actions
12. `task-12-ralph-web-workspace-integration.code-task.md` - WorkspaceStudio integration

## Execution Order

**Critical path:** Tasks 1 → 2 → 3 → 4 → 5 must be sequential (each depends on prior).

**Parallel tracks after Task 5:**
- TUI track: 6 → 7 → 8
- Web track: 9 + 10 (parallel) → 11 → 12

## Key Acceptance Criteria

- [ ] `#ralph <task>` starts autonomous loop from TUI
- [ ] Loop persists state to `.axiom/ralph/` directory
- [ ] Completion detected via `complete.txt` or output signals
- [ ] Max iterations (default 20) and timeouts enforced
- [ ] Real-time iteration progress in both TUI and Web
- [ ] `#ralph-stop` gracefully terminates loop
- [ ] Web workspace shows Ralph as right panel mode

## Reference

See `ralph-impl.md` for full architecture and design details.
