//! WorkspaceManager - Central coordinator for multi-workspace support
//!
//! Manages workspace lifecycle, configuration, and AxiomService instances.

use super::storage::{WorkspaceRegistry, WorkspaceStorage};
use super::types::{Workspace, WorkspaceConfig, WorkspaceId, WorkspaceView};
use crate::config::AxiomConfig;
use crate::error::{AxiomError, Result};
use crate::service::AxiomService;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Manages multiple workspaces and their associated services
pub struct WorkspaceManager {
    /// Workspace registry (persistent)
    registry: RwLock<WorkspaceRegistry>,

    /// Storage backend
    storage: WorkspaceStorage,

    /// Active AxiomService instances (lazy-loaded)
    services: RwLock<HashMap<WorkspaceId, Arc<AxiomService>>>,

    /// Global configuration
    global_config: AxiomConfig,

    /// Workspace-specific configs (cached)
    workspace_configs: RwLock<HashMap<WorkspaceId, WorkspaceConfig>>,
}

impl WorkspaceManager {
    /// Create a new WorkspaceManager
    pub fn new(global_config: AxiomConfig) -> Result<Self> {
        let storage = WorkspaceStorage::new()?;
        let registry = storage.load_registry()?;

        Ok(Self {
            registry: RwLock::new(registry),
            storage,
            services: RwLock::new(HashMap::new()),
            global_config,
            workspace_configs: RwLock::new(HashMap::new()),
        })
    }

    /// Create with custom storage (for testing)
    pub fn with_storage(global_config: AxiomConfig, storage: WorkspaceStorage) -> Result<Self> {
        let registry = storage.load_registry()?;

        Ok(Self {
            registry: RwLock::new(registry),
            storage,
            services: RwLock::new(HashMap::new()),
            global_config,
            workspace_configs: RwLock::new(HashMap::new()),
        })
    }

    // ========== Workspace CRUD ==========

    /// List all workspaces
    pub fn list_workspaces(&self) -> Vec<WorkspaceView> {
        self.registry
            .read()
            .list()
            .into_iter()
            .map(WorkspaceView::from)
            .collect()
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, id: WorkspaceId) -> Option<Workspace> {
        self.registry.read().get(id).cloned()
    }

    /// Get the currently active workspace
    pub fn active_workspace(&self) -> Option<Workspace> {
        let registry = self.registry.read();
        registry
            .active_workspace
            .and_then(|id| registry.get(id).cloned())
    }

    /// Get the active workspace ID
    pub fn active_workspace_id(&self) -> Option<WorkspaceId> {
        self.registry.read().active_workspace
    }

    /// Create a new local workspace
    pub fn create_workspace(&self, name: &str, path: PathBuf) -> Result<Workspace> {
        // Validate path exists
        if !path.exists() {
            return Err(AxiomError::Config(format!(
                "Workspace path does not exist: {}",
                path.display()
            )));
        }

        // Check for duplicates
        {
            let registry = self.registry.read();
            if registry.find_by_path(&path).is_some() {
                return Err(AxiomError::Config(format!(
                    "Workspace already exists for path: {}",
                    path.display()
                )));
            }
        }

        let workspace = Workspace::new_local(name, path);

        // Initialize workspace directory structure
        self.storage.init_workspace_dir(&workspace)?;

        // Add to registry and save
        {
            let mut registry = self.registry.write();
            registry.add(workspace.clone());
        }
        self.save()?;

        Ok(workspace)
    }

    /// Create a new remote workspace
    pub fn create_remote_workspace(
        &self,
        name: &str,
        path: PathBuf,
        host: &str,
        port: u16,
    ) -> Result<Workspace> {
        let workspace = Workspace::new_remote(name, path, host, port);

        {
            let mut registry = self.registry.write();
            registry.add(workspace.clone());
        }
        self.save()?;

        Ok(workspace)
    }

    /// Delete a workspace (does not delete files)
    pub fn delete_workspace(&self, id: WorkspaceId) -> Result<Option<Workspace>> {
        // Stop any running service
        self.stop_service(id);

        let workspace = {
            let mut registry = self.registry.write();
            registry.remove(id)
        };

        if workspace.is_some() {
            self.save()?;
        }

        // Clean up cached config
        self.workspace_configs.write().remove(&id);

        Ok(workspace)
    }

    /// Update workspace metadata
    pub fn update_workspace(&self, id: WorkspaceId, name: Option<&str>) -> Result<Workspace> {
        let workspace = {
            let mut registry = self.registry.write();
            let ws = registry
                .get_mut(id)
                .ok_or_else(|| AxiomError::Config(format!("Workspace not found: {}", id)))?;

            if let Some(n) = name {
                ws.name = n.to_string();
            }
            ws.clone()
        };

        self.save()?;
        Ok(workspace)
    }

    // ========== Workspace Activation ==========

    /// Activate a workspace (make it the current workspace)
    pub fn activate_workspace(&self, id: WorkspaceId) -> Result<Arc<AxiomService>> {
        // Verify workspace exists
        {
            let registry = self.registry.read();
            if registry.get(id).is_none() {
                return Err(AxiomError::Config(format!("Workspace not found: {}", id)));
            }
        }

        // Update active workspace
        {
            let mut registry = self.registry.write();
            registry.set_active(Some(id));
        }
        self.save()?;

        // Get or create service
        self.get_or_create_service(id)
    }

    /// Deactivate the current workspace
    pub fn deactivate_workspace(&self) -> Result<()> {
        let active_id = self.registry.read().active_workspace;

        if let Some(id) = active_id {
            self.stop_service(id);
        }

        {
            let mut registry = self.registry.write();
            registry.set_active(None);
        }
        self.save()?;

        Ok(())
    }

    // ========== Service Management ==========

    /// Get the AxiomService for a workspace (creates if needed)
    pub fn get_or_create_service(&self, id: WorkspaceId) -> Result<Arc<AxiomService>> {
        // Check if already exists
        if let Some(service) = self.services.read().get(&id) {
            return Ok(Arc::clone(service));
        }

        // Get workspace
        let workspace = self
            .registry
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AxiomError::Config(format!("Workspace not found: {}", id)))?;

        // Load workspace config
        let ws_config = self.get_workspace_config(id)?;

        // Build effective config (global + workspace overrides)
        let effective_config = self.build_effective_config(&ws_config);

        // Create service
        let service = AxiomService::new(effective_config, workspace.path.clone())?;
        let service = Arc::new(service);

        // Store service
        self.services.write().insert(id, Arc::clone(&service));

        Ok(service)
    }

    /// Get existing service without creating
    pub fn get_service(&self, id: WorkspaceId) -> Option<Arc<AxiomService>> {
        self.services.read().get(&id).cloned()
    }

    /// Stop and remove a service
    pub fn stop_service(&self, id: WorkspaceId) {
        if let Some(service) = self.services.write().remove(&id) {
            // Service will be dropped, triggering cleanup
            drop(service);
        }
    }

    /// Stop all services
    pub fn stop_all_services(&self) {
        self.services.write().clear();
    }

    // ========== Configuration ==========

    /// Get workspace-specific configuration
    pub fn get_workspace_config(&self, id: WorkspaceId) -> Result<WorkspaceConfig> {
        // Check cache
        if let Some(config) = self.workspace_configs.read().get(&id) {
            return Ok(config.clone());
        }

        // Load from disk
        let workspace = self
            .registry
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AxiomError::Config(format!("Workspace not found: {}", id)))?;

        let config = self.storage.load_workspace_config(&workspace)?;

        // Cache it
        self.workspace_configs.write().insert(id, config.clone());

        Ok(config)
    }

    /// Save workspace-specific configuration
    pub fn save_workspace_config(&self, id: WorkspaceId, config: &WorkspaceConfig) -> Result<()> {
        let workspace = self
            .registry
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AxiomError::Config(format!("Workspace not found: {}", id)))?;

        self.storage.save_workspace_config(&workspace, config)?;

        // Update cache
        self.workspace_configs.write().insert(id, config.clone());

        Ok(())
    }

    /// Build effective config by merging global and workspace configs
    fn build_effective_config(&self, ws_config: &WorkspaceConfig) -> AxiomConfig {
        let mut config = self.global_config.clone();

        // Apply workspace LLM overrides
        if let Some(ref llm) = ws_config.llm {
            if let Some(ref provider) = llm.default_provider {
                // Would set default provider here
                let _ = provider;
            }
            // Apply other overrides...
        }

        config
    }

    // ========== Persistence ==========

    /// Save registry to disk
    fn save(&self) -> Result<()> {
        let registry = self.registry.read();
        self.storage.save_registry(&registry)
    }

    /// Reload registry from disk
    pub fn reload(&self) -> Result<()> {
        let new_registry = self.storage.load_registry()?;
        *self.registry.write() = new_registry;
        Ok(())
    }

    // ========== Utilities ==========

    /// Find workspace by path
    pub fn find_by_path(&self, path: &std::path::Path) -> Option<Workspace> {
        self.registry.read().find_by_path(path).cloned()
    }

    /// Get recent workspaces
    pub fn recent_workspaces(&self, limit: usize) -> Vec<WorkspaceView> {
        let registry = self.registry.read();
        registry
            .recent
            .iter()
            .take(limit)
            .filter_map(|id| registry.get(*id).map(WorkspaceView::from))
            .collect()
    }

    /// Get number of active services
    pub fn active_service_count(&self) -> usize {
        self.services.read().len()
    }
}

impl Drop for WorkspaceManager {
    fn drop(&mut self) {
        // Clean shutdown of all services
        self.stop_all_services();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_manager() -> (WorkspaceManager, TempDir, TempDir) {
        let config_dir = TempDir::new().unwrap();
        let workspace_dir = TempDir::new().unwrap();

        let storage = WorkspaceStorage::with_config_dir(config_dir.path().to_path_buf());
        let manager =
            WorkspaceManager::with_storage(AxiomConfig::default(), storage).unwrap();

        (manager, config_dir, workspace_dir)
    }

    #[test]
    fn test_create_and_list_workspace() {
        let (manager, _config_dir, workspace_dir) = test_manager();

        let ws = manager
            .create_workspace("test", workspace_dir.path().to_path_buf())
            .unwrap();

        assert_eq!(ws.name, "test");

        let list = manager.list_workspaces();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test");
    }

    #[test]
    fn test_activate_workspace() {
        let (manager, _config_dir, workspace_dir) = test_manager();

        let ws = manager
            .create_workspace("test", workspace_dir.path().to_path_buf())
            .unwrap();

        assert!(manager.active_workspace().is_none());

        manager.activate_workspace(ws.id).unwrap();

        let active = manager.active_workspace().unwrap();
        assert_eq!(active.id, ws.id);
    }

    #[test]
    fn test_delete_workspace() {
        let (manager, _config_dir, workspace_dir) = test_manager();

        let ws = manager
            .create_workspace("test", workspace_dir.path().to_path_buf())
            .unwrap();

        manager.delete_workspace(ws.id).unwrap();

        assert!(manager.get_workspace(ws.id).is_none());
        assert!(manager.list_workspaces().is_empty());
    }
}
