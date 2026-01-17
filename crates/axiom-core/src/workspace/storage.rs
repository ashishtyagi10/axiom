//! Workspace persistence layer
//!
//! Handles loading and saving workspace data to disk.

use super::types::{Workspace, WorkspaceConfig, WorkspaceId};
use crate::error::{AxiomError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Global workspace registry stored in ~/.axiom/workspaces.json
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceRegistry {
    /// All registered workspaces
    pub workspaces: HashMap<WorkspaceId, Workspace>,

    /// ID of the currently active workspace
    pub active_workspace: Option<WorkspaceId>,

    /// Recently accessed workspace IDs (most recent first)
    #[serde(default)]
    pub recent: Vec<WorkspaceId>,
}

impl WorkspaceRegistry {
    /// Maximum number of recent workspaces to track
    const MAX_RECENT: usize = 20;

    /// Add a workspace to the registry
    pub fn add(&mut self, workspace: Workspace) {
        let id = workspace.id;
        self.workspaces.insert(id, workspace);
        self.add_to_recent(id);
    }

    /// Remove a workspace from the registry
    pub fn remove(&mut self, id: WorkspaceId) -> Option<Workspace> {
        self.recent.retain(|&r| r != id);
        if self.active_workspace == Some(id) {
            self.active_workspace = None;
        }
        self.workspaces.remove(&id)
    }

    /// Get a workspace by ID
    pub fn get(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(&id)
    }

    /// Get a mutable workspace by ID
    pub fn get_mut(&mut self, id: WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces.get_mut(&id)
    }

    /// Set the active workspace
    pub fn set_active(&mut self, id: Option<WorkspaceId>) {
        // Deactivate previous
        if let Some(prev_id) = self.active_workspace {
            if let Some(ws) = self.workspaces.get_mut(&prev_id) {
                ws.is_active = false;
            }
        }

        // Activate new
        if let Some(new_id) = id {
            if let Some(ws) = self.workspaces.get_mut(&new_id) {
                ws.is_active = true;
                ws.touch();
                self.add_to_recent(new_id);
            }
        }

        self.active_workspace = id;
    }

    /// Add workspace to recent list
    fn add_to_recent(&mut self, id: WorkspaceId) {
        self.recent.retain(|&r| r != id);
        self.recent.insert(0, id);
        self.recent.truncate(Self::MAX_RECENT);
    }

    /// List all workspaces sorted by last accessed
    pub fn list(&self) -> Vec<&Workspace> {
        let mut workspaces: Vec<_> = self.workspaces.values().collect();
        workspaces.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        workspaces
    }

    /// Find workspace by path
    pub fn find_by_path(&self, path: &Path) -> Option<&Workspace> {
        self.workspaces.values().find(|ws| ws.path == path)
    }
}

/// Storage backend for workspace data
pub struct WorkspaceStorage {
    /// Base directory for axiom config (~/.axiom)
    config_dir: PathBuf,

    /// Path to workspace registry file
    registry_path: PathBuf,
}

impl WorkspaceStorage {
    /// Create a new storage instance
    pub fn new() -> Result<Self> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| AxiomError::Config("Could not determine home directory".into()))?
            .join(".axiom");

        Ok(Self {
            registry_path: config_dir.join("workspaces.json"),
            config_dir,
        })
    }

    /// Create with custom config directory (for testing)
    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self {
            registry_path: config_dir.join("workspaces.json"),
            config_dir,
        }
    }

    /// Ensure config directory exists
    pub fn ensure_config_dir(&self) -> Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir).map_err(|e| {
                AxiomError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create config directory: {}", e),
                ))
            })?;
        }
        Ok(())
    }

    /// Load workspace registry from disk
    pub fn load_registry(&self) -> Result<WorkspaceRegistry> {
        if !self.registry_path.exists() {
            return Ok(WorkspaceRegistry::default());
        }

        let content = fs::read_to_string(&self.registry_path).map_err(|e| {
            AxiomError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read workspace registry: {}", e),
            ))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            AxiomError::Config(format!("Failed to parse workspace registry: {}", e))
        })
    }

    /// Save workspace registry to disk
    pub fn save_registry(&self, registry: &WorkspaceRegistry) -> Result<()> {
        self.ensure_config_dir()?;

        let content = serde_json::to_string_pretty(registry).map_err(|e| {
            AxiomError::Config(format!("Failed to serialize workspace registry: {}", e))
        })?;

        // Write atomically using temp file
        let temp_path = self.registry_path.with_extension("json.tmp");
        fs::write(&temp_path, &content).map_err(|e| {
            AxiomError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write workspace registry: {}", e),
            ))
        })?;

        fs::rename(&temp_path, &self.registry_path).map_err(|e| {
            AxiomError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to save workspace registry: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Load workspace-specific config
    pub fn load_workspace_config(&self, workspace: &Workspace) -> Result<WorkspaceConfig> {
        let config_path = workspace.effective_config_path();

        if !config_path.exists() {
            return Ok(WorkspaceConfig::default());
        }

        let content = fs::read_to_string(&config_path).map_err(|e| {
            AxiomError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read workspace config: {}", e),
            ))
        })?;

        // Support both TOML and JSON
        if config_path.extension().map_or(false, |e| e == "toml") {
            toml::from_str(&content)
                .map_err(|e| AxiomError::Config(format!("Failed to parse workspace TOML: {}", e)))
        } else {
            serde_json::from_str(&content)
                .map_err(|e| AxiomError::Config(format!("Failed to parse workspace JSON: {}", e)))
        }
    }

    /// Save workspace-specific config
    pub fn save_workspace_config(
        &self,
        workspace: &Workspace,
        config: &WorkspaceConfig,
    ) -> Result<()> {
        let config_path = workspace.effective_config_path();

        // Ensure .axiom directory exists in workspace
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    AxiomError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to create workspace config directory: {}", e),
                    ))
                })?;
            }
        }

        let content = if config_path.extension().map_or(false, |e| e == "toml") {
            toml::to_string_pretty(config)
                .map_err(|e| AxiomError::Config(format!("Failed to serialize TOML: {}", e)))?
        } else {
            serde_json::to_string_pretty(config)
                .map_err(|e| AxiomError::Config(format!("Failed to serialize JSON: {}", e)))?
        };

        fs::write(&config_path, content).map_err(|e| {
            AxiomError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write workspace config: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Initialize a new workspace directory structure
    pub fn init_workspace_dir(&self, workspace: &Workspace) -> Result<()> {
        let axiom_dir = workspace.path.join(".axiom");

        if !axiom_dir.exists() {
            fs::create_dir_all(&axiom_dir).map_err(|e| {
                AxiomError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create .axiom directory: {}", e),
                ))
            })?;
        }

        // Create default config if it doesn't exist
        let config_path = workspace.effective_config_path();
        if !config_path.exists() {
            self.save_workspace_config(workspace, &WorkspaceConfig::default())?;
        }

        Ok(())
    }
}

impl Default for WorkspaceStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create workspace storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_registry_operations() {
        let mut registry = WorkspaceRegistry::default();

        let ws1 = Workspace::new_local("test1", "/tmp/test1");
        let ws2 = Workspace::new_local("test2", "/tmp/test2");

        let id1 = ws1.id;
        let id2 = ws2.id;

        registry.add(ws1);
        registry.add(ws2);

        assert_eq!(registry.workspaces.len(), 2);
        assert!(registry.get(id1).is_some());

        registry.set_active(Some(id1));
        assert_eq!(registry.active_workspace, Some(id1));
        assert!(registry.get(id1).unwrap().is_active);

        registry.remove(id1);
        assert!(registry.get(id1).is_none());
        assert_eq!(registry.active_workspace, None);
    }

    #[test]
    fn test_storage_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = WorkspaceStorage::with_config_dir(temp_dir.path().to_path_buf());

        let mut registry = WorkspaceRegistry::default();
        let ws = Workspace::new_local("test", "/tmp/test");
        registry.add(ws);

        storage.save_registry(&registry).unwrap();
        let loaded = storage.load_registry().unwrap();

        assert_eq!(loaded.workspaces.len(), 1);
    }
}
