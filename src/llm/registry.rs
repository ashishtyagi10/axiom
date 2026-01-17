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
    ///
    /// If the requested provider is unavailable, tries the fallback chain.
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

    /// Get display info for all providers (for model selector)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{ChatMessage, LlmProvider, ProviderCapabilities};
    use std::sync::Arc;

    /// Mock provider for testing
    struct MockProvider {
        id: String,
        name: String,
        model: RwLock<String>,
        status: ProviderStatus,
    }

    impl MockProvider {
        fn new(id: &str, status: ProviderStatus) -> Self {
            Self {
                id: id.to_string(),
                name: format!("Mock {}", id),
                model: RwLock::new("mock-model".to_string()),
                status,
            }
        }
    }

    impl LlmProvider for MockProvider {
        fn id(&self) -> &str {
            &self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn model(&self) -> String {
            self.model.read().clone()
        }

        fn set_model(&self, model: &str) -> Result<(), LlmError> {
            *self.model.write() = model.to_string();
            Ok(())
        }

        fn list_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["mock-model".to_string(), "mock-model-2".to_string()])
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn status(&self) -> ProviderStatus {
            self.status.clone()
        }

        fn send_message(&self, _messages: Vec<ChatMessage>, _event_tx: crossbeam_channel::Sender<crate::events::Event>) {
            // Mock - does nothing
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ProviderRegistry::new();
        assert!(registry.provider_ids().is_empty());
        assert!(registry.active().is_none());
    }

    #[test]
    fn test_registry_register_provider() {
        let mut registry = ProviderRegistry::new();
        let provider: SharedProvider = Arc::new(MockProvider::new("test", ProviderStatus::Ready));
        registry.register(provider);

        assert!(registry.provider_ids().contains(&"test".to_string()));
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_registry_get_provider() {
        let mut registry = ProviderRegistry::new();
        let provider: SharedProvider = Arc::new(MockProvider::new("claude", ProviderStatus::Ready));
        registry.register(provider);

        assert!(registry.get("claude").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_set_active() {
        let mut registry = ProviderRegistry::new();
        let provider: SharedProvider = Arc::new(MockProvider::new("claude", ProviderStatus::Ready));
        registry.register(provider);

        assert!(registry.set_active("claude").is_ok());
        assert_eq!(registry.active_id(), "claude");
        assert!(registry.active().is_some());
    }

    #[test]
    fn test_registry_set_active_nonexistent() {
        let registry = ProviderRegistry::new();
        assert!(registry.set_active("nonexistent").is_err());
    }

    #[test]
    fn test_registry_fallback_chain() {
        let mut registry = ProviderRegistry::new();

        // First provider is unavailable
        let unavailable: SharedProvider = Arc::new(MockProvider::new("claude", ProviderStatus::Unavailable("No key".to_string())));
        registry.register(unavailable);

        // Second provider is ready
        let ready: SharedProvider = Arc::new(MockProvider::new("ollama", ProviderStatus::Ready));
        registry.register(ready);

        registry.fallback_chain = vec!["claude".to_string(), "ollama".to_string()];

        // Should fall back to ollama
        let provider = registry.get_with_fallback("claude");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().id(), "ollama");
    }

    #[test]
    fn test_registry_available_providers() {
        let mut registry = ProviderRegistry::new();

        let ready1: SharedProvider = Arc::new(MockProvider::new("provider1", ProviderStatus::Ready));
        let ready2: SharedProvider = Arc::new(MockProvider::new("provider2", ProviderStatus::Ready));
        let unavailable: SharedProvider = Arc::new(MockProvider::new("provider3", ProviderStatus::Unavailable("down".to_string())));

        registry.register(ready1);
        registry.register(ready2);
        registry.register(unavailable);

        let available = registry.available_providers();
        assert_eq!(available.len(), 2);
    }

    #[test]
    fn test_registry_all_models() {
        let mut registry = ProviderRegistry::new();
        let provider: SharedProvider = Arc::new(MockProvider::new("test", ProviderStatus::Ready));
        registry.register(provider);

        let models = registry.all_models();
        assert_eq!(models.len(), 2);
        assert!(models.iter().any(|(p, m)| p == "test" && m == "mock-model"));
    }

    #[test]
    fn test_provider_info_display() {
        let info = ProviderInfo {
            id: "claude".to_string(),
            name: "Claude".to_string(),
            model: "claude-3-sonnet".to_string(),
            status: ProviderStatus::Ready,
        };
        assert_eq!(info.display(), "Claude (claude-3-sonnet)");
    }

    #[test]
    fn test_provider_info_status_indicators() {
        let ready = ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            model: "model".to_string(),
            status: ProviderStatus::Ready,
        };
        assert_eq!(ready.status_indicator(), "●");

        let busy = ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            model: "model".to_string(),
            status: ProviderStatus::Busy,
        };
        assert_eq!(busy.status_indicator(), "◐");

        let unavailable = ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            model: "model".to_string(),
            status: ProviderStatus::Unavailable("error".to_string()),
        };
        assert_eq!(unavailable.status_indicator(), "○");

        let rate_limited = ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            model: "model".to_string(),
            status: ProviderStatus::RateLimited,
        };
        assert_eq!(rate_limited.status_indicator(), "◌");
    }
}
