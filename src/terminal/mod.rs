//! Terminal emulation module
//!
//! Contains PTY wrapper with proper resize support.

mod pty;

pub use pty::Pty;
