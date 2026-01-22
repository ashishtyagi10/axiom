---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Loop Executor

## Description
Implement the core Ralph Loop execution logic. The executor orchestrates iterations with fresh context per iteration, manages state persistence, detects completion, handles feedback between iterations, and enforces safety limits.

## Background
The Ralph Loop executor is the heart of the autonomous development cycle. It runs iterations until the task is complete or limits are reached. Each iteration gets fresh LLM context (avoiding accumulation issues) while state persists via files. The executor integrates with the existing Executor and Conductor systems.

## Technical Requirements
1. Create `crates/axiom-core/src/ralph/executor.rs` with RalphExecutor struct
2. Implement iteration orchestration:
   - Load/create state at start
   - Run iteration with current task + feedback
   - Capture iteration output/summary
   - Update state after iteration
   - Check completion conditions
   - Loop or terminate
3. Implement completion detection:
   - Check for `complete.txt` file
   - Parse output for completion signals (configurable patterns)
   - Manual completion via command
4. Implement safety limits:
   - Max iterations enforcement (default 20)
   - Per-iteration timeout (default 5 minutes)
   - Circuit breaker for repeated failures (3 consecutive failures)
5. Implement feedback loop:
   - Read feedback from previous iteration
   - Include feedback in next iteration prompt
   - Clear feedback after consumption
6. Emit notifications for all lifecycle events
7. Support graceful cancellation via StopRalphLoop command

## Dependencies
- Task 1: Ralph State Management
- Task 2: Ralph Commands
- Task 3: Ralph Notifications
- Existing Executor and Conductor in axiom-core
- tokio for async execution and timeouts

## Implementation Approach
1. Create RalphExecutor struct with state, config, notification sender
2. Implement `start()` method that spawns the loop
3. Implement `run_iteration()` that executes single iteration via agent system
4. Implement completion detection with configurable signals
5. Implement circuit breaker logic for failure handling
6. Add cancellation token support for graceful shutdown
7. Write integration tests with mock agent execution

## Acceptance Criteria

1. **Loop Execution**
   - Given a task and config
   - When RalphExecutor::start() is called
   - Then iterations run until completion or limit reached

2. **Fresh Context Per Iteration**
   - Given multiple iterations
   - When each iteration runs
   - Then it receives only task + feedback, not accumulated history

3. **State Persistence**
   - Given an iteration completing
   - When state is saved
   - Then loop can be resumed after restart

4. **Completion Detection - File**
   - Given complete.txt is created during iteration
   - When iteration finishes
   - Then loop terminates with TaskComplete reason

5. **Completion Detection - Output**
   - Given completion signal in iteration output
   - When output is parsed
   - Then loop terminates with TaskComplete reason

6. **Max Iterations Limit**
   - Given max_iterations = 5 in config
   - When 5 iterations complete without task completion
   - Then loop terminates with MaxIterations reason

7. **Timeout Enforcement**
   - Given timeout_per_iteration = 30 seconds
   - When iteration exceeds timeout
   - Then iteration is cancelled and marked as Timeout

8. **Circuit Breaker**
   - Given 3 consecutive iteration failures
   - When failure threshold reached
   - Then loop terminates with Error reason

9. **Graceful Cancellation**
   - Given a running loop
   - When StopRalphLoop is received
   - Then current iteration completes and loop terminates with UserStopped

10. **Notification Emission**
    - Given loop execution
    - When lifecycle events occur
    - Then appropriate notifications are emitted

## Metadata
- **Complexity**: High
- **Labels**: Backend, Executor, Core Logic, Ralph Loop
- **Required Skills**: Rust, async/await, tokio, state machines, error handling
