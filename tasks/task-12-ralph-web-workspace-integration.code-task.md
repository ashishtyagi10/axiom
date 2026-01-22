---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Web Workspace Integration

## Description
Integrate Ralph Loop into the WorkspaceStudio component. This adds Ralph as a mode option in the right panel, handles Ralph-specific notifications via WebSocket, and provides status indication in the UI.

## Background
WorkspaceStudio is the main workspace view containing orchestrator/developer agents and multiple panel modes. Ralph Loop should be integrated as an additional mode in the right panel (alongside team/terminal), with appropriate WebSocket handling and status visibility.

## Technical Requirements
1. Modify `web/src/app/workspace/[id]/WorkspaceStudio.tsx`:
   - Add 'ralph' to right panel mode options (team/terminal/ralph)
   - Render RalphLoop component when ralph mode is selected
   - Pass useRalphLoop hook state to component
2. Add Ralph mode tab/button in panel header
3. Handle Ralph notifications via existing WebSocket connection:
   - Route Ralph* notifications to useRalphLoop hook
   - Update UI state on notification receipt
4. Add status indicator in workspace header when Ralph is running:
   - Small badge or icon showing Ralph active
   - Click to switch to Ralph panel
5. Consider keyboard shortcut to toggle Ralph panel
6. Ensure proper state cleanup on workspace unmount/change

## Dependencies
- Task 9: Ralph Web Component
- Task 10: Ralph Web Hook
- Task 11: Ralph Web Actions
- Existing WorkspaceStudio structure

## Implementation Approach
1. Add 'ralph' to panel mode type and state
2. Add Ralph tab in right panel header
3. Import and integrate useRalphLoop hook
4. Add conditional rendering for RalphLoop component
5. Wire up WebSocket notification routing for Ralph events
6. Add header status indicator component
7. Implement keyboard shortcut (e.g., Ctrl+R for Ralph)
8. Test integration with running Ralph Loop

## Acceptance Criteria

1. **Panel Mode Option**
   - Given workspace loaded
   - When right panel renders
   - Then Ralph tab/mode is available

2. **Mode Switching**
   - Given Ralph tab clicked
   - When mode changes to 'ralph'
   - Then RalphLoop component renders

3. **Hook Integration**
   - Given ralph mode active
   - When useRalphLoop provides state
   - Then RalphLoop component receives props

4. **Notification Routing**
   - Given WebSocket Ralph notification
   - When notification is received
   - Then it is forwarded to useRalphLoop handler

5. **Header Status Indicator**
   - Given active Ralph Loop
   - When workspace header renders
   - Then Ralph running indicator is visible

6. **Indicator Click**
   - Given Ralph indicator clicked
   - When clicked
   - Then right panel switches to ralph mode

7. **Keyboard Shortcut**
   - Given any panel focused
   - When Ralph shortcut is pressed
   - Then right panel switches to ralph mode

8. **State Cleanup**
   - Given workspace unmount or change
   - When component unmounts
   - Then Ralph state and listeners are cleaned up

9. **Initial Load**
   - Given workspace with existing Ralph Loop running
   - When workspace loads
   - Then Ralph state is restored and displayed

10. **Error Display**
    - Given Ralph Loop error
    - When error notification received
    - Then error is displayed appropriately in UI

## Metadata
- **Complexity**: Medium
- **Labels**: Web, Integration, WorkspaceStudio, Ralph Loop
- **Required Skills**: React, Next.js, WebSocket, component integration
