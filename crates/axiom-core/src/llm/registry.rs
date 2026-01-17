//! Provider registry for managing multiple LLM providers

use super::{LlmError, ProviderStatus, SharedProvider};
use crate::config::AxiomConfig;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Registry for managing multiple LLM providers
pub struct ProviderRegistry {
    /// All registered providers
    providers: HashMap<String, SharedProvider>,

    /// Currently active provider ID
    active_provider: RwLock<String>,

    /// Fallback chain for automatic failover
    fallback_chain: Vec<String>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active_provider: RwLock::new(String::new()),
            fallback_chain: Vec::new(),
        }
    }

    /// Create a registry from configuration
    pub fn from_config(config: &AxiomConfig) -> Self {
        let mut registry = Self::new();

        // Set default provider
        *registry.active_provider.write() = config.llm.default_provider.clone();

        // Build fallback chain from enabled providers
        registry.fallback_chain = config
            .llm
            .providers
            .iter()
            .filter(|(_, cfg)| cfg.enabled)
            .map(|(name, _)| name.clone())
            .collect();

        registry
    }

    /// Register a provider
    pub fn register(&mut self, provider: SharedProvider) {
        let id = provider.id().to_string();
        self.providers.insert(id, provider);
    }

    /// Get a provider by ID
    pub fn get(&self, id: &str) -> Option<SharedProvider> {
        self.providers.get(id).cloned()
    }

    /// Get the currently active provider
    pub fn active(&self) -> Option<SharedProvider> {
        let id = self.active_provider.read();
        self.get(&id)
    }

    /// Set the active provider
    pub fn set_active(&self, id: &str) -> Result<(), LlmError> {
        if !self.providers.contains_key(id) {
            return Err(LlmError::ProviderUnavailable(format!(
                "Provider '{}' not registered",
                id
            )));
        }
        *self.active_provider.write() = id.to_string();
        Ok(())
    }

    /// Get active provider ID
    pub fn active_id(&self) -> String {
        self.active_provider.read().clone()
    }

    /// Get a provider with automatic fallback
    pub fn get_with_fallback(&self, id: &str) -> Option<SharedProvider> {
        // Try the requested provider first
        if let Some(provider) = self.get(id) {
            if provider.status() == ProviderStatus::Ready {
                return Some(provider);
            }
        }

        // Try fallback chain
        for fallback_id in &self.fallback_chain {
            if let Some(provider) = self.get(fallback_id) {
                if provider.status() == ProviderStatus::Ready {
                    return Some(provider);
                }
            }
        }

        None
    }

    /// Get all registered provider IDs
    pub fn provider_ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Get all available (ready) providers
    pub fn available_providers(&self) -> Vec<SharedProvider> {
        self.providers
            .values()
            .filter(|p| p.status() == ProviderStatus::Ready)
            .cloned()
            .collect()
    }

    /// Get display info for all providers
    pub fn provider_info(&self) -> Vec<ProviderInfo> {
        self.providers
            .values()
            .map(|p| ProviderInfo {
                id: p.id().to_string(),
                name: p.name().to_string(),
                model: p.model(),
                status: p.status(),
            })
            .collect()
    }

    /// List all models across all providers
    pub fn all_models(&self) -> Vec<(String, String)> {
        let mut models = Vec::new();
        for provider in self.providers.values() {
            if let Ok(provider_models) = provider.list_models() {
                for model in provider_models {
                    models.push((provider.id().to_string(), model));
                }
            }
        }
        models
    }

    /// Set model on a specific provider
    pub fn set_model(&self, provider_id: &str, model: &str) -> Result<(), LlmError> {
        let provider = self
            .get(provider_id)
            .ok_or_else(|| LlmError::ProviderUnavailable(provider_id.to_string()))?;
        provider.set_model(model)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider information for display
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub status: ProviderStatus,
}

impl ProviderInfo {
    /// Get a display string for the provider
    pub fn display(&self) -> String {
        format!("{} ({})", self.name, self.model)
    }

    /// Get status indicator
    pub fn status_indicator(&self) -> &str {
        match &self.status {
            ProviderStatus::Ready => "●",
            ProviderStatus::Busy => "◐",
            ProviderStatus::Unavailable(_) => "○",
            ProviderStatus::RateLimited => "◌",
        }
    }
}
