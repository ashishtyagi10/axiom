---
status: pending
created: 2026-01-21
started: null
completed: null
---
# Task: Ralph TUI Input Commands

## Description
Extend the TUI input panel with Ralph Loop commands. Users can start, stop, and check status of Ralph Loop using `#ralph` command syntax, consistent with existing `#agent` command patterns.

## Background
The InputPanel supports `#agent prompt` syntax for invoking CLI agents. Ralph Loop commands should follow a similar pattern for consistency. The input panel parses commands and sends appropriate Commands to the backend.

## Technical Requirements
1. Extend input parsing in `crates/axiom-tui/src/panels/input.rs`:
   - `#ralph <task>`: Start Ralph Loop with given task
   - `#ralph-stop`: Stop current Ralph Loop
   - `#ralph-status`: Query and display Ralph Loop status
   - `#ralph-feedback <text>`: Provide feedback for next iteration
2. Parse optional config flags for #ralph command:
   - `--max <N>`: Set max iterations
   - `--timeout <seconds>`: Set per-iteration timeout
3. Send appropriate Command to backend via event bus
4. Provide user feedback on command acceptance/rejection
5. Add command autocompletion hints for #ralph commands

## Dependencies
- Task 2: Ralph Commands
- Existing InputPanel and command parsing
- EventBus for sending commands

## Implementation Approach
1. Add Ralph command patterns to input parser
2. Implement `parse_ralph_command()` function
3. Extract task and optional config flags
4. Construct appropriate Command variant
5. Send command via event bus
6. Add visual feedback (status message) on command send
7. Add #ralph to command autocomplete suggestions
8. Write unit tests for command parsing

## Acceptance Criteria

1. **Start Command**
   - Given input "#ralph implement the login feature"
   - When Enter is pressed
   - Then StartRalphLoop command is sent with task text

2. **Start with Config**
   - Given input "#ralph --max 10 --timeout 120 fix all bugs"
   - When Enter is pressed
   - Then StartRalphLoop command includes config with max=10, timeout=120

3. **Stop Command**
   - Given input "#ralph-stop"
   - When Enter is pressed
   - Then StopRalphLoop command is sent

4. **Status Command**
   - Given input "#ralph-status"
   - When Enter is pressed
   - Then GetRalphStatus command is sent

5. **Feedback Command**
   - Given input "#ralph-feedback focus on error handling"
   - When Enter is pressed
   - Then UpdateRalphFeedback command is sent with feedback text

6. **Invalid Command Handling**
   - Given input "#ralph" with no task
   - When Enter is pressed
   - Then error message is shown (task required)

7. **Autocomplete**
   - Given user typing "#ra"
   - When autocomplete is triggered
   - Then #ralph commands are suggested

8. **Command Feedback**
   - Given any Ralph command sent
   - When command is processed
   - Then status message confirms command was sent

## Metadata
- **Complexity**: Low
- **Labels**: TUI, Input, Commands, Ralph Loop
- **Required Skills**: Rust, parsing, command-line patterns
