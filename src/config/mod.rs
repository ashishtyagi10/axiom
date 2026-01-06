//! Configuration module for Axiom
//!
//! Handles loading and parsing of `.axiom.toml` configuration files
//! with support for environment variable expansion.

mod loader;
mod types;

pub use loader::{load_config, sample_config, ConfigError};
pub use types::{AxiomConfig, LlmConfig, ProviderConfig};
