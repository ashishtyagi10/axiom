---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Service Integration

## Description
Integrate Ralph Loop into AxiomService, the main service facade. This includes handling Ralph commands, managing the executor lifecycle, spawning background execution, and forwarding notifications to connected UIs.

## Background
AxiomService is the main facade that handles commands from UIs and emits notifications. Ralph Loop integration requires adding state management to the service, handling the new commands, and coordinating between the executor and the notification system.

## Technical Requirements
1. Add Ralph state to AxiomService in `crates/axiom-core/src/service.rs`:
   - `ralph_state: Arc<RwLock<Option<RalphState>>>`
   - `ralph_executor: Arc<RwLock<Option<RalphExecutor>>>`
   - `ralph_cancel_token: Option<CancellationToken>`
2. Handle Ralph commands in command processing:
   - StartRalphLoop: Validate no active loop, create executor, spawn background task
   - StopRalphLoop: Signal cancellation, wait for graceful shutdown
   - GetRalphStatus: Return current state via notification
   - UpdateRalphFeedback: Update state with feedback for next iteration
3. Spawn executor in background tokio task
4. Forward executor notifications to UI notification channel
5. Handle executor completion and cleanup
6. Ensure thread-safe access to Ralph state

## Dependencies
- Task 1-4: All Ralph core components
- Existing AxiomService structure
- tokio for async execution
- parking_lot for RwLock

## Implementation Approach
1. Add Ralph-related fields to AxiomService struct
2. Create `handle_ralph_command()` method for command routing
3. Implement `start_ralph_loop()` that initializes and spawns executor
4. Implement `stop_ralph_loop()` with graceful shutdown
5. Wire notification forwarding from executor to service notification channel
6. Add cleanup on loop completion
7. Write integration tests with mock command flow

## Acceptance Criteria

1. **Start Ralph Loop**
   - Given no active loop
   - When StartRalphLoop command is received
   - Then executor is created and loop begins in background

2. **Prevent Duplicate Loops**
   - Given an active loop
   - When StartRalphLoop command is received
   - Then error notification is sent (only one loop per workspace)

3. **Stop Ralph Loop**
   - Given an active loop
   - When StopRalphLoop command is received
   - Then loop gracefully terminates and state is cleaned up

4. **Get Ralph Status**
   - Given any Ralph state (idle, running, complete)
   - When GetRalphStatus command is received
   - Then RalphStatusUpdate notification is emitted

5. **Update Feedback**
   - Given an active loop
   - When UpdateRalphFeedback command is received
   - Then feedback is stored for next iteration

6. **Notification Forwarding**
   - Given executor emitting notifications
   - When notifications are generated
   - Then they are forwarded to connected UIs

7. **Cleanup on Completion**
   - Given loop completing for any reason
   - When executor finishes
   - Then service state is cleaned up appropriately

8. **Thread Safety**
   - Given concurrent access to Ralph state
   - When multiple operations occur
   - Then no data races or deadlocks occur

## Metadata
- **Complexity**: High
- **Labels**: Backend, Service, Integration, Ralph Loop
- **Required Skills**: Rust, async/await, tokio, concurrency, service architecture
