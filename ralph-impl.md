Ralph Loop Implementation Plan
Problem Statement
Axiom currently has agent execution capabilities but lacks an autonomous iterative loop mechanism that can continuously work on tasks until completion. The Ralph Loop pattern enables continuous autonomous development cycles with fresh context on each iteration, avoiding context accumulation issues while persisting state through files and git.
Current State
Existing Architecture
Backend (axiom-core): Service-oriented architecture with AxiomService as the main facade
Command/Notification Pattern: Commands sent to backend, Notifications streamed back to UIs
Agent System: Executor, Conductor, AgentRegistry, PtyAgentManager for various agent types
TUI (axiom-tui): TuiApp with event loop, four main panels (FileTree, Output, Input, Agents)
Web (Next.js): WorkspaceStudio with orchestrator/developer agents, WebSocket connection for real-time updates
Agent Types: Shell, Coder, Search, FileOps, CliAgent, Conductor, Custom
Output Streaming: Agent output streams via Event::AgentOutput, PTY output via Event::PtyOutput
Gap Analysis
No autonomous loop mechanism to retry tasks until completion
No state persistence between iterations (task description, iteration count, feedback)
No completion detection mechanism
No iteration history or feedback tracking
No max iteration safeguards
Proposed Changes
Core Backend (axiom-core)
1. Ralph Loop State Management (crates/axiom-core/src/ralph/state.rs)
Create state management for Ralph Loop iterations:
RalphState: Current task, iteration count, feedback history, completion status
RalphConfig: Max iterations, timeout per iteration, completion signals
State persistence via .axiom/ralph/ directory:
task.md: Current task description
iteration.txt: Current iteration number
feedback.txt: Feedback from last iteration
work-summary.txt: Summary of work done
complete.txt: Completion signal file
2. Ralph Loop Commands (crates/axiom-core/src/commands/mod.rs)
Extend Command enum with Ralph Loop operations:
StartRalphLoop { task, config }: Begin a Ralph Loop session
StopRalphLoop: Terminate current loop
GetRalphStatus: Query current loop status
UpdateRalphFeedback { feedback }: Provide feedback to next iteration
3. Ralph Loop Notifications (crates/axiom-core/src/notifications.rs)
Add notifications for Ralph Loop events:
RalphIterationStarted { iteration, task }
RalphIterationComplete { iteration, summary, status }
RalphLoopComplete { total_iterations, reason }
RalphLoopError { iteration, error }
4. Ralph Loop Executor (crates/axiom-core/src/ralph/executor.rs)
Implement the core Ralph Loop logic:
Iteration orchestration with fresh context per iteration
State persistence to .axiom/ralph/ files
Completion detection (check for complete.txt or completion signals in output)
Feedback loop between iterations
Safety limits (max iterations, timeouts)
Integration with existing Executor and Conductor
5. Ralph Loop Service Integration (crates/axiom-core/src/service.rs)
Integrate Ralph Loop into AxiomService:
Add ralph_state: Arc<RwLock<Option<RalphState>>>
Handle StartRalphLoop command
Spawn background thread for loop execution
Forward iterations to agent execution system
Emit notifications for UI updates
TUI Implementation (axiom-tui)
6. Ralph Loop Panel (crates/axiom-tui/src/panels/ralph.rs)
Create a dedicated panel for Ralph Loop visualization:
Display current iteration number
Show task description
Real-time iteration progress
Iteration history with status (running/complete/failed)
Last feedback/summary
Start/Stop controls
Configuration options
7. TUI App Integration (crates/axiom-tui/src/app.rs)
Integrate Ralph Loop panel into TUI:
Add ralph panel to panel registry
Layout adjustment to accommodate ralph panel (optional toggle)
Handle ralph-specific input commands
Process Ralph Loop notifications
Update ralph panel from service state
8. Ralph Loop Input Commands (crates/axiom-tui/src/panels/input.rs)
Extend input panel with Ralph Loop commands:
#ralph <task>: Start Ralph Loop
#ralph-stop: Stop current loop
#ralph-status: Show loop status
Web Implementation (web)
9. Ralph Loop React Component (web/src/components/RalphLoop.tsx)
Create React component for Ralph Loop UI:
Iteration counter and progress
Task display and edit capability
Real-time status updates
Iteration history timeline
Start/Stop/Pause controls
Configuration modal (max iterations, timeout)
Feedback input for manual intervention
10. Ralph Loop Hook (web/src/lib/api/hooks.ts)
Add useRalphLoop hook for state management:
Connection to WebSocket for real-time updates
Ralph Loop command sending
Local state for iteration history
Status tracking (idle/running/paused/complete)
11. Ralph Loop Actions (web/src/app/actions/ralph.ts)
Server actions for Ralph Loop operations:
startRalphLoopAction(workspaceId, task, config)
stopRalphLoopAction(workspaceId)
getRalphStatusAction(workspaceId)
provideFeedbackAction(workspaceId, feedback)
12. WorkspaceStudio Integration (web/src/app/workspace/[id]/WorkspaceStudio.tsx)
Integrate Ralph Loop into workspace:
Add ralph mode to right panel (team/terminal/ralph)
Render RalphLoop component
Handle ralph-specific notifications via WebSocket
Status indicator in UI header
Testing & Validation
13. Unit Tests
Ralph Loop state management tests
Completion detection tests
Iteration limit enforcement tests
State persistence tests
14. Integration Tests
End-to-end loop execution test
TUI rendering tests
Web UI interaction tests
WebSocket notification flow tests
Implementation Sequence
Core backend state and command infrastructure (items 1-3)
Ralph Loop executor implementation (item 4)
Service integration (item 5)
TUI panel and integration (items 6-8)
Web component and hooks (items 9-10)
Web actions and integration (items 11-12)
Testing and validation (items 13-14)
Technical Considerations
Context Management: Each iteration should have fresh LLM context, state persists via files
Completion Detection: Support multiple signals (file-based, output parsing, manual confirmation)
Safety: Max iterations (default 20), per-iteration timeout (default 5 min), circuit breaker for repeated failures
Concurrent Loops: Support only one active Ralph Loop per workspace initially
Git Integration: Optionally commit after each successful iteration
Error Handling: Graceful degradation, iteration failures don't kill the loop immediately
