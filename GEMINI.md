# Axiom - Terminal IDE with AI Integration

## Project Overview
Axiom is a Rust-based Terminal User Interface (TUI) Integrated Development Environment (IDE) that seamlessly integrates local Large Language Models (LLMs) via Ollama. It features a panel-based layout with a file tree, text editor, terminal, and AI chat interface.

## Architecture

### Core Components
*   **TUI Framework**: Built using `ratatui` for rendering and `crossterm` for event handling.
*   **Event-Driven**: Uses a bounded `crossbeam-channel` as an `EventBus` to decouple input/async tasks from the main UI loop.
*   **Panel System**: The UI is composed of panels implementing the `Panel` trait (`src/panels/mod.rs`).
    *   `FileTreePanel`: Directory navigation.
    *   `EditorPanel`: Text editing with syntax highlighting (syntect) and diff visualization.
    *   `TerminalPanel`: Integrated terminal using `portable-pty`.
    *   `ChatPanel`: AI interface for communicating with Ollama.
    *   `ModelSelector`: Modal for switching LLM models (triggered by `Ctrl+M`).

### Data Flow
1.  **Input**: User input (Key/Mouse) is captured in a separate thread and sent to the `EventBus`.
2.  **Main Loop**: The main thread (`run_app` in `src/main.rs`) processes events from the bus with a 50ms tick/timeout for responsiveness.
3.  **Routing**: Events are routed to the focused panel or handled globally (e.g., `Ctrl+M` for model selector, `Tab` for focus cycle).
4.  **Mouse Handling**: Global mouse click handler switches focus between panels. Click events are also forwarded to the active panel for specific actions (e.g., clicking Editor tabs or Chat scrollbars).
5.  **Async Operations**: Long-running tasks (LLM requests) run in background threads and send results back via the `EventBus`.

### LLM Integration
*   **Provider**: `LlmProvider` trait (`src/llm/mod.rs`) defines the interface. Currently, `OllamaProvider` (`src/llm/ollama.rs`) is the sole implementation.
*   **Code Application**: The `ChatPanel` parses LLM responses. If a code block with a filename is found (e.g., ```rust:src/main.rs), it triggers an `Event::FileModification`.
*   **Diff View**: The `EditorPanel` receives the modification event and applies it using a diff view (green `+` for additions, yellow `~` for changes), allowing the user to review AI suggestions.

## Key Files & Directories

*   `src/main.rs`: Application entry point, terminal setup, and main event loop.
*   `src/state/app.rs`: Central `AppState` (focus, input mode, global flags).
*   `src/events/mod.rs`: Definition of the `Event` enum and `EventBus`.
*   `src/panels/`: Source code for individual UI panels.
    *   `src/panels/editor/diff.rs`: Logic for computing and displaying diffs.
*   `src/ui/model_selector.rs`: Implementation of the model selection modal.
*   `src/llm/ollama.rs`: Implementation of the Ollama API client.
*   `Cargo.toml`: Project dependencies (ratatui, tokio, portable-pty, etc.).

## Build & Run

*   **Build**: `cargo build`
*   **Run**: `cargo run`
*   **Release Build**: `cargo build --release`
*   **Test**: `cargo test`

## Development Conventions

*   **State**: Shared state is minimal in `AppState`. Panel-specific state should remain encapsulated within the panel struct.
*   **Concurrency**: Use the `EventBus` for all cross-thread communication. Do not block the main UI thread.
*   **Error Handling**: Use the `Result` type alias from `core::error` (powered by `thiserror`).
*   **Formatting**: Follow standard Rust formatting (`cargo fmt`).
