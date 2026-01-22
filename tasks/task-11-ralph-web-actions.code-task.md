---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Web Actions

## Description
Create Next.js server actions for Ralph Loop operations. These actions provide the server-side API for the web UI to communicate with the axiom-core backend.

## Background
Next.js server actions allow the web UI to invoke backend operations securely. Ralph Loop actions need to send commands to the axiom-core service and handle responses appropriately.

## Technical Requirements
1. Create `web/src/app/actions/ralph.ts` with server actions:
   - `startRalphLoopAction(workspaceId: string, task: string, config?: RalphConfig)`: Start loop
   - `stopRalphLoopAction(workspaceId: string)`: Stop current loop
   - `getRalphStatusAction(workspaceId: string)`: Get current status
   - `provideFeedbackAction(workspaceId: string, feedback: string)`: Submit feedback
2. Validate inputs before sending to backend
3. Handle errors and return appropriate error responses
4. Use existing workspace connection infrastructure
5. Add TypeScript types for action parameters and returns

## Dependencies
- Task 2: Ralph Commands (command types)
- Existing server action patterns in web app
- Workspace connection/service infrastructure

## Implementation Approach
1. Create ralph.ts with 'use server' directive
2. Define input validation for each action
3. Implement startRalphLoopAction with task and config handling
4. Implement stopRalphLoopAction with workspace lookup
5. Implement getRalphStatusAction for status queries
6. Implement provideFeedbackAction for feedback submission
7. Add error handling and logging
8. Write action tests

## Acceptance Criteria

1. **Start Action**
   - Given valid workspaceId and task
   - When startRalphLoopAction is called
   - Then StartRalphLoop command is sent to backend

2. **Start Action Validation**
   - Given empty task
   - When startRalphLoopAction is called
   - Then validation error is returned

3. **Stop Action**
   - Given valid workspaceId
   - When stopRalphLoopAction is called
   - Then StopRalphLoop command is sent to backend

4. **Status Action**
   - Given valid workspaceId
   - When getRalphStatusAction is called
   - Then GetRalphStatus command is sent and response returned

5. **Feedback Action**
   - Given valid workspaceId and feedback
   - When provideFeedbackAction is called
   - Then UpdateRalphFeedback command is sent

6. **Config Handling**
   - Given custom config with maxIterations and timeout
   - When startRalphLoopAction is called with config
   - Then config is passed to backend correctly

7. **Error Handling**
   - Given backend error
   - When any action fails
   - Then appropriate error is returned to client

8. **Workspace Validation**
   - Given invalid workspaceId
   - When any action is called
   - Then workspace not found error is returned

## Metadata
- **Complexity**: Low
- **Labels**: Web, Server Actions, API, Ralph Loop
- **Required Skills**: Next.js, TypeScript, server actions, error handling
