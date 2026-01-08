//! System clipboard wrapper for Axiom
//!
//! Provides a simple interface to the system clipboard using arboard.

use arboard::Clipboard;
use parking_lot::Mutex;
use std::sync::OnceLock;

/// Global clipboard instance (lazily initialized)
static CLIPBOARD: OnceLock<Mutex<Option<Clipboard>>> = OnceLock::new();

/// Initialize the clipboard (call once at startup)
pub fn init() {
    CLIPBOARD.get_or_init(|| Mutex::new(Clipboard::new().ok()));
}

/// Copy text to clipboard
pub fn copy(text: &str) -> Result<(), String> {
    let guard = CLIPBOARD.get().ok_or("Clipboard not initialized")?;
    let mut lock = guard.lock();
    let clipboard = lock.as_mut().ok_or("Clipboard unavailable")?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

/// Get text from clipboard
pub fn paste() -> Result<String, String> {
    let guard = CLIPBOARD.get().ok_or("Clipboard not initialized")?;
    let mut lock = guard.lock();
    let clipboard = lock.as_mut().ok_or("Clipboard unavailable")?;
    clipboard.get_text().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_init() {
        init();
        // Should not panic on double init
        init();
    }
}
