//! Configuration module for Axiom
//!
//! Handles loading and parsing of `.axiom.toml` configuration files
//! with support for environment variable expansion.

mod cli_agents;
mod loader;
mod types;
mod writer;

pub use cli_agents::{CliAgentConfig, CliAgentsConfig};
pub use loader::{load_config, sample_config, ConfigError};
pub use types::{AxiomConfig, LlmConfig, ProviderConfig};
pub use writer::{config_path, save_config, user_config_path, WriteError};
