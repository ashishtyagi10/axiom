---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Web Hook

## Description
Create the `useRalphLoop` React hook for Ralph Loop state management in the web application. The hook handles WebSocket connection for real-time updates, command sending, and local state management.

## Background
The web application uses React hooks for state management and WebSocket connections for real-time updates from the backend. The useRalphLoop hook encapsulates all Ralph Loop logic for easy consumption by components.

## Technical Requirements
1. Create hook in `web/src/lib/api/hooks.ts` or new file `web/src/hooks/useRalphLoop.ts`:
   - Connect to WebSocket for Ralph notifications
   - Maintain local state: status, currentIteration, iterationHistory, task, config
   - Provide command functions: startLoop, stopLoop, getStatus, provideFeedback
   - Handle reconnection and connection state
2. State interface:
   ```typescript
   interface RalphLoopState {
     status: 'idle' | 'running' | 'paused' | 'complete' | 'error';
     currentIteration: number;
     maxIterations: number;
     task: string;
     iterationHistory: IterationRecord[];
     lastError?: string;
     elapsedTime?: number;
   }
   ```
3. Handle all Ralph notification types and update state accordingly
4. Provide loading and error states
5. Support workspace context (workspaceId parameter)

## Dependencies
- Task 3: Ralph Notifications (notification types)
- Task 11: Ralph Web Actions (server actions) - can mock initially
- Existing WebSocket infrastructure in web app

## Implementation Approach
1. Define TypeScript interfaces for state and iteration records
2. Create useRalphLoop hook with useState/useReducer for state
3. Set up WebSocket listener for Ralph notifications
4. Implement notification handlers to update state
5. Create command functions that call server actions
6. Add cleanup on unmount
7. Handle edge cases (reconnection, stale state)
8. Write hook tests with mock WebSocket

## Acceptance Criteria

1. **Hook Initialization**
   - Given useRalphLoop(workspaceId) called
   - When hook initializes
   - Then returns initial state with status 'idle'

2. **WebSocket Connection**
   - Given hook mounted
   - When WebSocket connects
   - Then Ralph notifications are received

3. **Start Loop**
   - Given startLoop(task, config) called
   - When command is sent
   - Then state updates to 'running' on success

4. **Stop Loop**
   - Given stopLoop() called
   - When command is sent
   - Then state updates appropriately on completion

5. **Iteration Started Notification**
   - Given RalphIterationStarted notification received
   - When handler processes it
   - Then currentIteration updates

6. **Iteration Complete Notification**
   - Given RalphIterationComplete notification received
   - When handler processes it
   - Then iteration is added to history with status

7. **Loop Complete Notification**
   - Given RalphLoopComplete notification received
   - When handler processes it
   - Then status updates to 'complete' with reason

8. **Error Handling**
   - Given RalphLoopError notification received
   - When handler processes it
   - Then lastError is set and status may update

9. **Provide Feedback**
   - Given provideFeedback(text) called
   - When command is sent
   - Then feedback is submitted for next iteration

10. **Cleanup on Unmount**
    - Given component unmounts
    - When hook cleanup runs
    - Then WebSocket listener is removed

## Metadata
- **Complexity**: Medium
- **Labels**: Web, React, Hook, State Management, WebSocket, Ralph Loop
- **Required Skills**: React hooks, TypeScript, WebSocket, state management
