// src/platform/mod.rs - Core platform abstraction

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::manager::{Manager, ManagedState, ManagerStatus, PlatformRequirements};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub mod database;
pub mod filesystem;
pub mod network;
pub mod storage;

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

/// File system operations
#[async_trait]
pub trait FileSystemProvider: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn delete_file(&self, path: &str) -> Result<()>;
    async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>>;
    async fn create_directory(&self, path: &str) -> Result<()>;
    async fn file_exists(&self, path: &str) -> bool;
    async fn get_metadata(&self, path: &str) -> Result<FileMetadata>;
}

/// Database operations
#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    async fn execute(&self, query: &str, params: &[serde_json::Value]) -> Result<QueryResult>;
    async fn query(&self, query: &str, params: &[serde_json::Value]) -> Result<Vec<Row>>;
    async fn transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Transaction) -> Result<R> + Send,
        R: Send;
    async fn migrate(&self, migrations: &[Migration]) -> Result<()>;
}

/// Storage operations (key-value)
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn clear(&self) -> Result<()>;
}

/// Network operations
#[async_trait]
pub trait NetworkProvider: Send + Sync {
    async fn request(&self, request: NetworkRequest) -> Result<NetworkResponse>;
    async fn upload_file(&self, url: &str, file_data: &[u8]) -> Result<NetworkResponse>;
    async fn download_file(&self, url: &str) -> Result<Vec<u8>>;
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: chrono::DateTime<chrono::Utc>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub is_directory: bool,
    pub is_readonly: bool,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub accessed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Database query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}

/// Database row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub columns: HashMap<String, serde_json::Value>,
}

/// Database transaction
pub struct Transaction {
    // Implementation will be platform-specific
}

/// Database migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub version: u32,
    pub description: String,
    pub up_sql: String,
    pub down_sql: String,
}

/// Network request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: Option<u64>,
}

/// Network response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// Main platform manager that coordinates all platform providers
pub struct PlatformManager {
    state: ManagedState,
    filesystem: Box<dyn FileSystemProvider>,
    database: Box<dyn DatabaseProvider>,
    network: Box<dyn NetworkProvider>,
    storage: Box<dyn StorageProvider>,
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

#[async_trait]
impl Manager for PlatformManager {
    fn name(&self) -> &str {
        "platform_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::Initializing).await;

        // Platform-specific initialization
        #[cfg(not(target_arch = "wasm32"))]
        native::initialize().await?;

        #[cfg(target_arch = "wasm32")]
        web::initialize().await?;

        self.state.set_state(crate::manager::ManagerState::Running).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::ShuttingDown).await;

        // Platform-specific cleanup
        #[cfg(not(target_arch = "wasm32"))]
        native::cleanup().await?;

        #[cfg(target_arch = "wasm32")]
        web::cleanup().await?;

        self.state.set_state(crate::manager::ManagerState::Shutdown).await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        status.add_metadata("platform", serde_json::json!(self.capabilities.platform_name));
        status.add_metadata("capabilities", serde_json::to_value(&self.capabilities).unwrap_or_default());
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
    pub filesystem: Box<dyn FileSystemProvider>,
    pub database: Box<dyn DatabaseProvider>,
    pub network: Box<dyn NetworkProvider>,
    pub storage: Box<dyn StorageProvider>,
}