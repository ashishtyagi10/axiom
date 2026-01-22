---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph State Management

## Description
Create the state management foundation for Ralph Loop iterations. This includes defining the core data structures (RalphState, RalphConfig) and implementing file-based state persistence to the `.axiom/ralph/` directory.

## Background
The Ralph Loop pattern enables continuous autonomous development cycles with fresh context on each iteration. State must persist between iterations via files since each iteration gets fresh LLM context. The `.axiom/ralph/` directory serves as the persistence layer for task description, iteration count, feedback, and completion signals.

## Technical Requirements
1. Create `RalphState` struct containing: current task, iteration count, feedback history, completion status, started_at timestamp
2. Create `RalphConfig` struct containing: max_iterations (default 20), timeout_per_iteration (default 5 min), completion_signals configuration
3. Implement state persistence to `.axiom/ralph/` directory with files:
   - `task.md`: Current task description
   - `iteration.txt`: Current iteration number
   - `feedback.txt`: Feedback from last iteration
   - `work-summary.txt`: Summary of work done
   - `complete.txt`: Completion signal file
4. Implement state loading from persistence files
5. Implement state clearing/reset functionality
6. Add serialization/deserialization for state structs (serde)

## Dependencies
- serde, serde_json for serialization
- tokio for async file operations
- thiserror for error types
- Existing axiom-core crate structure

## Implementation Approach
1. Create new module `crates/axiom-core/src/ralph/mod.rs` and `state.rs`
2. Define RalphState and RalphConfig with serde derives
3. Implement RalphState methods: new(), load(), save(), clear(), update_iteration(), set_feedback(), mark_complete()
4. Add RalphError variants to error handling
5. Write unit tests for state persistence round-trips

## Acceptance Criteria

1. **State Creation**
   - Given a task description and config
   - When RalphState::new() is called
   - Then a valid state is created with iteration 0 and empty feedback history

2. **State Persistence**
   - Given an active RalphState
   - When save() is called
   - Then all state files are written to `.axiom/ralph/` directory

3. **State Loading**
   - Given existing `.axiom/ralph/` files
   - When RalphState::load() is called
   - Then state is correctly reconstructed from files

4. **Iteration Update**
   - Given an active RalphState at iteration N
   - When update_iteration() is called with summary
   - Then iteration increments to N+1 and summary is recorded

5. **Completion Detection**
   - Given a RalphState
   - When complete.txt exists or mark_complete() is called
   - Then is_complete() returns true

6. **Unit Test Coverage**
   - Given the state management implementation
   - When running the test suite
   - Then all persistence and state transition scenarios have tests

## Metadata
- **Complexity**: Medium
- **Labels**: Backend, State Management, Persistence, Ralph Loop
- **Required Skills**: Rust, serde, async file I/O, state machines
