//! Configuration loader with environment variable expansion
//!
//! Loads configuration from `.axiom.toml` in project root or user config directory.

use super::types::{AxiomConfig, ProviderConfig};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Configuration loading error
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
}

/// Load configuration from various sources
///
/// Priority order:
/// 1. Project-level `.axiom.toml`
/// 2. User-level `~/.config/axiom/config.toml`
/// 3. Default configuration
pub fn load_config(project_dir: &Path) -> Result<AxiomConfig, ConfigError> {
    // Try project-level config first
    let project_config = project_dir.join(".axiom.toml");
    if project_config.exists() {
        return load_from_file(&project_config);
    }

    // Try user-level config
    if let Some(user_config) = get_user_config_path() {
        if user_config.exists() {
            return load_from_file(&user_config);
        }
    }

    // Return default config with environment variable overrides
    Ok(apply_env_overrides(AxiomConfig::default()))
}

/// Get user config directory path
fn get_user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("axiom").join("config.toml"))
}

/// Load configuration from a specific file
fn load_from_file(path: &Path) -> Result<AxiomConfig, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let mut config: AxiomConfig = toml::from_str(&content)?;

    // Expand environment variables in the config
    expand_env_vars(&mut config);

    // Apply environment variable overrides
    config = apply_env_overrides(config);

    Ok(config)
}

/// Expand ${VAR} patterns in string values
fn expand_env_vars(config: &mut AxiomConfig) {
    let env_regex = Regex::new(r"\$\{([^}]+)\}").unwrap();

    for provider in config.llm.providers.values_mut() {
        if let Some(ref api_key) = provider.api_key {
            provider.api_key = Some(expand_string(api_key, &env_regex));
        }
        if let Some(ref base_url) = provider.base_url {
            provider.base_url = Some(expand_string(base_url, &env_regex));
        }
    }
}

/// Expand environment variables in a single string
fn expand_string(s: &str, regex: &Regex) -> String {
    regex
        .replace_all(s, |caps: &regex::Captures| {
            let var_name = &caps[1];
            std::env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name))
        })
        .to_string()
}

/// Apply environment variable overrides for common settings
///
/// Supports direct environment variables:
/// - ANTHROPIC_API_KEY -> claude.api_key
/// - GOOGLE_API_KEY / GEMINI_API_KEY -> gemini.api_key
/// - OPENAI_API_KEY -> openai.api_key
/// - OLLAMA_BASE_URL -> ollama.base_url
fn apply_env_overrides(mut config: AxiomConfig) -> AxiomConfig {
    // Claude / Anthropic
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            let provider = config
                .llm
                .providers
                .entry("claude".to_string())
                .or_insert_with(ProviderConfig::default);
            provider.api_key = Some(key);
            provider.enabled = true;
        }
    }

    // Gemini / Google
    for env_var in ["GOOGLE_API_KEY", "GEMINI_API_KEY"] {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                let provider = config
                    .llm
                    .providers
                    .entry("gemini".to_string())
                    .or_insert_with(ProviderConfig::default);
                provider.api_key = Some(key);
                provider.enabled = true;
                break;
            }
        }
    }

    // OpenAI
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            let provider = config
                .llm
                .providers
                .entry("openai".to_string())
                .or_insert_with(ProviderConfig::default);
            provider.api_key = Some(key);
            provider.enabled = true;
        }
    }

    // Ollama base URL override
    if let Ok(url) = std::env::var("OLLAMA_BASE_URL") {
        if !url.is_empty() {
            let provider = config
                .llm
                .providers
                .entry("ollama".to_string())
                .or_insert_with(ProviderConfig::default);
            provider.base_url = Some(url);
        }
    }

    // Default provider override
    if let Ok(provider) = std::env::var("AXIOM_DEFAULT_PROVIDER") {
        if !provider.is_empty() {
            config.llm.default_provider = provider;
        }
    }

    config
}

/// Create a sample configuration file content
pub fn sample_config() -> &'static str {
    r#"# Axiom Configuration
# Place this file in your project root as .axiom.toml
# or in ~/.config/axiom/config.toml for global settings

[llm]
# Default provider: claude, gemini, ollama, openai
default_provider = "claude"

# Request timeout in seconds
timeout = 120

# Maximum retries for failed requests
max_retries = 3

[llm.providers.claude]
enabled = true
api_key = "${ANTHROPIC_API_KEY}"
default_model = "claude-sonnet-4-20250514"

[llm.providers.gemini]
enabled = true
api_key = "${GOOGLE_API_KEY}"
default_model = "gemini-2.0-flash"

[llm.providers.ollama]
enabled = true
base_url = "http://localhost:11434"
default_model = "gemma3:4b"

[llm.providers.openai]
enabled = false
api_key = "${OPENAI_API_KEY}"
default_model = "gpt-4o"
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AxiomConfig::default();
        assert_eq!(config.llm.default_provider, "ollama");
        assert!(config.llm.providers.contains_key("ollama"));
    }

    #[test]
    fn test_expand_env_var() {
        let regex = Regex::new(r"\$\{([^}]+)\}").unwrap();
        std::env::set_var("TEST_VAR", "test_value");
        let result = expand_string("prefix_${TEST_VAR}_suffix", &regex);
        assert_eq!(result, "prefix_test_value_suffix");
        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_missing_env_var() {
        let regex = Regex::new(r"\$\{([^}]+)\}").unwrap();
        let result = expand_string("${NONEXISTENT_VAR}", &regex);
        assert_eq!(result, "${NONEXISTENT_VAR}");
    }
}
