//! Axiom - Terminal IDE with AI integration
//!
//! Entry point with proper terminal setup and cleanup.
//!
//! Run modes:
//! - `cargo run` - TUI in same terminal (default)
//! - `cargo run -n` - TUI in new terminal window
//! - `cargo run --web` - Start web server and open browser

use axiom::{
    agents::{Conductor, Executor, PtyAgentManager},
    config::{config_path, load_config, save_config, AxiomConfig},
    core::Result,
    events::{Event, EventBus},
    llm::{ClaudeProvider, GeminiProvider, OllamaProvider, ProviderRegistry},
    panels::PanelRegistry,
    state::{AppState, OutputContext, PanelId, WorkspaceId},
    ui::{self, settings::SettingsAction, workspace_selector::WorkspaceSelectorAction, SelectorMode, toggle_theme, current_variant},
    watcher::FileWatcher,
};
use axiom_core::{
    ModelSubcommand, SlashCommand, SlashCommandData, SlashCommandParser, SlashCommandResult,
    ThemeSubcommand, UiAction, WorkspaceSubcommand,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Command-line arguments
struct Args {
    /// Directory path to open
    path: Option<PathBuf>,
    /// Workspace ID to open
    workspace: Option<String>,
    /// Run in web mode (start server and open browser)
    web: bool,
    /// Open TUI in a new terminal window
    new_window: bool,
}

impl Args {
    /// Parse command-line arguments
    fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut path = None;
        let mut workspace = None;
        let mut web = false;
        let mut new_window = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--web" | "-W" => {
                    web = true;
                }
                "-n" | "--new-window" => {
                    new_window = true;
                }
                "--workspace" | "-w" => {
                    workspace = args.next();
                }
                _ if !arg.starts_with('-') => {
                    // Treat as path
                    path = Some(PathBuf::from(arg));
                }
                _ => {
                    // Ignore unknown flags
                }
            }
        }

        Self { path, workspace, web, new_window }
    }
}

/// Application entry point.
///
/// Determines the run mode based on arguments:
/// - `--web` / `-W`: Start web server and open browser
/// - `-n` / `--new-window`: Spawn TUI in a new terminal window
/// - Default: Run TUI in current terminal
fn main() -> Result<()> {
    // If AXIOM_TUI is set, we were spawned by -n flag - run TUI directly
    if std::env::var("AXIOM_TUI").is_ok() {
        return run_tui();
    }

    let args = Args::parse();

    if args.web {
        return run_web(args);
    }

    if args.new_window {
        return spawn_in_new_terminal();
    }

    // Default: run TUI in current terminal
    run_tui()
}

/// Spawns the TUI in a new terminal window (used with `-n` flag).
///
/// Sets `AXIOM_TUI=1` so the spawned process runs TUI directly.
///
/// Platform-specific implementations:
/// - **macOS**: Uses `osascript` to open Terminal.app
/// - **Linux**: Tries common terminal emulators (gnome-terminal, konsole, etc.)
/// - **Windows**: Uses Windows Terminal or falls back to cmd.exe
fn spawn_in_new_terminal() -> Result<()> {
    let exe = std::env::current_exe()
        .map_err(|e| axiom::core::AxiomError::Config(format!("Failed to get executable path: {}", e)))?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_str = args.join(" ");

    #[cfg(target_os = "macos")]
    {
        // Use osascript to open Terminal.app with the command
        let script = format!(
            r#"tell application "Terminal"
                activate
                do script "AXIOM_TUI=1 '{}' {}"
            end tell"#,
            exe.display(),
            args_str
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .map_err(|e| axiom::core::AxiomError::Config(format!("Failed to spawn terminal: {}", e)))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // Try common terminal emulators in order
        let terminals = ["gnome-terminal", "konsole", "xfce4-terminal", "xterm"];
        let cmd = format!("AXIOM_TUI=1 '{}' {}", exe.display(), args_str);

        for term in terminals {
            let result = match term {
                "gnome-terminal" => std::process::Command::new(term)
                    .arg("--")
                    .arg("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .spawn(),
                "konsole" | "xfce4-terminal" => std::process::Command::new(term)
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .spawn(),
                _ => std::process::Command::new(term)
                    .arg("-e")
                    .arg(&cmd)
                    .spawn(),
            };
            if result.is_ok() {
                return Ok(());
            }
        }
        return Err(axiom::core::AxiomError::Config(
            "No supported terminal emulator found".into(),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        // Use Windows Terminal if available, fall back to cmd
        let cmd = format!("set AXIOM_TUI=1 && \"{}\" {}", exe.display(), args_str);

        // Try Windows Terminal first
        if std::process::Command::new("wt")
            .arg("cmd")
            .arg("/c")
            .arg(&cmd)
            .spawn()
            .is_err()
        {
            // Fall back to cmd.exe
            std::process::Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("cmd")
                .arg("/c")
                .arg(&cmd)
                .spawn()
                .map_err(|e| axiom::core::AxiomError::Config(format!("Failed to spawn terminal: {}", e)))?;
        }
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return Err(axiom::core::AxiomError::Config(
            "Unsupported platform for auto-launch. Use AXIOM_TUI=1 to run directly.".into(),
        ));
    }
}

/// Runs both the API server and web UI, then opens the browser.
///
/// Starts:
/// - Axiom API server on port 8080
/// - Next.js UI server on port 3000
/// - Opens browser to http://localhost:3000
#[cfg(feature = "web")]
fn run_web(_args: Args) -> Result<()> {
    use std::sync::Mutex;

    let api_port: u16 = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let ui_port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    // Kill any existing process on UI port (cleanup from previous run)
    kill_process_on_port(ui_port);

    // Brief pause to allow port release
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Start Next.js UI server
    let web_dir = std::path::Path::new("web");
    let standalone_dir = web_dir.join(".next/standalone");

    if !standalone_dir.exists() {
        eprintln!("Web UI not built. Building...");
        let status = std::process::Command::new("npm")
            .arg("run")
            .arg("build")
            .current_dir(web_dir)
            .status()
            .map_err(|e| axiom::core::AxiomError::Config(format!("Failed to build web UI: {}", e)))?;

        if !status.success() {
            return Err(axiom::core::AxiomError::Config("Failed to build web UI".into()));
        }
    }

    // Copy static files to standalone (Next.js requirement)
    let static_src = web_dir.join(".next/static");
    let static_dst = standalone_dir.join(".next/static");
    if static_src.exists() && !static_dst.exists() {
        let _ = std::fs::create_dir_all(static_dst.parent().unwrap());
        copy_dir_recursive(&static_src, &static_dst)?;
    }

    // Copy public folder if it exists
    let public_src = web_dir.join("public");
    let public_dst = standalone_dir.join("public");
    if public_src.exists() && !public_dst.exists() {
        copy_dir_recursive(&public_src, &public_dst)?;
    }

    // Start Next.js server
    let next_server = std::process::Command::new("node")
        .arg("server.js")
        .current_dir(&standalone_dir)
        .env("PORT", ui_port.to_string())
        .env("HOSTNAME", "0.0.0.0")
        .spawn()
        .map_err(|e| axiom::core::AxiomError::Config(format!("Failed to start UI server: {}", e)))?;

    // Use Arc to share next_server handle for cleanup
    let next_server = Arc::new(Mutex::new(Some(next_server)));
    let next_server_clone = next_server.clone();

    // Register Ctrl+C handler for clean shutdown
    ctrlc::set_handler(move || {
        if let Ok(mut guard) = next_server_clone.lock() {
            if let Some(mut server) = guard.take() {
                let _ = server.kill();
            }
        }
        std::process::exit(0);
    })
    .ok();

    println!("Starting Axiom UI at http://localhost:{}", ui_port);
    println!("Starting Axiom API at http://localhost:{}", api_port);
    println!("Press Ctrl+C to stop");

    // Create runtime and run API server
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| axiom::core::AxiomError::Config(format!("Runtime error: {}", e)))?;

    let result = rt.block_on(async {
        // Open browser after servers start
        let browser_url = format!("http://localhost:{}", ui_port);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            if let Err(e) = open::that(&browser_url) {
                eprintln!("Failed to open browser: {}", e);
            }
        });

        axiom_server::run_server(api_port)
            .await
            .map_err(|e| axiom::core::AxiomError::Config(e.to_string()))
    });

    // Clean up Next.js server
    if let Ok(mut guard) = next_server.lock() {
        if let Some(mut server) = guard.take() {
            let _ = server.kill();
        }
    }

    result
}

/// Kill any process using the specified port (macOS/Linux)
#[cfg(feature = "web")]
fn kill_process_on_port(port: u16) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("lsof -ti:{} | xargs kill -9 2>/dev/null", port))
            .status();
    }
}

/// Recursively copy a directory
#[cfg(feature = "web")]
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Fallback when web feature is not enabled.
#[cfg(not(feature = "web"))]
fn run_web(_args: Args) -> Result<()> {
    eprintln!("Error: Web mode not available.");
    eprintln!("Rebuild with: cargo build --features web");
    std::process::exit(1);
}

/// Runs the TUI application.
///
/// Sets up the terminal in raw mode, initializes the TUI backend,
/// runs the application loop, and ensures the terminal is restored
/// to its original state upon exit or panic.
fn run_tui() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    // Run app with panic recovery
    let result = run_app(&mut term, args);

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
fn run_app<B: Backend>(terminal: &mut Terminal<B>, args: Args) -> Result<()> {
    // Initialize clipboard
    axiom::clipboard::init();

    // Create event bus with bounded channel
    let event_bus = EventBus::new(1024);

    // Create application state with optional path from args
    let mut state = if let Some(path) = args.path {
        // Resolve to absolute path
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join(path)
        };

        // Verify path exists and is a directory
        if path.is_dir() {
            AppState::with_cwd(path)
        } else {
            eprintln!("Warning: {} is not a directory, using current directory", path.display());
            AppState::new()
        }
    } else {
        AppState::new()
    };

    // Load configuration
    let config = load_config(&state.cwd).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        AxiomConfig::default()
    });

    // Initialize workspace manager
    if let Err(e) = state.init_workspace_manager(axiom_core::AxiomConfig::default()) {
        eprintln!("Warning: Failed to initialize workspace manager: {}", e);
    }

    // If --workspace argument provided, try to switch to it
    if let Some(workspace_id_str) = args.workspace {
        if let Ok(ws_id) = workspace_id_str.parse::<WorkspaceId>() {
            match state.switch_workspace(ws_id) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Warning: Failed to switch to workspace {}: {}", workspace_id_str, e);
                }
            }
        } else {
            eprintln!("Warning: Invalid workspace ID: {}", workspace_id_str);
        }
    } else if let Some(manager) = &state.workspace_manager {
        // Try to find existing workspace for cwd or create/detect one
        if state.active_workspace_id.is_none() {
            // Check if there's an existing workspace for this path
            if let Some(workspace) = manager.find_by_path(&state.cwd) {
                state.active_workspace_id = Some(workspace.id);
            }
            // Note: We don't auto-create workspaces - user should explicitly create them
        }
    }

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

    // Create PTY agent manager for CLI agents (wrapped in Arc<RwLock> for sharing)
    let pty_manager = Arc::new(parking_lot::RwLock::new(PtyAgentManager::new(event_bus.sender())));

    // Give OutputPanel access to PTY manager for CLI agent rendering
    panels.output.set_pty_manager(pty_manager.clone(), event_bus.sender());

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

    // Initialize workspace selector with existing workspaces
    // (it will be shown as main screen until a workspace is selected)
    if let Some(manager) = &state.workspace_manager {
        let workspaces = manager.list_workspaces();
        panels.open_workspace_selector(workspaces, state.active_workspace_id);
    }

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
                &pty_manager,
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
                &pty_manager,
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
    pty_manager: &Arc<parking_lot::RwLock<PtyAgentManager>>,
) -> Result<bool> {
    // Workspace selection phase: only handle workspace selector events and resize
    if state.active_workspace_id.is_none() {
        match event {
            Event::Key(key) => {
                let is_creating = panels.workspace_selector.mode == SelectorMode::CreateNew;
                let is_browsing = panels.workspace_selector.mode == SelectorMode::BrowseFolders;

                // Handle BrowseFolders mode
                if is_browsing {
                    match key.code {
                        KeyCode::Esc => {
                            panels.workspace_selector.cancel_browse();
                            return Ok(false);
                        }
                        KeyCode::Up => {
                            panels.workspace_selector.up();
                            return Ok(false);
                        }
                        KeyCode::Down => {
                            panels.workspace_selector.down();
                            return Ok(false);
                        }
                        KeyCode::Enter => {
                            panels.workspace_selector.folder_enter();
                            return Ok(false);
                        }
                        KeyCode::Backspace => {
                            panels.workspace_selector.folder_parent();
                            return Ok(false);
                        }
                        _ => return Ok(false),
                    }
                }

                match key.code {
                    // No Esc to dismiss - must select a workspace
                    KeyCode::Up => {
                        panels.workspace_selector.up();
                    }
                    KeyCode::Down => {
                        panels.workspace_selector.down();
                    }
                    KeyCode::Tab if is_creating => {
                        panels.workspace_selector.down(); // Tab switches fields in create mode
                    }
                    // Ctrl+B in CreateNew mode: open folder browser
                    KeyCode::Char('b') if is_creating && key.modifiers.contains(KeyModifiers::CONTROL) => {
                        panels.workspace_selector.start_folder_browse();
                    }
                    KeyCode::Enter => {
                        let action = panels.workspace_selector.enter();
                        handle_workspace_selector_action_initial(action, state, panels, pty_manager);
                    }
                    KeyCode::Delete => {
                        panels.workspace_selector.delete();
                    }
                    KeyCode::Char('y') if panels.workspace_selector.mode == SelectorMode::ConfirmDelete => {
                        let action = panels.workspace_selector.confirm();
                        handle_workspace_selector_action_initial(action, state, panels, pty_manager);
                    }
                    KeyCode::Char('n') if panels.workspace_selector.mode == SelectorMode::ConfirmDelete => {
                        panels.workspace_selector.deny();
                    }
                    KeyCode::Char(c) if is_creating => {
                        panels.workspace_selector.insert_char(c);
                    }
                    KeyCode::Backspace if is_creating => {
                        panels.workspace_selector.backspace();
                    }
                    KeyCode::Left if is_creating => {
                        panels.workspace_selector.cursor_left();
                    }
                    KeyCode::Right if is_creating => {
                        panels.workspace_selector.cursor_right();
                    }
                    _ => {}
                }
                return Ok(false);
            }
            Event::Resize(w, h) => {
                // Handle resize during workspace selection
                let _ = (w, h); // Layout will be recalculated on next render
                return Ok(false);
            }
            _ => return Ok(false), // Ignore all other events during workspace selection
        }
    }

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

            // Handle workspace selector modal
            if state.input_mode.is_modal_open("workspace_selector") {
                let is_creating = panels.workspace_selector.mode == SelectorMode::CreateNew;
                let is_browsing = panels.workspace_selector.mode == SelectorMode::BrowseFolders;

                // Handle BrowseFolders mode
                if is_browsing {
                    match key.code {
                        KeyCode::Esc => {
                            panels.workspace_selector.cancel_browse();
                            return Ok(false);
                        }
                        KeyCode::Up => {
                            panels.workspace_selector.up();
                            return Ok(false);
                        }
                        KeyCode::Down => {
                            panels.workspace_selector.down();
                            return Ok(false);
                        }
                        KeyCode::Enter => {
                            panels.workspace_selector.folder_enter();
                            return Ok(false);
                        }
                        KeyCode::Backspace => {
                            panels.workspace_selector.folder_parent();
                            return Ok(false);
                        }
                        _ => return Ok(false),
                    }
                }

                match key.code {
                    KeyCode::Esc => {
                        let action = panels.workspace_selector.escape();
                        if action == WorkspaceSelectorAction::Cancel {
                            state.input_mode.to_normal();
                        }
                        return Ok(false);
                    }
                    KeyCode::Up => {
                        panels.workspace_selector.up();
                        return Ok(false);
                    }
                    KeyCode::Down => {
                        panels.workspace_selector.down();
                        return Ok(false);
                    }
                    KeyCode::Tab if is_creating => {
                        panels.workspace_selector.down(); // Tab switches fields in create mode
                        return Ok(false);
                    }
                    // Ctrl+B in CreateNew mode: open folder browser
                    KeyCode::Char('b') if is_creating && key.modifiers.contains(KeyModifiers::CONTROL) => {
                        panels.workspace_selector.start_folder_browse();
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        let action = panels.workspace_selector.enter();
                        handle_workspace_selector_action(action, state, panels, pty_manager);
                        return Ok(false);
                    }
                    KeyCode::Delete => {
                        panels.workspace_selector.delete();
                        return Ok(false);
                    }
                    KeyCode::Char('y') if panels.workspace_selector.mode == SelectorMode::ConfirmDelete => {
                        let action = panels.workspace_selector.confirm();
                        handle_workspace_selector_action(action, state, panels, pty_manager);
                        return Ok(false);
                    }
                    KeyCode::Char('n') if panels.workspace_selector.mode == SelectorMode::ConfirmDelete => {
                        panels.workspace_selector.deny();
                        return Ok(false);
                    }
                    KeyCode::Char(c) if is_creating => {
                        panels.workspace_selector.insert_char(c);
                        return Ok(false);
                    }
                    KeyCode::Backspace if is_creating => {
                        panels.workspace_selector.backspace();
                        return Ok(false);
                    }
                    KeyCode::Left if is_creating => {
                        panels.workspace_selector.cursor_left();
                        return Ok(false);
                    }
                    KeyCode::Right if is_creating => {
                        panels.workspace_selector.cursor_right();
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

            // Ctrl+T: Toggle theme
            if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
                toggle_theme();
                state.info(format!("Theme: {}", current_variant().as_str()));
                return Ok(false);
            }

            // Ctrl+W: Open workspace selector
            if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if let Some(manager) = &state.workspace_manager {
                    let workspaces = manager.list_workspaces();
                    panels.open_workspace_selector(workspaces, state.active_workspace_id);
                    state.input_mode.open_modal("workspace_selector");
                } else {
                    state.error("Workspace manager not initialized");
                }
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

            // 'i' in Normal mode: Enter insert mode (but not if Input panel is focused - let it type directly)
            if key.code == KeyCode::Char('i')
                && !state.input_mode.is_editing()
                && !state.input_mode.is_modal()
                && state.focus.current() != PanelId::INPUT
            {
                state.input_mode.to_insert();
                return Ok(false);
            }

            // Auto-enter insert mode when Input panel is focused and user types
            if state.focus.current() == PanelId::INPUT && !state.input_mode.is_editing() {
                state.input_mode.to_insert();
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

                // Focus Input panel for typing more prompts to Conductor
                state.focus.focus(PanelId::INPUT);
                panels.handle_focus_change(PanelId::INPUT, screen_area);
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

        Event::ConductorResponse(ref response) => {
            // Add assistant response to conductor history for LLM context
            conductor.add_response(response.clone());
        }

        Event::AgentWake(id) => {
            // Wake an idle agent (used for persistent Conductor)
            let mut registry = panels.agent_registry.write();

            // Remove old child agents from previous interaction
            registry.remove_children(*id);

            if let Some(agent) = registry.get_mut(*id) {
                // Keep output history - displays as Q&A pairs (ChatGPT style)
                // Each new interaction appends: question + answer
                agent.status = axiom::agents::AgentStatus::Running;
            }
            drop(registry);
            panels.set_output_context(OutputContext::Agent { agent_id: *id });
        }

        Event::SwitchContext(ref context) => {
            panels.set_output_context(context.clone());

            // Smart focus: determine where to focus based on context type
            match context {
                OutputContext::Agent { agent_id } => {
                    let registry = panels.agent_registry.read();
                    if let Some(agent) = registry.get(*agent_id) {
                        let is_cli_agent = agent.agent_type.is_cli_agent();
                        drop(registry);

                        if is_cli_agent {
                            // CLI agent: focus Output panel for PTY interaction
                            state.focus.focus(PanelId::OUTPUT);
                            panels.handle_focus_change(PanelId::OUTPUT, screen_area);
                        } else {
                            // Conductor/other agents: focus Input panel for typing prompts
                            state.focus.focus(PanelId::INPUT);
                            panels.handle_focus_change(PanelId::INPUT, screen_area);
                        }
                    }
                }
                OutputContext::File { .. } | OutputContext::Empty => {
                    // File viewing is passive - focus Input for commands
                    state.focus.focus(PanelId::INPUT);
                    panels.handle_focus_change(PanelId::INPUT, screen_area);
                }
            }
        }

        Event::FocusPanel(panel_id) => {
            state.focus.focus(*panel_id);
            panels.handle_focus_change(*panel_id, screen_area);
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

        // ===== CLI Agent Events =====

        Event::CliAgentInvoke { ref agent_id, ref prompt } => {
            // Get CLI agent config
            if let Some(cli_config) = config.cli_agents.get(agent_id) {
                // Spawn agent in registry
                let request = axiom::agents::AgentSpawnRequest {
                    agent_type: axiom::agents::AgentType::CliAgent {
                        config_id: agent_id.clone(),
                    },
                    name: cli_config.name.clone(),
                    description: truncate_cmd(prompt, 50),
                    parameters: Some(prompt.clone()),
                    parent_id: None,
                };
                let runtime_id = {
                    let mut registry = panels.agent_registry.write();
                    registry.spawn(request)
                };

                // Start PTY session
                let mut manager = pty_manager.write();
                if let Err(e) = manager.start(runtime_id, cli_config, prompt, &state.cwd) {
                    drop(manager);
                    state.error(format!("Failed to start {}: {}", cli_config.name, e));
                    let mut registry = panels.agent_registry.write();
                    registry.error(runtime_id, e.to_string());
                } else {
                    drop(manager);
                    // Mark as running and switch to output view
                    let mut registry = panels.agent_registry.write();
                    registry.start(runtime_id);
                    drop(registry);
                    panels.set_output_context(OutputContext::Agent { agent_id: runtime_id });

                    // Focus Output panel for PTY interaction
                    state.focus.focus(PanelId::OUTPUT);
                    panels.handle_focus_change(PanelId::OUTPUT, screen_area);

                    state.info(format!("Started {} agent", cli_config.name));
                }
            } else {
                state.error(format!("Unknown CLI agent: {}", agent_id));
            }
        }

        Event::CliAgentOutput { id, ref data } => {
            // Output is already processed by the PTY parser - no need to store separately
            // The OutputPanel will render directly from the PTY manager's screen
            // But we still track line count for status
            let mut registry = panels.agent_registry.write();
            if let Some(agent) = registry.get_mut(*id) {
                agent.line_count += data.iter().filter(|&&b| b == b'\n').count();
            }
        }

        Event::CliAgentExit { id, exit_code } => {
            // Mark agent as complete
            pty_manager.write().mark_exited(*id);
            let mut registry = panels.agent_registry.write();
            if *exit_code == 0 {
                registry.complete(*id);
            } else {
                registry.error(*id, format!("Exited with code {}", exit_code));
            }
        }

        Event::CliAgentInput { id, ref data } => {
            // Forward input to PTY
            if let Err(e) = pty_manager.write().write(*id, data) {
                state.error(format!("Failed to write to CLI agent: {}", e));
            }
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

        // ===== Workspace Events =====

        Event::WorkspaceSwitch(id) => {
            // Attempt to switch workspace
            match state.switch_workspace(*id) {
                Ok(new_path) => {
                    // Cancel all running agents before switching
                    cancel_all_agents(panels, pty_manager);

                    // Update panels
                    panels.handle_workspace_switch(&new_path);

                    // Reload workspace-specific config if available
                    if let Some(manager) = &state.workspace_manager {
                        if let Ok(ws_config) = manager.get_workspace_config(*id) {
                            // Merge workspace CLI agents with global config
                            // For now, we just use global config
                            let _ = ws_config;
                        }
                    }

                    state.info(format!("Switched to: {}", state.workspace_name()));
                }
                Err(e) => {
                    state.error(format!("Failed to switch workspace: {}", e));
                }
            }
        }

        Event::WorkspaceCreate { ref name, ref path } => {
            if let Some(manager) = &state.workspace_manager {
                match manager.create_workspace(name, path.clone()) {
                    Ok(workspace) => {
                        state.info(format!("Created workspace: {}", workspace.name));
                        // Optionally switch to the new workspace
                        let _ = state.switch_workspace(workspace.id);
                        panels.handle_workspace_switch(&workspace.path);
                    }
                    Err(e) => {
                        state.error(format!("Failed to create workspace: {}", e));
                    }
                }
            } else {
                state.error("Workspace manager not initialized");
            }
        }

        Event::WorkspaceDelete(id) => {
            if let Some(manager) = &state.workspace_manager {
                match manager.delete_workspace(*id) {
                    Ok(Some(deleted)) => {
                        state.info(format!("Deleted workspace: {}", deleted.name));
                        // If we deleted the active workspace, clear the active ID
                        if state.active_workspace_id == Some(*id) {
                            state.active_workspace_id = None;
                        }
                    }
                    Ok(None) => {
                        state.error("Workspace not found");
                    }
                    Err(e) => {
                        state.error(format!("Failed to delete workspace: {}", e));
                    }
                }
            } else {
                state.error("Workspace manager not initialized");
            }
        }

        Event::WorkspaceSwitched { id: _, ref path } => {
            // Notification that workspace was switched (from another source)
            panels.handle_workspace_switch(path);
        }

        // ===== Slash Command Events =====

        Event::SlashCommand(ref cmd) => {
            let result = execute_slash_command(cmd, state, panels, config, pty_manager);
            if handle_slash_result(result, state, panels, config, screen_area, pty_manager)? {
                return Ok(true); // Exit requested
            }
        }

        // Other events
        _ => {}
    }

    Ok(false)
}

/// Handle workspace selector action
fn handle_workspace_selector_action(
    action: WorkspaceSelectorAction,
    state: &mut AppState,
    panels: &mut PanelRegistry,
    pty_manager: &Arc<parking_lot::RwLock<PtyAgentManager>>,
) {
    match action {
        WorkspaceSelectorAction::Select(id) => {
            // Switch to selected workspace
            match state.switch_workspace(id) {
                Ok(new_path) => {
                    cancel_all_agents(panels, pty_manager);
                    panels.handle_workspace_switch(&new_path);
                    state.info(format!("Switched to: {}", state.workspace_name()));
                }
                Err(e) => {
                    state.error(format!("Failed to switch: {}", e));
                }
            }
            state.input_mode.to_normal();
        }
        WorkspaceSelectorAction::Create { name, path } => {
            if let Some(manager) = &state.workspace_manager {
                match manager.create_workspace(&name, path.clone()) {
                    Ok(workspace) => {
                        state.info(format!("Created: {}", workspace.name));
                        // Switch to the new workspace
                        if let Ok(_) = state.switch_workspace(workspace.id) {
                            cancel_all_agents(panels, pty_manager);
                            panels.handle_workspace_switch(&workspace.path);
                        }
                    }
                    Err(e) => {
                        state.error(format!("Failed to create: {}", e));
                    }
                }
            }
            state.input_mode.to_normal();
        }
        WorkspaceSelectorAction::Delete(id) => {
            // Use a separate scope for the manager borrow
            let delete_result = state.workspace_manager.as_ref()
                .map(|manager| manager.delete_workspace(id));

            match delete_result {
                Some(Ok(Some(deleted))) => {
                    let deleted_name = deleted.name.clone();
                    state.info(format!("Deleted: {}", deleted_name));
                    if state.active_workspace_id == Some(id) {
                        state.active_workspace_id = None;
                    }
                    // Refresh the workspace list in the selector
                    if let Some(manager) = &state.workspace_manager {
                        let workspaces = manager.list_workspaces();
                        panels.open_workspace_selector(workspaces, state.active_workspace_id);
                    }
                }
                Some(Ok(None)) => {
                    state.error("Workspace not found");
                }
                Some(Err(e)) => {
                    state.error(format!("Failed to delete: {}", e));
                }
                None => {
                    state.error("Workspace manager not initialized");
                }
            }
        }
        WorkspaceSelectorAction::Cancel => {
            state.input_mode.to_normal();
        }
        WorkspaceSelectorAction::None => {
            // No action needed
        }
    }
}

/// Handle workspace selector action during initial workspace selection phase
/// (before any workspace is active - not a modal, the main screen)
fn handle_workspace_selector_action_initial(
    action: WorkspaceSelectorAction,
    state: &mut AppState,
    panels: &mut PanelRegistry,
    pty_manager: &Arc<parking_lot::RwLock<PtyAgentManager>>,
) {
    match action {
        WorkspaceSelectorAction::Select(id) => {
            // Switch to selected workspace
            match state.switch_workspace(id) {
                Ok(new_path) => {
                    cancel_all_agents(panels, pty_manager);
                    panels.handle_workspace_switch(&new_path);
                    state.info(format!("Opened: {}", state.workspace_name()));
                }
                Err(e) => {
                    state.error(format!("Failed to open: {}", e));
                }
            }
            // Don't close modal - workspace is now active, render.rs will show main UI
        }
        WorkspaceSelectorAction::Create { name, path } => {
            if let Some(manager) = &state.workspace_manager {
                match manager.create_workspace(&name, path.clone()) {
                    Ok(workspace) => {
                        state.info(format!("Created: {}", workspace.name));
                        // Switch to the new workspace
                        if let Ok(_) = state.switch_workspace(workspace.id) {
                            cancel_all_agents(panels, pty_manager);
                            panels.handle_workspace_switch(&workspace.path);
                        }
                    }
                    Err(e) => {
                        state.error(format!("Failed to create: {}", e));
                    }
                }
            }
            // Don't close modal - workspace is now active, render.rs will show main UI
        }
        WorkspaceSelectorAction::Delete(id) => {
            // Use a separate scope for the manager borrow
            let delete_result = state.workspace_manager.as_ref()
                .map(|manager| manager.delete_workspace(id));

            match delete_result {
                Some(Ok(Some(deleted))) => {
                    let deleted_name = deleted.name.clone();
                    state.info(format!("Deleted: {}", deleted_name));
                    // Refresh the workspace list in the selector
                    if let Some(manager) = &state.workspace_manager {
                        let workspaces = manager.list_workspaces();
                        panels.open_workspace_selector(workspaces, state.active_workspace_id);
                    }
                }
                Some(Ok(None)) => {
                    state.error("Workspace not found");
                }
                Some(Err(e)) => {
                    state.error(format!("Failed to delete: {}", e));
                }
                None => {
                    state.error("Workspace manager not initialized");
                }
            }
        }
        WorkspaceSelectorAction::Cancel => {
            // Cannot cancel during initial selection - must select a workspace
            // Do nothing
        }
        WorkspaceSelectorAction::None => {
            // No action needed
        }
    }
}

/// Cancel all running agents when switching workspaces
fn cancel_all_agents(
    panels: &mut PanelRegistry,
    pty_manager: &Arc<parking_lot::RwLock<axiom::agents::PtyAgentManager>>,
) {
    // Get all running agent IDs
    let running_ids: Vec<_> = {
        let registry = panels.agent_registry.read();
        registry
            .agents()
            .filter(|a| a.status == axiom::agents::AgentStatus::Running)
            .map(|a| a.id)
            .collect()
    };

    // Mark agents as cancelled in registry
    {
        let mut registry = panels.agent_registry.write();
        for id in &running_ids {
            registry.cancel(*id);
        }
    }

    // Remove PTY sessions
    {
        let mut manager = pty_manager.write();
        for id in running_ids {
            manager.remove(id);
        }
    }
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

/// Execute a slash command and return the result
fn execute_slash_command(
    cmd: &SlashCommand,
    state: &AppState,
    panels: &PanelRegistry,
    config: &AxiomConfig,
    _pty_manager: &Arc<parking_lot::RwLock<PtyAgentManager>>,
) -> SlashCommandResult {
    match cmd {
        SlashCommand::Help { command } => {
            if let Some(topic) = command {
                // Get help for specific command
                if let Some(help) = SlashCommandParser::get_command_help(topic) {
                    let mut text = format!("/{} - {}\n\n", help.name, help.description);
                    text.push_str(&format!("Usage: {}\n", help.usage));
                    if !help.aliases.is_empty() {
                        text.push_str(&format!("Aliases: /{}\n", help.aliases.join(", /")));
                    }
                    if !help.examples.is_empty() {
                        text.push_str("\nExamples:\n");
                        for ex in &help.examples {
                            text.push_str(&format!("  {}\n", ex));
                        }
                    }
                    SlashCommandResult::data(SlashCommandData::Text(text))
                } else {
                    SlashCommandResult::error(format!("Unknown command: {}", topic))
                }
            } else {
                // Get help for all commands
                let commands = SlashCommandParser::get_all_commands_help();
                SlashCommandResult::data(SlashCommandData::Help { commands })
            }
        }

        SlashCommand::Clear => SlashCommandResult::action(UiAction::ClearOutput),

        SlashCommand::Settings => SlashCommandResult::action(UiAction::OpenSettings),

        SlashCommand::Exit => SlashCommandResult::exit(),

        SlashCommand::Version => {
            let version = axiom_core::version().to_string();
            SlashCommandResult::data(SlashCommandData::Version {
                version,
                commit: None,
            })
        }

        SlashCommand::Init { path } => {
            // Initialize workspace at the given path or current directory
            let target_path = path.clone().unwrap_or_else(|| state.cwd.clone());
            if target_path.exists() && target_path.is_dir() {
                // Could create a workspace here, but for now just show message
                SlashCommandResult::success(format!(
                    "Would initialize workspace at: {}",
                    target_path.display()
                ))
            } else {
                SlashCommandResult::error(format!("Path does not exist: {}", target_path.display()))
            }
        }

        SlashCommand::Workspace(sub) => match sub {
            WorkspaceSubcommand::List => {
                if let Some(manager) = &state.workspace_manager {
                    let workspaces: Vec<axiom_core::WorkspaceInfo> = manager
                        .list_workspaces()
                        .iter()
                        .map(|w| axiom_core::WorkspaceInfo {
                            id: w.id.to_string(),
                            name: w.name.clone(),
                            path: w.path.to_string_lossy().to_string(),
                            is_active: state.active_workspace_id == Some(w.id),
                        })
                        .collect();
                    SlashCommandResult::data(SlashCommandData::WorkspaceList(workspaces))
                } else {
                    SlashCommandResult::error("Workspace manager not initialized")
                }
            }
            WorkspaceSubcommand::Switch { id } => {
                if id.is_some() {
                    // Switch to specific workspace - need to parse ID
                    SlashCommandResult::success("Use /workspace switch to open the selector")
                } else {
                    // Open workspace selector
                    SlashCommandResult::action(UiAction::OpenWorkspaceSelector)
                }
            }
            WorkspaceSubcommand::Create { name, path } => {
                if let Some(manager) = &state.workspace_manager {
                    match manager.create_workspace(name, path.clone()) {
                        Ok(ws) => SlashCommandResult::success(format!("Created workspace: {}", ws.name)),
                        Err(e) => SlashCommandResult::error(format!("Failed to create workspace: {}", e)),
                    }
                } else {
                    SlashCommandResult::error("Workspace manager not initialized")
                }
            }
        },

        SlashCommand::Model(sub) => match sub {
            ModelSubcommand::List => {
                // Open model selector
                SlashCommandResult::action(UiAction::OpenModelSelector)
            }
            ModelSubcommand::Set { model: _ } => {
                // For now, open the model selector - direct setting requires more work
                SlashCommandResult::action(UiAction::OpenModelSelector)
            }
            ModelSubcommand::Current => {
                let registry = panels.llm_registry.read();
                if let Some(provider) = registry.active() {
                    SlashCommandResult::success(format!(
                        "Current model: {} ({})",
                        provider.model(),
                        provider.id()
                    ))
                } else {
                    SlashCommandResult::error("No model selected")
                }
            }
        },

        SlashCommand::Theme(sub) => match sub {
            ThemeSubcommand::Toggle => SlashCommandResult::action(UiAction::ToggleTheme),
            ThemeSubcommand::Set { variant } => {
                SlashCommandResult::action(UiAction::SetTheme {
                    variant: variant.clone(),
                })
            }
        },

        SlashCommand::Custom { name, args: _ } => {
            // Custom commands are not supported yet
            SlashCommandResult::error(format!("Unknown command: /{}", name))
        }
    }
}

/// Handle the result of a slash command execution
///
/// Returns true if the application should exit
fn handle_slash_result(
    result: SlashCommandResult,
    state: &mut AppState,
    panels: &mut PanelRegistry,
    config: &mut AxiomConfig,
    screen_area: ratatui::layout::Rect,
    pty_manager: &Arc<parking_lot::RwLock<PtyAgentManager>>,
) -> Result<bool> {
    match result {
        SlashCommandResult::Success { message } => {
            if let Some(msg) = message {
                state.info(msg);
            }
            Ok(false)
        }

        SlashCommandResult::UiAction(action) => {
            match action {
                UiAction::OpenSettings => {
                    panels.open_settings(config);
                    state.input_mode.open_modal("settings");
                }
                UiAction::OpenModelSelector => {
                    panels.open_model_selector();
                    state.input_mode.open_modal("model_selector");
                }
                UiAction::OpenWorkspaceSelector => {
                    if let Some(manager) = &state.workspace_manager {
                        let workspaces = manager.list_workspaces();
                        panels.open_workspace_selector(workspaces, state.active_workspace_id);
                        state.input_mode.open_modal("workspace_selector");
                    } else {
                        state.error("Workspace manager not initialized");
                    }
                }
                UiAction::ClearOutput => {
                    panels.clear_output();
                    state.info("Output cleared");
                }
                UiAction::ToggleTheme => {
                    toggle_theme();
                    state.info(format!("Theme: {}", current_variant().as_str()));
                }
                UiAction::SetTheme { variant } => {
                    // For now, just toggle - specific theme setting requires more work
                    if variant == "dark" || variant == "light" {
                        let current = current_variant().as_str();
                        if (variant == "dark" && current != "dark")
                            || (variant == "light" && current != "light")
                        {
                            toggle_theme();
                        }
                        state.info(format!("Theme: {}", current_variant().as_str()));
                    } else {
                        state.error(format!("Unknown theme variant: {}", variant));
                    }
                }
                UiAction::FocusPanel { panel } => {
                    // Map panel name to PanelId
                    let panel_id = match panel.to_lowercase().as_str() {
                        "input" => Some(PanelId::INPUT),
                        "output" => Some(PanelId::OUTPUT),
                        "agents" => Some(PanelId::AGENTS),
                        "filetree" | "file_tree" | "files" => Some(PanelId::FILE_TREE),
                        _ => None,
                    };
                    if let Some(id) = panel_id {
                        state.focus.focus(id);
                        panels.handle_focus_change(id, screen_area);
                    } else {
                        state.error(format!("Unknown panel: {}", panel));
                    }
                }
            }
            Ok(false)
        }

        SlashCommandResult::Data(data) => {
            match data {
                SlashCommandData::Help { commands } => {
                    let mut text = String::from("Available commands:\n\n");
                    for cmd in commands {
                        text.push_str(&format!(
                            "  /{:<12} {}\n",
                            cmd.name, cmd.description
                        ));
                    }
                    text.push_str("\nUse /help <command> for more details.");
                    state.info(text);
                }
                SlashCommandData::Version { version, commit } => {
                    let msg = if let Some(hash) = commit {
                        format!("Axiom v{} ({})", version, hash)
                    } else {
                        format!("Axiom v{}", version)
                    };
                    state.info(msg);
                }
                SlashCommandData::WorkspaceList(workspaces) => {
                    if workspaces.is_empty() {
                        state.info("No workspaces found. Use /workspace create <name> <path> to create one.");
                    } else {
                        let mut text = String::from("Workspaces:\n\n");
                        for ws in workspaces {
                            let marker = if ws.is_active { " *" } else { "" };
                            text.push_str(&format!("  {}{} - {}\n", ws.name, marker, ws.path));
                        }
                        state.info(text);
                    }
                }
                SlashCommandData::ModelList { provider, models, active } => {
                    let mut text = format!("Models for {}:\n\n", provider);
                    for model in models {
                        let marker = if active.as_ref() == Some(&model) { " *" } else { "" };
                        text.push_str(&format!("  {}{}\n", model, marker));
                    }
                    state.info(text);
                }
                SlashCommandData::Text(text) => {
                    state.info(text);
                }
            }
            Ok(false)
        }

        SlashCommandResult::Error { message } => {
            state.error(message);
            Ok(false)
        }

        SlashCommandResult::Exit => {
            state.quit();
            Ok(true)
        }
    }
}
