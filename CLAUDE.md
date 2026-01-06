# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Axiom is a Rust-based terminal IDE with AI integration. It's a TUI (Text User Interface) application built with Ratatui that integrates local LLM models via Ollama.

## Build & Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release with LTO optimization

# Run
cargo run                      # Run in debug mode
cargo run --release            # Run optimized build

# Test
cargo test                     # Run all tests
cargo test <module>::tests     # Run tests for specific module (e.g., cargo test state::focus::tests)

# Lint & Format
cargo fmt                      # Format code
cargo clippy                   # Lint with Clippy
```

## Architecture

### Panel-Based Design

All UI panels implement the `Panel` trait (`src/panels/mod.rs`). The four main panels are:
- **FileTreePanel** (`panels/file_tree.rs`): Directory navigation with expand/collapse
- **EditorPanel** (`panels/editor/`): Text editor with syntax highlighting and diff tracking
- **TerminalPanel** (`panels/terminal.rs`): Integrated PTY terminal with resize support
- **ChatPanel** (`panels/chat.rs`): AI chat interface with markdown rendering

New panels should implement the `Panel` trait and register in `PanelRegistry`.

### Event-Driven Architecture

```
User Input → crossterm → Input Reader Thread
                              ↓
                    Bounded Crossbeam Channel (EventBus)
                              ↓
                    Main Event Loop (50ms timeout)
                              ↓
                    Event Handlers + Panel Input
```

The `EventBus` (`src/events/mod.rs`) uses bounded channels with backpressure. New async operations should emit events rather than blocking the UI thread.

### State Management

- **AppState** (`src/state/app.rs`): Minimal central state (input mode, focus, quit flag, status message, cwd)
- **Panel-specific state**: Lives within each panel, not in AppState
- **InputMode** (`src/state/input_mode.rs`): State machine for Normal/Insert/Command/Search/Modal modes
- **FocusState** (`src/state/focus.rs`): Ring buffer for Tab cycling between panels

### LLM Integration

The `LlmProvider` trait (`src/llm/mod.rs`) allows different LLM backends. Currently implements Ollama (`src/llm/ollama.rs`) connecting to `localhost:11434`. Responses stream via the event bus.

**LLM File Modifications**: When an LLM response contains code blocks with file paths, the changes are automatically applied to the editor with diff visualization:
- Format: `` ```rust:src/main.rs `` (language:path) or `` ```src/main.rs `` (path only)
- The editor shows diffs with green `+` for additions, yellow `~` for modifications
- Title shows `[DIFF]` indicator when tracking changes
- Flow: `ChatPanel` parses response → `Event::FileModification` → `EditorPanel.apply_llm_modification()`

### Key Patterns

- **Thread-safe state**: Use `Arc<Mutex>` or `Arc<RwLock>` (via parking_lot) for shared state
- **Error handling**: All operations return `Result` types using thiserror
- **PTY management**: `PtyWrapper` (`src/terminal/pty.rs`) handles terminal emulation with dynamic resize
- **UI layout**: Responsive percentage-based layouts in `src/ui/layout.rs`, focus-aware sizing

## Module Structure

```
src/
├── main.rs           # Entry point, terminal setup, event loop
├── core/error.rs     # Typed error system
├── state/            # AppState, FocusState, InputMode
├── events/           # EventBus with bounded channels
├── panels/           # Panel trait and implementations
│   └── editor/       # Editor panel with highlight.rs, diff.rs
├── terminal/pty.rs   # PTY wrapper
├── llm/              # LlmProvider trait + Ollama
├── ui/               # Layout, rendering, markdown, modals
└── fs/               # File system utilities
```

## Development Guidelines

- Maintain trait-based composition for panels
- Keep panel-specific state encapsulated within panels
- New async operations should emit events through EventBus
- Use bounded channels to prevent memory bloat
- Tests are embedded in modules using `#[cfg(test)]` blocks
