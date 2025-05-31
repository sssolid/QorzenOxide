// src/platform/mod.rs - Core platform abstraction

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub mod database;
pub mod filesystem;
pub mod network;
pub mod storage;

// Re-export types
use crate::platform::database::DatabaseArc;
use crate::platform::filesystem::FileSystemArc;
use crate::platform::network::NetworkArc;
use crate::platform::storage::StorageArc;
pub use database::{DatabaseProvider, Migration, QueryResult, Row, Transaction};
pub use filesystem::{FileInfo, FileMetadata, FileSystemProvider};
pub use network::{NetworkProvider, NetworkRequest, NetworkResponse};
pub use storage::StorageProvider;

/// Platform capabilities detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    pub has_filesystem: bool,
    pub has_database: bool,
    pub has_background_tasks: bool,
    pub has_push_notifications: bool,
    pub has_biometric_auth: bool,
    pub has_camera: bool,
    pub has_location: bool,
    pub max_file_size: Option<u64>,
    pub supported_formats: Vec<String>,
    pub platform_name: String,
    pub platform_version: String,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            has_filesystem: false,
            has_database: false,
            has_background_tasks: false,
            has_push_notifications: false,
            has_biometric_auth: false,
            has_camera: false,
            has_location: false,
            max_file_size: None,
            supported_formats: Vec::new(),
            platform_name: "unknown".to_string(),
            platform_version: "unknown".to_string(),
        }
    }
}

/// Main platform manager that coordinates all platform providers
pub struct PlatformManager {
    state: ManagedState,
    filesystem: FileSystemArc,
    database: DatabaseArc,
    network: NetworkArc,
    storage: StorageArc,
    capabilities: PlatformCapabilities,
}

impl std::fmt::Debug for PlatformManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformManager")
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

impl PlatformManager {
    /// Creates a new platform manager with platform-specific providers
    pub fn new() -> Result<Self> {
        let capabilities = Self::detect_capabilities();

        #[cfg(not(target_arch = "wasm32"))]
        let providers = native::create_providers()?;

        #[cfg(target_arch = "wasm32")]
        let providers = web::create_providers()?;

        Ok(Self {
            state: ManagedState::new(Uuid::new_v4(), "platform_manager"),
            filesystem: providers.filesystem,
            database: providers.database,
            network: providers.network,
            storage: providers.storage,
            capabilities,
        })
    }

    /// Detects platform capabilities
    pub fn detect_capabilities() -> PlatformCapabilities {
        #[cfg(not(target_arch = "wasm32"))]
        return native::detect_capabilities();

        #[cfg(target_arch = "wasm32")]
        return web::detect_capabilities();
    }

    /// Returns platform capabilities
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    /// Returns filesystem provider
    pub fn filesystem(&self) -> &dyn FileSystemProvider {
        self.filesystem.as_ref()
    }

    /// Returns database provider
    pub fn database(&self) -> &dyn DatabaseProvider {
        self.database.as_ref()
    }

    /// Returns network provider
    pub fn network(&self) -> &dyn NetworkProvider {
        self.network.as_ref()
    }

    /// Returns storage provider
    pub fn storage(&self) -> &dyn StorageProvider {
        self.storage.as_ref()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Manager for PlatformManager {
    fn name(&self) -> &str {
        "platform_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Platform-specific initialization
        #[cfg(not(target_arch = "wasm32"))]
        native::initialize().await?;

        #[cfg(target_arch = "wasm32")]
        web::initialize().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Platform-specific cleanup
        #[cfg(not(target_arch = "wasm32"))]
        native::cleanup().await?;

        #[cfg(target_arch = "wasm32")]
        web::cleanup().await?;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        status.add_metadata(
            "platform",
            serde_json::json!(self.capabilities.platform_name),
        );
        status.add_metadata(
            "capabilities",
            serde_json::to_value(&self.capabilities).unwrap_or_default(),
        );
        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: true,
            requires_database: true,
            requires_native_apis: false,
            minimum_permissions: vec!["platform.access".to_string()],
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Manager for PlatformManager {
    fn name(&self) -> &str {
        "platform_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Platform-specific initialization
        #[cfg(not(target_arch = "wasm32"))]
        native::initialize().await?;

        #[cfg(target_arch = "wasm32")]
        web::initialize().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Platform-specific cleanup
        #[cfg(not(target_arch = "wasm32"))]
        native::cleanup().await?;

        #[cfg(target_arch = "wasm32")]
        web::cleanup().await?;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        status.add_metadata(
            "platform",
            serde_json::json!(self.capabilities.platform_name),
        );
        status.add_metadata(
            "capabilities",
            serde_json::to_value(&self.capabilities).unwrap_or_default(),
        );
        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: true,
            requires_database: true,
            requires_native_apis: false,
            minimum_permissions: vec!["platform.access".to_string()],
        }
    }
}

/// Platform provider collection
pub struct PlatformProviders {
    pub filesystem: FileSystemArc,
    pub database: DatabaseArc,
    pub network: NetworkArc,
    pub storage: StorageArc,
}
