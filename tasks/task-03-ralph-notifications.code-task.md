---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Loop Notifications

## Description
Add notifications for Ralph Loop events to the notification system. These notifications enable UIs to receive real-time updates about loop progress, iteration completion, and errors.

## Background
Axiom streams Notifications back to UIs for real-time updates. Ralph Loop needs notifications for iteration lifecycle events so TUI and Web can display progress, history, and status changes.

## Technical Requirements
1. Add new Notification variants to `crates/axiom-core/src/notifications.rs`:
   - `RalphIterationStarted { iteration: u32, task: String }`: Iteration beginning
   - `RalphIterationComplete { iteration: u32, summary: String, status: IterationStatus }`: Iteration finished
   - `RalphLoopComplete { total_iterations: u32, reason: CompletionReason }`: Loop finished
   - `RalphLoopError { iteration: u32, error: String }`: Error during iteration
   - `RalphStatusUpdate { state: RalphState }`: Status query response
2. Define `IterationStatus` enum: Success, Failed, Timeout
3. Define `CompletionReason` enum: TaskComplete, MaxIterations, UserStopped, Error
4. Implement serde serialization for all new types
5. Ensure notifications can carry sufficient context for UI rendering

## Dependencies
- Task 1: Ralph State Management (RalphState type)
- Existing Notification enum in axiom-core
- serde for serialization

## Implementation Approach
1. Define IterationStatus and CompletionReason enums
2. Add new variants to Notification enum
3. Ensure all nested types derive necessary traits (Clone, Debug, Serialize, Deserialize)
4. Add helper constructors for common notification patterns
5. Write tests for notification serialization

## Acceptance Criteria

1. **Iteration Started Notification**
   - Given a new iteration beginning
   - When RalphIterationStarted is emitted
   - Then it contains iteration number and current task

2. **Iteration Complete Notification**
   - Given an iteration finishing
   - When RalphIterationComplete is emitted
   - Then it contains iteration number, summary, and status

3. **Loop Complete Notification**
   - Given a loop finishing for any reason
   - When RalphLoopComplete is emitted
   - Then it contains total iterations and completion reason

4. **Error Notification**
   - Given an error during iteration
   - When RalphLoopError is emitted
   - Then it contains iteration number and error details

5. **Status Update Notification**
   - Given a GetRalphStatus command
   - When RalphStatusUpdate is emitted
   - Then it contains full current RalphState

6. **Notification Serialization**
   - Given any Ralph notification variant
   - When serialized for WebSocket/IPC transport
   - Then it deserializes correctly on the client side

## Metadata
- **Complexity**: Low
- **Labels**: Backend, Notifications, Events, Ralph Loop
- **Required Skills**: Rust, serde, event-driven design
