---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph Web Component

## Description
Create a React component for Ralph Loop UI in the Next.js web application. The component provides iteration visualization, task display, controls, and real-time status updates.

## Background
The web UI uses React components within a Next.js application. The Ralph Loop component will be displayed in the workspace, showing autonomous loop progress similar to the TUI panel but with web-appropriate interactions.

## Technical Requirements
1. Create `web/src/components/RalphLoop.tsx` with the following elements:
   - Iteration counter and progress bar (e.g., "3/20" with visual progress)
   - Task display with edit capability (when idle)
   - Real-time status indicator (Idle, Running, Complete, Error)
   - Iteration history timeline/list with expandable details
   - Start/Stop/Pause controls
   - Configuration modal (max iterations, timeout settings)
   - Feedback input for manual intervention
   - Elapsed time display
2. Use existing design system components (buttons, inputs, cards)
3. Support responsive layout for different screen sizes
4. Implement accessibility (ARIA labels, keyboard navigation)
5. Handle loading and error states gracefully

## Dependencies
- Task 10: Ralph Web Hook (for state management) - can be developed in parallel with mock data
- Existing web component library and styling
- React 18+ features

## Implementation Approach
1. Create component structure with TypeScript interfaces for props/state
2. Implement layout with header (status/controls), body (history), footer (feedback)
3. Add iteration timeline with status icons and expandable summaries
4. Implement control buttons with appropriate disabled states
5. Create configuration modal using existing modal patterns
6. Add feedback input with submit functionality
7. Style using existing Tailwind/CSS patterns
8. Write component tests with React Testing Library

## Acceptance Criteria

1. **Component Rendering**
   - Given RalphLoop component mounted
   - When rendered with state
   - Then all UI elements display correctly

2. **Iteration Progress**
   - Given active loop at iteration 5 of 20
   - When component renders
   - Then progress shows "5/20" with 25% filled progress bar

3. **Status Display**
   - Given different Ralph states
   - When status changes
   - Then indicator updates with appropriate color and icon

4. **Iteration History**
   - Given completed iterations
   - When history section renders
   - Then each iteration shows with status, summary preview, and expand option

5. **Start Control**
   - Given idle state with task entered
   - When Start button is clicked
   - Then onStart callback is invoked with task and config

6. **Stop Control**
   - Given running state
   - When Stop button is clicked
   - Then onStop callback is invoked

7. **Configuration Modal**
   - Given settings button clicked
   - When modal opens
   - Then max iterations and timeout can be configured

8. **Feedback Input**
   - Given feedback text entered
   - When Submit feedback is clicked
   - Then onFeedback callback is invoked with text

9. **Responsive Layout**
   - Given different viewport sizes
   - When component renders
   - Then layout adapts appropriately

10. **Accessibility**
    - Given screen reader user
    - When navigating component
    - Then all controls have appropriate ARIA labels

## Metadata
- **Complexity**: Medium
- **Labels**: Web, React, Component, UI, Ralph Loop
- **Required Skills**: React, TypeScript, Tailwind CSS, accessibility
