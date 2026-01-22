# Ralph Loop Implementation Scratchpad

## Overview
Implementing Ralph Loop - an autonomous iterative loop mechanism for Axiom that continuously works on tasks until completion.

## Architecture Summary

### Backend (axiom-core)
- **Commands**: `StartRalphLoop`, `StopRalphLoop`, `GetRalphStatus`, `UpdateRalphFeedback`
- **Notifications**: `RalphIterationStarted`, `RalphIterationComplete`, `RalphLoopComplete`, `RalphLoopError`, `RalphStatusUpdate`
- **State**: Persisted to `.axiom/ralph/` directory (task.md, iteration.txt, feedback.txt, work-summary.txt, complete.txt)
- **Executor**: Orchestrates iterations with fresh context, completion detection, safety limits

### TUI (axiom-tui)
- **RalphPanel**: New panel showing iteration progress, history, status
- **Input Commands**: `#ralph <task>`, `#ralph-stop`, `#ralph-status`, `#ralph-feedback <text>`
- **Integration**: Panel registration, focus cycling, notification routing

### Web (Next.js)
- **RalphLoop Component**: UI for iteration visualization, controls, history
- **useRalphLoop Hook**: Polling for status, state management
- **Server Actions**: startRalphLoopAction, stopRalphLoopAction, getRalphStatusAction, provideFeedbackAction
- **WorkspaceStudio Integration**: New 'ralph' mode in right panel (pending)

---

## Task Tracking

### Backend Core (Tasks 1-5) - COMPLETED ✓

- [x] **Task 1: Ralph State Management** (`crates/axiom-core/src/ralph/state.rs`)
  - RalphState, RalphConfig structs ✓
  - File persistence to `.axiom/ralph/` ✓
  - State loading/saving/clearing ✓
  - Unit tests ✓

- [x] **Task 2: Ralph Commands** (`crates/axiom-core/src/commands/mod.rs`)
  - Added command variants to Command enum ✓
  - Serde serialization ✓
  - Builder methods ✓
  - Unit tests ✓

- [x] **Task 3: Ralph Notifications** (`crates/axiom-core/src/notifications.rs`)
  - Added notification variants ✓
  - IterationStatus, CompletionReason enums ✓
  - Serde serialization ✓
  - Unit tests ✓

- [x] **Task 4: Ralph Executor** (`crates/axiom-core/src/ralph/executor.rs`)
  - Iteration orchestration ✓
  - Completion detection (file + output signals) ✓
  - Safety limits (max iterations, timeouts, circuit breaker) ✓
  - Feedback loop ✓
  - Graceful cancellation ✓
  - Unit tests ✓

- [x] **Task 5: Ralph Service Integration** (`crates/axiom-core/src/service.rs`)
  - Added ralph state to AxiomService ✓
  - Command handlers ✓
  - Background task spawning ✓
  - Notification forwarding ✓
  - Unit tests ✓

### TUI (Tasks 6-8) - COMPLETED ✓

- [x] **Task 6: Ralph TUI Panel** (`crates/axiom-tui/src/panels/ralph.rs`)
  - Panel trait implementation ✓
  - Iteration display, history, status ✓
  - Scrolling support ✓

- [x] **Task 7: Ralph TUI App Integration** (`crates/axiom-tui/src/app.rs`)
  - Panel registration ✓
  - Focus cycling (PanelId::RALPH = 4) ✓
  - Notification routing ✓
  - Layout adjustment (shows when Ralph is active) ✓

- [x] **Task 8: Ralph TUI Input Commands** (`crates/axiom-tui/src/panels/input.rs`)
  - Parse #ralph commands ✓
  - #ralph-stop, #ralph-status, #ralph-feedback ✓
  - Command sending ✓

### Web (Tasks 9-13) - COMPLETED ✓

- [x] **Task 9: Ralph Web Component** (`web/src/components/RalphPanel.tsx`)
  - Progress display ✓
  - Controls (start/stop) ✓
  - Iteration history ✓
  - Feedback input ✓
  - Completion display ✓

- [x] **Task 10: Ralph Web Hook** (`web/src/lib/api/hooks.ts`)
  - Polling for status updates ✓
  - State management ✓
  - Command functions (startLoop, stopLoop, updateFeedback) ✓

- [x] **Task 11: Ralph Web Actions** (`web/src/app/actions/ralph.ts`)
  - Server actions for Ralph operations ✓
  - Input validation ✓
  - Error handling ✓

- [x] **Task 12: Ralph Web Types** (`web/src/lib/api/types.ts`)
  - RalphState, RalphConfig, IterationRecord types ✓
  - RalphNotification type ✓
  - DEFAULT_RALPH_CONFIG constant ✓

- [x] **Task 13: Ralph Web API Client** (`web/src/lib/api/client.ts`)
  - startRalphLoop, stopRalphLoop, getRalphStatus, updateRalphFeedback methods ✓

---

## Remaining Work

### Critical: Actual Agent Invocation (COMPLETED ✓)

- [x] **Task 14: Implement CliAgentInvoker** (`crates/axiom-core/src/ralph/executor.rs:57-235`)
  - Implemented actual PTY session creation using `portable_pty` crate ✓
  - CommandBuilder with config (command, args, cwd, env vars) ✓
  - Background reader thread with vt100 parsing ✓
  - Wait for completion via Exit signal or timeout ✓
  - Extract text output from vt100 parser screen ✓
  - All 37 Ralph tests passing ✓

### Nice-to-have: WorkspaceStudio Integration

- [~] **Task 15: Ralph WorkspaceStudio Integration** (`web/src/app/workspace/[id]/WorkspaceStudio.tsx`)
  - RalphPanel component exists but not integrated into WorkspaceStudio
  - Would need to add 'ralph' to right panel modes
  - Would need Ralph tab in right sidebar
  - Deferred - component is ready, just not wired up

---

## Build Status

✓ `cargo build --package axiom-core --package axiom-tui` passes with only warnings
✓ All 37 Ralph unit tests pass

---

## Implementation Summary

**Completed:**
- Full state management with file persistence
- Comprehensive notifications for all Ralph events
- Complete TUI panel with history, progress, status
- Complete web component with controls and history
- Hooks and API client for web integration
- Input commands for TUI (#ralph, #ralph-stop, etc.)
- All unit tests passing

**Completed:**
- **CliAgentInvoker.invoke()** fully implemented with PTY integration
- Spawns CLI agents via PTY with vt100 terminal emulation
- Ralph Loop is now fully functional end-to-end

---

## Notes

- Each iteration needs fresh LLM context (state persists via files)
- Safety limits: max_iterations=20, timeout_per_iteration=5min, circuit_breaker=3 failures
- Only one active Ralph Loop per workspace
- Completion detection: `complete.txt` file OR configurable output signals (LOOP_COMPLETE, TASK_COMPLETE, DONE)
