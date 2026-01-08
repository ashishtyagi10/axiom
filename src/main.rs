//! Axiom - Terminal IDE with AI integration
//!
//! Entry point with proper terminal setup and cleanup.

use axiom::{
    agents::{Conductor, Executor},
    config::{config_path, load_config, save_config, AxiomConfig},
    core::Result,
    events::{Event, EventBus},
    llm::{ClaudeProvider, GeminiProvider, OllamaProvider, ProviderRegistry},
    panels::{Panel, PanelRegistry},
    state::{AppState, OutputContext},
    ui::{self, settings::SettingsAction},
    watcher::FileWatcher,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;
use std::time::Duration;

/// Application entry point.
///
/// Sets up the terminal in raw mode, initializes the TUI backend,
/// runs the application loop, and ensures the terminal is restored
/// to its original state upon exit or panic.
fn main() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    // Run app with panic recovery
    let result = run_app(&mut term);

    // Restore terminal (ALWAYS, even on error)
    terminal::disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    // Print any error
    if let Err(ref e) = result {
        eprintln!("Error: {}", e);
    }

    result
}

/// Main application loop.
///
/// Initializes the application state, event bus, panels, and file watcher.
/// Handles the main event loop, rendering the UI and processing events
/// until a quit signal is received.
fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    // Initialize clipboard
    axiom::clipboard::init();

    // Create event bus with bounded channel
    let event_bus = EventBus::new(1024);

    // Create application state
    let mut state = AppState::new();

    // Load configuration
    let config = load_config(&state.cwd).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        AxiomConfig::default()
    });

    // Create provider registry from config
    let llm_registry = create_provider_registry(&config);

    // Store config for settings modal (mutable for reloading)
    let mut config = config;

    // Create panels
    let mut panels = PanelRegistry::new(event_bus.sender(), &state.cwd, llm_registry, &config)?;

    // Create conductor and executor
    let mut conductor = Conductor::new(panels.llm_registry.clone(), event_bus.sender());
    let executor = Executor::new(
        event_bus.sender(),
        panels.agent_registry.clone(),
        state.cwd.clone(),
    );

    // Start file watcher for the project directory
    let _file_watcher = FileWatcher::new(&state.cwd, event_bus.sender())
        .map_err(|e| axiom::core::AxiomError::Config(format!("File watcher error: {}", e)))?;

    // Spawn input reader thread
    spawn_input_reader(event_bus.sender());

    // Get initial terminal size and notify panels
    let size = terminal.size()?;
    let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
    let layout = ui::get_layout_with_focus(area, Some(state.focus.current()));
    panels.notify_resize_all(&layout);

    // Track screen area for layout calculations
    let mut screen_area = ratatui::layout::Rect::new(0, 0, size.width, size.height);

    // Main event loop
    loop {
        // Render
        terminal.draw(|frame| {
            ui::render(frame, &state, &mut panels);
        })?;

        // Process events with timeout (50ms for responsive UI)
        if let Some(event) = event_bus.recv_timeout(Duration::from_millis(50)) {
            if handle_event(
                &event,
                &mut state,
                &mut panels,
                screen_area,
                &mut config,
                &mut conductor,
                &executor,
            )? {
                break; // Quit requested
            }

            // Handle resize events
            if let Event::Resize(w, h) = event {
                screen_area = ratatui::layout::Rect::new(0, 0, w, h);
                let layout = ui::get_layout_with_focus(screen_area, Some(state.focus.current()));
                panels.notify_resize_all(&layout);
            }
        }

        // Drain additional events to prevent lag
        for event in event_bus.drain(50) {
            if handle_event(
                &event,
                &mut state,
                &mut panels,
                screen_area,
                &mut config,
                &mut conductor,
                &executor,
            )? {
                break;
            }
        }

        // Check if file tree wants to open a file (auto-open on selection)
        if let Some(path) = panels.file_tree.take_pending_open() {
            // Switch output context to show this file
            panels.set_output_context(OutputContext::File { path: path.clone() });
            // Don't switch focus - let user keep navigating file tree
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

/// Processes a single application event.
///
/// Handles global keybindings (like quitting or focus switching) and routes
/// specific events to the appropriate panels (Editor, Terminal, Chat, etc.).
///
/// Returns `Ok(true)` if the application should exit.
fn handle_event(
    event: &Event,
    state: &mut AppState,
    panels: &mut PanelRegistry,
    screen_area: ratatui::layout::Rect,
    config: &mut AxiomConfig,
    conductor: &mut Conductor,
    executor: &Executor,
) -> Result<bool> {
    match event {
        // Global key bindings (checked first)
        Event::Key(key) => {
            // Handle settings modal
            if state.input_mode.is_modal_open("settings") {
                match key.code {
                    KeyCode::Esc => {
                        if panels.settings.editing {
                            panels.settings.cancel_edit();
                        } else {
                            state.input_mode.to_normal();
                        }
                        return Ok(false);
                    }
                    KeyCode::Up => {
                        panels.settings.up();
                        return Ok(false);
                    }
                    KeyCode::Down => {
                        panels.settings.down();
                        return Ok(false);
                    }
                    KeyCode::Left => {
                        panels.settings.left();
                        return Ok(false);
                    }
                    KeyCode::Right => {
                        panels.settings.right();
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        match panels.settings.enter() {
                            SettingsAction::Save => {
                                if let Some(new_config) = panels.apply_settings() {
                                    // Save to file
                                    let path = config_path(&state.cwd);
                                    if let Err(e) = save_config(&new_config, &path) {
                                        state.error(format!("Failed to save settings: {}", e));
                                    } else {
                                        // Reload providers with new config
                                        reload_providers(panels, &new_config);
                                        *config = new_config;
                                        state.info("Settings saved");
                                    }
                                }
                                state.input_mode.to_normal();
                            }
                            SettingsAction::Cancel => {
                                state.input_mode.to_normal();
                            }
                            SettingsAction::StartEdit | SettingsAction::None => {
                                // Already handled internally
                            }
                        }
                        return Ok(false);
                    }
                    KeyCode::Char(c) if panels.settings.editing => {
                        panels.settings.insert_char(c);
                        return Ok(false);
                    }
                    KeyCode::Backspace if panels.settings.editing => {
                        panels.settings.backspace();
                        return Ok(false);
                    }
                    KeyCode::Delete if panels.settings.editing => {
                        panels.settings.delete();
                        return Ok(false);
                    }
                    _ => return Ok(false),
                }
            }

            // Handle model selector modal
            if state.input_mode.is_modal_open("model_selector") {
                match key.code {
                    KeyCode::Esc => {
                        state.input_mode.to_normal();
                        return Ok(false);
                    }
                    KeyCode::Up => {
                        panels.model_selector.up();
                        return Ok(false);
                    }
                    KeyCode::Down => {
                        panels.model_selector.down();
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        if let Some(model) = panels.apply_selected_model() {
                            state.info(format!("Model changed to: {}", model));
                        }
                        state.input_mode.to_normal();
                        return Ok(false);
                    }
                    _ => return Ok(false),
                }
            }

            // 'q' in Normal mode: Quit (vim-style, avoids Ctrl+Q terminal conflict)
            if key.code == KeyCode::Char('q') && !state.input_mode.is_editing() && !state.input_mode.is_modal() {
                state.quit();
                return Ok(true);
            }

            // Ctrl+M: Open model selector
            if key.code == KeyCode::Char('m') && key.modifiers.contains(KeyModifiers::CONTROL) {
                panels.open_model_selector();
                state.input_mode.open_modal("model_selector");
                return Ok(false);
            }

            // Ctrl+,: Open settings modal
            if key.code == KeyCode::Char(',') && key.modifiers.contains(KeyModifiers::CONTROL) {
                panels.open_settings(config);
                state.input_mode.open_modal("settings");
                return Ok(false);
            }

            // Tab (without modifiers): Cycle focus
            // Ctrl+Tab is reserved for panel-specific use (e.g., editor tabs)
            if key.code == KeyCode::Tab
                && key.modifiers.is_empty()
                && !state.input_mode.is_editing()
                && !state.input_mode.is_modal()
            {
                state.focus.next();
                panels.handle_focus_change(state.focus.current(), screen_area);
                return Ok(false);
            }

            // Backtab/Shift+Tab (without Ctrl): Cycle focus backwards
            // Ctrl+Shift+Tab is reserved for panel-specific use (e.g., editor tabs)
            if key.code == KeyCode::BackTab
                && !key.modifiers.contains(KeyModifiers::CONTROL)
                && !state.input_mode.is_editing()
                && !state.input_mode.is_modal()
            {
                state.focus.prev();
                panels.handle_focus_change(state.focus.current(), screen_area);
                return Ok(false);
            }

            // Escape: Return to normal mode
            if key.code == KeyCode::Esc {
                state.input_mode.to_normal();
                return Ok(false);
            }

            // 'i' in Normal mode: Enter insert mode
            if key.code == KeyCode::Char('i') && !state.input_mode.is_editing() && !state.input_mode.is_modal() {
                state.input_mode.to_insert();
                return Ok(false);
            }

            // Forward to focused panel
            let focused = state.focus.current();
            panels.get_mut(focused).handle_input(event, state)?;
        }

        // Mouse events
        Event::Mouse(mouse) => {
            let x = mouse.column;
            let y = mouse.row;

            // Handle settings modal mouse events
            if state.input_mode.is_modal_open("settings") {
                match mouse.kind {
                    event::MouseEventKind::Down(event::MouseButton::Left) => {
                        // Check if click is inside modal
                        if panels.settings.contains(x, y) {
                            // Handle click on settings items
                            match panels.settings.handle_click(x, y) {
                                SettingsAction::Save => {
                                    if let Some(new_config) = panels.apply_settings() {
                                        let path = config_path(&state.cwd);
                                        if let Err(e) = save_config(&new_config, &path) {
                                            state.error(format!("Failed to save settings: {}", e));
                                        } else {
                                            reload_providers(panels, &new_config);
                                            *config = new_config;
                                            state.info("Settings saved");
                                        }
                                    }
                                    state.input_mode.to_normal();
                                }
                                SettingsAction::Cancel => {
                                    state.input_mode.to_normal();
                                }
                                SettingsAction::StartEdit | SettingsAction::None => {
                                    // Row selected or edit started
                                }
                            }
                        } else {
                            // Click outside modal - close it
                            state.input_mode.to_normal();
                        }
                    }
                    event::MouseEventKind::ScrollUp => {
                        if panels.settings.contains(x, y) {
                            panels.settings.up();
                        }
                    }
                    event::MouseEventKind::ScrollDown => {
                        if panels.settings.contains(x, y) {
                            panels.settings.down();
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }

            // Handle model selector modal mouse events
            if state.input_mode.is_modal_open("model_selector") {
                match mouse.kind {
                    event::MouseEventKind::Down(event::MouseButton::Left) => {
                        // Check if click is inside modal
                        if panels.model_selector.contains(x, y) {
                            // Check if click is on a model item
                            if panels.model_selector.handle_click(x, y) {
                                // Item was clicked, apply selection
                                if let Some(model) = panels.apply_selected_model() {
                                    state.info(format!("Model: {}", model));
                                }
                                state.input_mode.to_normal();
                            }
                        } else {
                            // Click outside modal - close it
                            state.input_mode.to_normal();
                        }
                    }
                    event::MouseEventKind::ScrollUp => {
                        if panels.model_selector.contains(x, y) {
                            panels.model_selector.handle_scroll(false);
                        }
                    }
                    event::MouseEventKind::ScrollDown => {
                        if panels.model_selector.contains(x, y) {
                            panels.model_selector.handle_scroll(true);
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }

            // Handle click on model badge in status bar
            if let event::MouseEventKind::Down(event::MouseButton::Left) = mouse.kind {
                if panels.is_model_badge_click(x, y) {
                    panels.open_model_selector();
                    state.input_mode.open_modal("model_selector");
                    return Ok(false);
                }
            }

            // Get current layout to determine panel areas
            let layout = ui::get_layout_with_focus(screen_area, Some(state.focus.current()));

            // Handle mouse click to focus panel AND forward click to panel
            if let event::MouseEventKind::Down(event::MouseButton::Left) = mouse.kind {
                if let Some(panel_id) = layout.panel_at(x, y) {
                    // Focus the panel if not already focused
                    if panel_id != state.focus.current() {
                        state.focus.focus(panel_id);
                        // Reset to normal mode when switching panels via mouse click
                        // This ensures Tab works after clicking to focus a panel
                        state.input_mode.to_normal();
                        // Call handle_focus_change like Tab does (notifies all panels, recalculates layout)
                        panels.handle_focus_change(state.focus.current(), screen_area);
                    }
                    // Forward the click event to the panel for handling (e.g., tab clicks, file selection)
                    panels.get_mut(panel_id).handle_input(event, state)?;
                }
            }

            // Handle mouse scroll in panels
            match mouse.kind {
                event::MouseEventKind::ScrollUp => {
                    if let Some(panel_id) = layout.panel_at(x, y) {
                        // Create a scroll event for the panel
                        let scroll_event = Event::Mouse(crossterm::event::MouseEvent {
                            kind: event::MouseEventKind::ScrollUp,
                            column: x,
                            row: y,
                            modifiers: mouse.modifiers,
                        });
                        panels.get_mut(panel_id).handle_input(&scroll_event, state)?;
                    }
                }
                event::MouseEventKind::ScrollDown => {
                    if let Some(panel_id) = layout.panel_at(x, y) {
                        let scroll_event = Event::Mouse(crossterm::event::MouseEvent {
                            kind: event::MouseEventKind::ScrollDown,
                            column: x,
                            row: y,
                            modifiers: mouse.modifiers,
                        });
                        panels.get_mut(panel_id).handle_input(&scroll_event, state)?;
                    }
                }
                _ => {}
            }
        }

        // PTY output - currently disabled (will be handled by shell agent)
        Event::PtyOutput(_) | Event::PtyExit(_) => {
            // TODO: Route to shell agent when implemented
        }

        // LLM events - currently disabled (will be handled by conductor)
        Event::LlmChunk(_) | Event::LlmDone | Event::LlmError(_) => {
            // TODO: Route to conductor when implemented
        }

        // New agent events
        Event::ConductorRequest(ref text) => {
            conductor.process(text.clone());
            // Switch to agent output view
            let registry = panels.agent_registry.read();
            if let Some(agent_id) = registry.selected_id() {
                drop(registry);
                panels.set_output_context(OutputContext::Agent { agent_id });
            }
        }

        Event::AgentSpawn(ref request) => {
            // Spawn the agent in registry
            let agent_id = {
                let mut registry = panels.agent_registry.write();
                registry.spawn(request.clone())
            };

            // Execute non-conductor agents
            if request.agent_type != axiom::agents::AgentType::Conductor {
                executor.execute(agent_id, request);
            } else {
                // Store the persistent conductor agent ID
                conductor.set_agent_id(agent_id);
                // Conductor handles its own execution
                conductor.execute(agent_id, request.parameters.as_deref().unwrap_or(""));
            }

            // Only switch context for top-level agents (no parent)
            // Child agents are shown in the Conductor's aggregated view
            if request.parent_id.is_none() {
                panels.set_output_context(OutputContext::Agent { agent_id });
            }
        }

        Event::AgentUpdate { id, ref status } => {
            let mut registry = panels.agent_registry.write();
            if let Some(agent) = registry.get_mut(*id) {
                agent.status = status.clone();
            }
        }

        Event::AgentOutput { id, ref chunk } => {
            let mut registry = panels.agent_registry.write();
            registry.append_output(*id, chunk);
        }

        Event::AgentComplete { id } => {
            let mut registry = panels.agent_registry.write();
            registry.complete(*id);
        }

        Event::AgentWake(id) => {
            // Wake an idle agent (used for persistent Conductor)
            let mut registry = panels.agent_registry.write();
            if let Some(agent) = registry.get_mut(*id) {
                agent.status = axiom::agents::AgentStatus::Running;
            }
            drop(registry);
            // Switch context to show the conductor
            panels.set_output_context(OutputContext::Agent { agent_id: *id });
        }

        Event::SwitchContext(ref context) => {
            panels.set_output_context(context.clone());
        }

        Event::ShellExecute(ref cmd) => {
            // Spawn shell agent (no parent - direct shell command)
            let request = axiom::agents::AgentSpawnRequest {
                agent_type: axiom::agents::AgentType::Shell,
                name: "Shell".to_string(),
                description: truncate_cmd(cmd, 50),
                parameters: Some(cmd.clone()),
                parent_id: None,
            };
            let agent_id = {
                let mut registry = panels.agent_registry.write();
                registry.spawn(request.clone())
            };
            executor.execute(agent_id, &request);
            panels.set_output_context(OutputContext::Agent { agent_id });
        }

        // File modification from LLM - for now just log (will be handled by coder agent)
        Event::FileModification { ref path, ref content } => {
            // TODO: Route to coder agent when implemented
            // For now, write the file directly
            let file_path = std::path::PathBuf::from(path);
            let resolved_path = if file_path.is_absolute() {
                file_path
            } else {
                state.cwd.join(&file_path)
            };

            if let Err(e) = std::fs::write(&resolved_path, content) {
                state.error(format!("Failed to write {}: {}", path, e));
            } else {
                state.info(format!("Modified: {}", path));
                // Switch to show the modified file
                panels.set_output_context(OutputContext::File { path: resolved_path });
            }
        }

        // File changed on disk (detected by file watcher)
        Event::FileChanged(ref path) => {
            // If currently viewing this file, refresh the view
            if let OutputContext::File { path: current_path } = panels.output_context() {
                if current_path == path {
                    // Re-set the context to trigger a reload
                    panels.set_output_context(OutputContext::File { path: path.clone() });
                    state.info(format!("Reloaded: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                }
            }
        }

        // Resize is handled in main loop
        Event::Resize(_, _) => {}

        // Tick - could be used for animations
        Event::Tick => {}

        // Other events
        _ => {}
    }

    Ok(false)
}

/// Spawns a dedicated thread to read input events (keyboard, mouse, resize).
///
/// Events are sent to the main loop via the provided channel.
/// The thread polls for events with a timeout to allow for clean shutdown.
fn spawn_input_reader(tx: crossbeam_channel::Sender<Event>) {
    std::thread::spawn(move || {
        loop {
            // Poll with timeout to allow thread shutdown
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                match event::read() {
                    Ok(event::Event::Key(key)) => {
                        if tx.send(Event::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(event::Event::Mouse(mouse)) => {
                        if tx.send(Event::Mouse(mouse)).is_err() {
                            break;
                        }
                    }
                    Ok(event::Event::Resize(w, h)) => {
                        if tx.send(Event::Resize(w, h)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    });
}

/// Creates the LLM provider registry based on configuration.
///
/// Registers all enabled providers (Ollama, Claude, Gemini) and sets
/// the default active provider.
fn create_provider_registry(config: &AxiomConfig) -> ProviderRegistry {
    let mut registry = ProviderRegistry::from_config(config);

    // Register Ollama provider (always available, uses local server)
    if let Some(ollama_config) = config.get_provider("ollama") {
        if ollama_config.enabled {
            let base_url = ollama_config
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let model = ollama_config
                .default_model
                .as_deref()
                .unwrap_or("gemma3:4b");
            registry.register(Arc::new(OllamaProvider::new(base_url, model)));
        }
    } else {
        // Register with defaults if not in config
        registry.register(Arc::new(OllamaProvider::default()));
    }

    // Register Claude provider if API key is available
    if let Some(claude_config) = config.get_provider("claude") {
        if claude_config.enabled {
            if let Some(ref api_key) = claude_config.api_key {
                if !api_key.is_empty() && !api_key.starts_with("${") {
                    let model = claude_config
                        .default_model
                        .as_deref()
                        .unwrap_or("claude-sonnet-4-20250514");
                    registry.register(Arc::new(ClaudeProvider::new(api_key, model)));
                }
            }
        }
    }

    // Register Gemini provider if API key is available
    if let Some(gemini_config) = config.get_provider("gemini") {
        if gemini_config.enabled {
            if let Some(ref api_key) = gemini_config.api_key {
                if !api_key.is_empty() && !api_key.starts_with("${") {
                    let model = gemini_config
                        .default_model
                        .as_deref()
                        .unwrap_or("gemini-2.0-flash");
                    registry.register(Arc::new(GeminiProvider::new(api_key, model)));
                }
            }
        }
    }

    // Set the default provider as active
    let _ = registry.set_active(&config.llm.default_provider);

    // Fallback to ollama if default provider is not available
    if registry.active().is_none() {
        let _ = registry.set_active("ollama");
    }

    registry
}

/// Reloads LLM providers with a new configuration.
///
/// Recreates the provider registry for use by the conductor.
fn reload_providers(panels: &mut PanelRegistry, config: &AxiomConfig) {
    // Create new provider registry
    let new_registry = create_provider_registry(config);

    // Replace the registry
    *panels.llm_registry.write() = new_registry;

    // Provider selection will be handled by conductor when implemented
}

/// Checks if a given path corresponds to a source code file.
///
/// Determines if a file should be automatically opened in the editor
/// based on its file extension or specific filename (e.g., Dockerfile, Makefile).
fn is_source_file(path: &std::path::Path) -> bool {
    let source_extensions = [
        "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp",
        "rb", "php", "swift", "kt", "scala", "cs", "fs", "hs", "ml", "ex", "exs",
        "clj", "lisp", "scm", "lua", "r", "jl", "nim", "zig", "v", "d",
        "html", "css", "scss", "sass", "less", "vue", "svelte",
        "json", "yaml", "yml", "toml", "xml", "md", "txt",
        "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd",
        "sql", "graphql", "proto",
        "dockerfile", "makefile", "cmake",
    ];

    // Check extension
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        if source_extensions.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // Check filename (for files without extensions)
    if let Some(name) = path.file_name() {
        let name_lower = name.to_string_lossy().to_lowercase();
        let special_files = [
            "dockerfile", "makefile", "cmakelists.txt", "cargo.toml",
            "package.json", "tsconfig.json", "pyproject.toml",
            "gemfile", "rakefile", "justfile",
        ];
        if special_files.contains(&name_lower.as_str()) {
            return true;
        }
    }

    false
}

/// Truncate a command string for display
fn truncate_cmd(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
