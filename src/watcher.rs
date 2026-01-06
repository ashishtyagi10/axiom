//! File system watcher for detecting file changes
//!
//! Watches the project directory and sends events when files are modified.
//! This enables the editor to auto-open/update files modified by external tools
//! like Claude CLI, Gemini CLI, or any other process.

use crate::events::Event;
use crossbeam_channel::Sender;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use std::path::Path;
use std::time::Duration;

/// File watcher that monitors a directory for changes
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Create a new file watcher for the given directory
    ///
    /// # Arguments
    /// * `watch_path` - Directory to watch recursively
    /// * `event_tx` - Channel to send FileChanged events
    pub fn new(watch_path: &Path, event_tx: Sender<Event>) -> notify::Result<Self> {
        let tx = event_tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    // Only handle create and modify events
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            for path in event.paths {
                                // Skip directories, hidden files, and common non-source files
                                if path.is_file() && !should_ignore(&path) {
                                    let _ = tx.send(Event::FileChanged(path));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_millis(500))
                .with_compare_contents(false),
        )?;

        watcher.watch(watch_path, RecursiveMode::Recursive)?;

        Ok(Self { _watcher: watcher })
    }
}

/// Check if a path should be ignored by the watcher
fn should_ignore(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Ignore hidden files and directories
    if path.file_name()
        .map(|n| n.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
    {
        return true;
    }

    // Ignore common non-source directories and files
    let ignore_patterns = [
        "/target/",
        "/node_modules/",
        "/.git/",
        "/__pycache__/",
        "/.venv/",
        "/venv/",
        "/dist/",
        "/build/",
        ".pyc",
        ".pyo",
        ".class",
        ".o",
        ".a",
        ".so",
        ".dylib",
        ".lock",
        ".log",
        ".tmp",
        ".swp",
        ".swo",
        "~",
    ];

    for pattern in ignore_patterns {
        if path_str.contains(pattern) || path_str.ends_with(pattern) {
            return true;
        }
    }

    false
}
