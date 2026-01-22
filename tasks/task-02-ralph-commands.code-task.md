---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Loop Commands

## Description
Extend the Command enum in axiom-core with Ralph Loop operations. These commands enable UIs (TUI and Web) to control Ralph Loop execution through the existing command/notification pattern.

## Background
Axiom uses a Command/Notification pattern where Commands are sent to the backend and Notifications are streamed back to UIs. Ralph Loop needs commands for starting, stopping, querying status, and providing feedback to iterations.

## Technical Requirements
1. Add new Command variants to `crates/axiom-core/src/commands/mod.rs`:
   - `StartRalphLoop { task: String, config: RalphConfig }`: Begin a Ralph Loop session
   - `StopRalphLoop`: Terminate current loop
   - `GetRalphStatus`: Query current loop status
   - `UpdateRalphFeedback { feedback: String }`: Provide feedback to next iteration
2. Implement serde serialization for new commands
3. Add command validation (e.g., can't start if already running)
4. Document command usage and expected responses

## Dependencies
- Task 1: Ralph State Management (RalphConfig type)
- Existing Command enum in axiom-core
- serde for serialization

## Implementation Approach
1. Import RalphConfig from ralph::state module
2. Add new variants to Command enum with appropriate fields
3. Ensure serde derives work correctly for new variants
4. Add any necessary helper methods for command construction
5. Update command documentation

## Acceptance Criteria

1. **StartRalphLoop Command**
   - Given a task description and optional config
   - When StartRalphLoop command is constructed
   - Then it serializes correctly and contains all required fields

2. **StopRalphLoop Command**
   - Given an active Ralph Loop
   - When StopRalphLoop command is sent
   - Then it can be deserialized and processed by the service

3. **GetRalphStatus Command**
   - Given any Ralph Loop state (idle, running, complete)
   - When GetRalphStatus command is sent
   - Then it triggers a status response

4. **UpdateRalphFeedback Command**
   - Given feedback text
   - When UpdateRalphFeedback command is constructed
   - Then the feedback is preserved through serialization

5. **Command Serialization**
   - Given any Ralph command variant
   - When serialized to JSON and back
   - Then the command is identical to the original

6. **Unit Test Coverage**
   - Given the command implementations
   - When running the test suite
   - Then all command variants have serialization round-trip tests

## Metadata
- **Complexity**: Low
- **Labels**: Backend, Commands, API, Ralph Loop
- **Required Skills**: Rust, serde, enum design
