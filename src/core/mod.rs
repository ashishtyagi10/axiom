//! Core infrastructure module
//!
//! This module provides the foundational types and utilities used throughout the application.
//! It includes:
//! - **Error Handling**: Unified error types (`AxiomError`, `Result`) for consistent error management.
//! - **Configuration**: Core configuration structures (if any).
//! - **Utilities**: Common helper functions and types.

mod error;

pub use error::{AxiomError, PtyError, LlmError, Result};
