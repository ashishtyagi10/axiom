//! Workspace type definitions
//!
//! Core types for workspace management - serializable for persistence and IPC.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Unique workspace identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

impl WorkspaceId {
    /// Generate a new random workspace ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for WorkspaceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Workspace connection type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkspaceType {
    /// Local filesystem workspace
    Local,

    /// Remote workspace (SSH or similar)
    Remote {
        host: String,
        port: u16,
        #[serde(default)]
        user: Option<String>,
    },

    /// Container-based workspace
    Container {
        image: String,
        #[serde(default)]
        container_id: Option<String>,
    },
}

impl Default for WorkspaceType {
    fn default() -> Self {
        Self::Local
    }
}

/// Workspace metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique identifier
    pub id: WorkspaceId,

    /// Human-readable name
    pub name: String,

    /// Root path of the workspace
    pub path: PathBuf,

    /// Workspace type (local, remote, container)
    pub workspace_type: WorkspaceType,

    /// Creation timestamp (Unix epoch seconds)
    pub created_at: i64,

    /// Last access timestamp (Unix epoch seconds)
    pub last_accessed: i64,

    /// Whether this workspace is currently active
    #[serde(default)]
    pub is_active: bool,

    /// Custom tags for organization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Workspace-specific settings override path
    /// Defaults to <workspace_path>/.axiom/config.toml
    #[serde(default)]
    pub config_path: Option<PathBuf>,
}

impl Workspace {
    /// Create a new local workspace
    pub fn new_local(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            id: WorkspaceId::new(),
            name: name.into(),
            path: path.into(),
            workspace_type: WorkspaceType::Local,
            created_at: now,
            last_accessed: now,
            is_active: false,
            tags: Vec::new(),
            config_path: None,
        }
    }

    /// Create a new remote workspace
    pub fn new_remote(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            id: WorkspaceId::new(),
            name: name.into(),
            path: path.into(),
            workspace_type: WorkspaceType::Remote {
                host: host.into(),
                port,
                user: None,
            },
            created_at: now,
            last_accessed: now,
            is_active: false,
            tags: Vec::new(),
            config_path: None,
        }
    }

    /// Update last accessed timestamp to now
    pub fn touch(&mut self) {
        self.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }

    /// Get the effective config path
    pub fn effective_config_path(&self) -> PathBuf {
        self.config_path
            .clone()
            .unwrap_or_else(|| self.path.join(".axiom").join("config.toml"))
    }

    /// Check if workspace path exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Workspace-specific configuration overrides
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// LLM provider overrides for this workspace
    #[serde(default)]
    pub llm: Option<WorkspaceLlmConfig>,

    /// Custom CLI agents specific to this workspace
    #[serde(default)]
    pub cli_agents: Vec<WorkspaceCliAgent>,

    /// Custom environment variables
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,

    /// Ignored paths (gitignore-style patterns)
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
}

/// Workspace-specific LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLlmConfig {
    /// Override default provider
    pub default_provider: Option<String>,

    /// Override default model
    pub default_model: Option<String>,

    /// Provider-specific overrides
    #[serde(default)]
    pub providers: std::collections::HashMap<String, WorkspaceProviderConfig>,
}

/// Workspace-specific provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceProviderConfig {
    /// Override model for this provider
    pub model: Option<String>,

    /// Override API key (not recommended, use env vars)
    pub api_key: Option<String>,

    /// Override base URL
    pub base_url: Option<String>,
}

/// Custom CLI agent definition for a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceCliAgent {
    /// Agent identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Command to execute
    pub command: String,

    /// Default arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,

    /// Working directory override
    pub cwd: Option<PathBuf>,

    /// Icon for display
    #[serde(default = "default_icon")]
    pub icon: String,
}

fn default_icon() -> String {
    "🔧".to_string()
}

/// Summary view of a workspace for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceView {
    pub id: WorkspaceId,
    pub name: String,
    pub path: PathBuf,
    pub workspace_type: WorkspaceType,
    pub is_active: bool,
    pub last_accessed: i64,
    pub exists: bool,
}

impl From<&Workspace> for WorkspaceView {
    fn from(ws: &Workspace) -> Self {
        Self {
            id: ws.id,
            name: ws.name.clone(),
            path: ws.path.clone(),
            workspace_type: ws.workspace_type.clone(),
            is_active: ws.is_active,
            last_accessed: ws.last_accessed,
            exists: ws.exists(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_id_roundtrip() {
        let id = WorkspaceId::new();
        let s = id.to_string();
        let parsed: WorkspaceId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_workspace_serialization() {
        let ws = Workspace::new_local("test", "/tmp/test");
        let json = serde_json::to_string(&ws).unwrap();
        let parsed: Workspace = serde_json::from_str(&json).unwrap();
        assert_eq!(ws.id, parsed.id);
        assert_eq!(ws.name, parsed.name);
    }

    #[test]
    fn test_workspace_type_serialization() {
        let remote = WorkspaceType::Remote {
            host: "example.com".into(),
            port: 22,
            user: Some("user".into()),
        };
        let json = serde_json::to_string(&remote).unwrap();
        assert!(json.contains("Remote"));
        assert!(json.contains("example.com"));
    }
}
