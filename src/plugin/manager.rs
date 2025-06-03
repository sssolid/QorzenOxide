use async_trait::async_trait;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use futures::FutureExt;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::{
    loader::{PluginInstallationManager, PluginStatus},
    manifest::PluginManifest,
    search::{SearchCoordinator, SearchProvider},
    Plugin, PluginApiClient, PluginContext, PluginFileSystem,
};
use crate::error::{Error, Result};
use crate::event::{EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};
use crate::platform::{filesystem::FileSystemArc, PlatformManager};
#[cfg(not(target_arch = "wasm32"))]
use reqwest::Client;

/// Plugin installation source types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallationSource {
    /// Local directory containing plugin
    Local { path: PathBuf },
    /// Git repository URL
    Git { url: String, branch: Option<String> },
    /// Plugin registry URL
    Registry {
        url: String,
        plugin_id: String,
        version: Option<String>,
    },
    /// Pre-compiled binary
    Binary { path: PathBuf },
}

/// Plugin installation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallRequest {
    pub source: InstallationSource,
    pub force_reinstall: bool,
    pub auto_enable: bool,
}

/// Plugin registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub download_url: String,
    pub checksum: String,
    pub supported_platforms: Vec<String>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub rating: Option<f32>,
    pub downloads: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Plugin registry client
#[derive(Debug)]
#[allow(unused_variables)]
pub struct PluginRegistry {
    base_url: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub client: Client,
}

impl PluginRegistry {
    /// Create a new plugin registry client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            #[cfg(not(target_arch = "wasm32"))]
            client: Client::new(),
        }
    }

    /// Search for plugins in the registry
    pub async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<RegistryPlugin>> {
        #[cfg(not(target_arch = "wasm32"))]
        let client = Client::new();
        let url = format!("{}/search", self.base_url);
        #[cfg(not(target_arch = "wasm32"))]
        let mut request = client.get(&url).query(&[("q", query)]);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(limit) = limit {
            request = request.query(&[("limit", &limit.to_string())]);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let response = request.send().await.map_err(|e| {
                Error::plugin("registry", format!("Failed to search registry: {}", e))
            })?;

            if !response.status().is_success() {
                return Err(Error::plugin(
                    "registry",
                    format!("Registry search failed: {}", response.status()),
                ));
            }

            let plugins: Vec<RegistryPlugin> = response.json().await.map_err(|e| {
                Error::plugin(
                    "registry",
                    format!("Failed to parse registry response: {}", e),
                )
            })?;

            Ok(plugins)
        }

        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, return mock data or implement web-specific fetching
            Ok(vec![RegistryPlugin {
                id: "example_plugin".to_string(),
                name: "Example Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "An example plugin for demonstration".to_string(),
                author: "Qorzen Team".to_string(),
                license: "MIT".to_string(),
                homepage: Some("https://example.com".to_string()),
                repository: Some("https://github.com/sssolid/example-plugin".to_string()),
                download_url: "https://registry.qorzen.com/plugins/example_plugin/1.0.0/download"
                    .to_string(),
                checksum: "sha256:1234567890abcdef".to_string(),
                supported_platforms: vec!["web".to_string(), "desktop".to_string()],
                dependencies: vec![],
                tags: vec!["example".to_string(), "demo".to_string()],
                rating: Some(4.5),
                downloads: 1250,
                last_updated: chrono::Utc::now(),
            }])
        }
    }

    /// Get plugin details by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<Option<RegistryPlugin>> {
        let plugins = self.search(plugin_id, Some(1)).await?;
        Ok(plugins.into_iter().find(|p| p.id == plugin_id))
    }
}

/// Enhanced plugin manager with real installation capabilities
#[derive(Debug)]
pub struct PluginManager {
    state: ManagedState,
    installation_manager: Arc<Mutex<PluginInstallationManager>>,
    search_coordinator: Arc<SearchCoordinator>,
    registry: Arc<PluginRegistry>,
    event_bus: Option<Arc<EventBusManager>>,
    platform_manager: Option<Arc<PlatformManager>>,

    // Runtime plugin state
    active_plugins: Arc<RwLock<HashMap<String, Arc<Mutex<Box<dyn Plugin>>>>>>,
    plugin_contexts: Arc<RwLock<HashMap<String, PluginContext>>>,
    search_providers: Arc<RwLock<HashMap<String, Arc<dyn SearchProvider>>>>,
    plugin_registry: Arc<RwLock<HashMap<String, PluginManifest>>>,

    // Configuration
    plugins_directory: PathBuf,
    auto_load_plugins: bool,
    hot_reload_enabled: bool,
    require_restart_after_install: bool,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(plugins_directory: PathBuf) -> Self {
        let installation_manager = Arc::new(Mutex::new(PluginInstallationManager::new(
            plugins_directory.clone(),
        )));

        let registry_url = std::env::var("QORZEN_PLUGIN_REGISTRY")
            .unwrap_or_else(|_| "https://registry.qorzen.com".to_string());

        Self {
            state: ManagedState::new(Uuid::new_v4(), "plugin_manager"),
            installation_manager,
            search_coordinator: Arc::new(SearchCoordinator::new()),
            registry: Arc::new(PluginRegistry::new(registry_url)),
            event_bus: None,
            platform_manager: None,
            active_plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_contexts: Arc::new(RwLock::new(HashMap::new())),
            search_providers: Arc::new(RwLock::new(HashMap::new())),
            plugin_registry: Arc::new(RwLock::new(HashMap::new())),
            plugins_directory,
            auto_load_plugins: true,
            hot_reload_enabled: cfg!(debug_assertions) && !cfg!(target_arch = "wasm32"),
            require_restart_after_install: !cfg!(debug_assertions),
        }
    }

    /// Set the event bus manager
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    /// Set the platform manager
    pub fn set_platform_manager(&mut self, platform_manager: Arc<PlatformManager>) {
        self.platform_manager = Some(platform_manager);
    }

    /// Search for plugins in the registry
    pub async fn search_registry(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<RegistryPlugin>> {
        self.registry.search(query, limit).await
    }

    /// Install a plugin from various sources
    pub async fn install_plugin(&self, request: PluginInstallRequest) -> Result<String> {
        tracing::info!("Installing plugin from source: {:?}", request.source);

        match request.source {
            InstallationSource::Local { path } => {
                self.install_from_local(path, request.force_reinstall).await
            }
            InstallationSource::Git { url, branch } => {
                self.install_from_git(url, branch, request.force_reinstall)
                    .await
            }
            InstallationSource::Registry {
                url: _,
                plugin_id,
                version,
            } => {
                self.install_from_registry(plugin_id, version, request.force_reinstall)
                    .await
            }
            InstallationSource::Binary { path } => {
                self.install_from_binary(path, request.force_reinstall)
                    .await
            }
        }
    }

    /// Install plugin from local directory
    async fn install_from_local(&self, path: PathBuf, force: bool) -> Result<String> {
        let manifest_path = path.join("plugin.toml");
        if !manifest_path.exists() {
            return Err(Error::plugin(
                "installer",
                "No plugin.toml found in the specified directory",
            ));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let manifest = PluginManifest::load_from_file(&manifest_path).await?;
            let plugin_id = manifest.plugin.id.clone();

            // Check if plugin already exists
            let installation_manager = self.installation_manager.lock().await;
            if let Some(_existing) = installation_manager.get_installation(&plugin_id).await {
                if !force {
                    return Err(Error::plugin(
                        &plugin_id,
                        "Plugin already installed. Use force_reinstall to override",
                    ));
                }
                drop(installation_manager);
                self.uninstall_plugin(&plugin_id).await?;
            } else {
                drop(installation_manager);
            }

            // Copy plugin to plugins directory
            let target_dir = self.plugins_directory.join(&plugin_id);
            if target_dir.exists() {
                tokio::fs::remove_dir_all(&target_dir).await.map_err(|e| {
                    Error::plugin(
                        &plugin_id,
                        format!("Failed to remove existing plugin directory: {}", e),
                    )
                })?;
            }

            self.copy_directory(&path, &target_dir).await?;

            // Register the installation
            let installation_manager = self.installation_manager.lock().await;
            installation_manager.discover_plugins().await?;

            Ok(plugin_id)
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(Error::platform(
                "wasm",
                "plugin_install",
                "Local plugin installation not supported in WASM environment",
            ))
        }
    }

    /// Install plugin from git repository
    async fn install_from_git(
        &self,
        url: String,
        branch: Option<String>,
        force: bool,
    ) -> Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let temp_dir = tempfile::tempdir().map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to create temp directory: {}", e),
                )
            })?;

            // Clone the repository
            let mut cmd = tokio::process::Command::new("git");
            cmd.args(["clone"]);

            if let Some(branch) = branch {
                cmd.args(["--branch", &branch]);
            }

            cmd.args([&url, temp_dir.path().to_str().unwrap()]);

            let output = cmd.output().await.map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to clone git repository: {}", e),
                )
            })?;

            if !output.status.success() {
                return Err(Error::plugin(
                    "installer",
                    format!(
                        "Git clone failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }

            // Install from the cloned directory
            self.install_from_local(temp_dir.path().to_path_buf(), force)
                .await
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(Error::platform(
                "wasm",
                "plugin_install",
                "Git plugin installation not supported in WASM environment",
            ))
        }
    }

    /// Install plugin from registry
    async fn install_from_registry(
        &self,
        plugin_id: String,
        version: Option<String>,
        force: bool,
    ) -> Result<String> {
        let plugin_info = self
            .registry
            .get_plugin(&plugin_id)
            .await?
            .ok_or_else(|| Error::plugin(&plugin_id, "Plugin not found in registry"))?;

        // Check version compatibility
        if let Some(requested_version) = version {
            if plugin_info.version != requested_version {
                return Err(Error::plugin(
                    &plugin_id,
                    format!(
                        "Requested version {} not available, found {}",
                        requested_version, plugin_info.version
                    ),
                ));
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Download and install
            let temp_dir = tempfile::tempdir().map_err(|e| {
                Error::plugin(
                    &plugin_id,
                    format!("Failed to create temp directory: {}", e),
                )
            })?;

            let download_path = temp_dir.path().join("plugin.tar.gz");

            // Download the plugin archive
            let response = reqwest::get(&plugin_info.download_url).await.map_err(|e| {
                Error::plugin(&plugin_id, format!("Failed to download plugin: {}", e))
            })?;

            if !response.status().is_success() {
                return Err(Error::plugin(
                    &plugin_id,
                    format!("Download failed: {}", response.status()),
                ));
            }

            let bytes = response.bytes().await.map_err(|e| {
                Error::plugin(&plugin_id, format!("Failed to read download: {}", e))
            })?;

            tokio::fs::write(&download_path, &bytes)
                .await
                .map_err(|e| {
                    Error::plugin(&plugin_id, format!("Failed to write download: {}", e))
                })?;

            // Extract the archive
            let extract_dir = temp_dir.path().join("extracted");
            tokio::fs::create_dir_all(&extract_dir).await.map_err(|e| {
                Error::plugin(
                    &plugin_id,
                    format!("Failed to create extract directory: {}", e),
                )
            })?;

            // Use tar to extract (you might want to use a pure Rust solution)
            let output = tokio::process::Command::new("tar")
                .args([
                    "-xzf",
                    download_path.to_str().unwrap(),
                    "-C",
                    extract_dir.to_str().unwrap(),
                ])
                .output()
                .await
                .map_err(|e| {
                    Error::plugin(&plugin_id, format!("Failed to extract archive: {}", e))
                })?;

            if !output.status.success() {
                return Err(Error::plugin(
                    &plugin_id,
                    format!(
                        "Extraction failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }

            // Find the plugin directory in the extracted content
            let mut plugin_dir = None;
            let mut entries = tokio::fs::read_dir(&extract_dir).await.map_err(|e| {
                Error::plugin(
                    &plugin_id,
                    format!("Failed to read extracted content: {}", e),
                )
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                Error::plugin(&plugin_id, format!("Failed to read directory entry: {}", e))
            })? {
                if entry
                    .file_type()
                    .await
                    .map_err(|e| {
                        Error::plugin(&plugin_id, format!("Failed to get file type: {}", e))
                    })?
                    .is_dir()
                {
                    let manifest_path = entry.path().join("plugin.toml");
                    if manifest_path.exists() {
                        plugin_dir = Some(entry.path());
                        break;
                    }
                }
            }

            let plugin_dir = plugin_dir.ok_or_else(|| {
                Error::plugin(&plugin_id, "No valid plugin directory found in archive")
            })?;

            // Install from the extracted directory
            self.install_from_local(plugin_dir, force).await
        }

        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, we can't actually install plugins at runtime
            // Instead, we simulate the installation for UI purposes
            tracing::info!("Simulating plugin installation for WASM: {}", plugin_id);
            Ok(plugin_id)
        }
    }

    /// Install plugin from binary
    async fn install_from_binary(&self, path: PathBuf, _force: bool) -> Result<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For now, just copy the binary to the plugins directory
            // In a real implementation, you'd want to extract metadata and validate
            let plugin_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown_plugin")
                .to_string();

            let target_path = self.plugins_directory.join(&plugin_id);
            tokio::fs::create_dir_all(&target_path).await.map_err(|e| {
                Error::plugin(
                    &plugin_id,
                    format!("Failed to create plugin directory: {}", e),
                )
            })?;

            let binary_target = target_path.join("plugin.so"); // or .dll on Windows
            tokio::fs::copy(&path, &binary_target)
                .await
                .map_err(|e| Error::plugin(&plugin_id, format!("Failed to copy binary: {}", e)))?;

            Ok(plugin_id)
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(Error::platform(
                "wasm",
                "plugin_install",
                "Binary plugin installation not supported in WASM environment",
            ))
        }
    }

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        tracing::info!("Uninstalling plugin: {}", plugin_id);

        // Stop the plugin if it's running
        if self.active_plugins.read().await.contains_key(plugin_id) {
            self.stop_plugin(plugin_id).await?;
        }

        // Remove from installation manager
        let installation_manager = self.installation_manager.lock().await;
        installation_manager.uninstall_plugin(plugin_id).await?;

        // Clean up local state
        self.plugin_contexts.write().await.remove(plugin_id);
        self.plugin_registry.write().await.remove(plugin_id);

        tracing::info!("Plugin {} uninstalled successfully", plugin_id);
        Ok(())
    }

    /// Load and start a plugin
    pub async fn load_plugin(&self, plugin_id: &str) -> Result<()> {
        tracing::info!("Loading plugin: {}", plugin_id);

        let installation_manager = self.installation_manager.lock().await;
        let installation = installation_manager
            .get_installation(plugin_id)
            .await
            .ok_or_else(|| Error::plugin(plugin_id, "Plugin not found"))?;

        if self.active_plugins.read().await.contains_key(plugin_id) {
            return Err(Error::plugin(plugin_id, "Plugin already loaded"));
        }

        let context = self.create_plugin_context(&installation.manifest).await?;
        self.plugin_contexts
            .write()
            .await
            .insert(plugin_id.to_string(), context.clone());

        let plugin = installation_manager.load_plugin(plugin_id, context).await?;
        self.active_plugins
            .write()
            .await
            .insert(plugin_id.to_string(), Arc::new(Mutex::new(plugin)));
        self.plugin_registry
            .write()
            .await
            .insert(plugin_id.to_string(), installation.manifest.clone());

        // Register search providers if any
        if let Some(search_config) = &installation.manifest.search {
            for provider_config in &search_config.providers {
                tracing::info!(
                    "Plugin {} provides search provider: {}",
                    plugin_id,
                    provider_config.id
                );
            }
        }

        installation_manager
            .update_status(plugin_id, PluginStatus::Running)
            .await;
        tracing::info!("Plugin {} loaded and started successfully", plugin_id);
        Ok(())
    }

    /// Stop a plugin
    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<()> {
        if let Some(plugin_arc) = self.active_plugins.write().await.remove(plugin_id) {
            let mut plugin = plugin_arc.lock().await;
            plugin.shutdown().await?;

            let installation_manager = self.installation_manager.lock().await;
            installation_manager
                .update_status(plugin_id, PluginStatus::Stopped)
                .await;
            tracing::info!("Plugin {} stopped successfully", plugin_id);
        }
        Ok(())
    }

    /// Get list of installed plugins
    pub async fn list_installed_plugins(&self) -> Vec<super::loader::PluginInstallation> {
        let installation_manager = self.installation_manager.lock().await;
        installation_manager.list_installations().await
    }

    /// Get list of active (loaded) plugins
    pub async fn list_active_plugins(&self) -> Vec<String> {
        self.active_plugins.read().await.keys().cloned().collect()
    }

    /// Get plugin statistics
    pub async fn get_plugin_stats(&self) -> PluginStats {
        let installations = self.list_installed_plugins().await;
        let active_count = self.active_plugins.read().await.len();
        let search_providers_count = self.search_providers.read().await.len();

        let mut status_counts = HashMap::new();
        for installation in &installations {
            *status_counts.entry(installation.status).or_insert(0) += 1;
        }

        PluginStats {
            total_plugins: installations.len(),
            active_plugins: active_count,
            search_providers: search_providers_count,
            status_counts,
        }
    }

    /// Copy directory recursively (helper method)
    #[cfg(not(target_arch = "wasm32"))]
    fn copy_directory<'a>(
        &'a self,
        src: &'a std::path::Path,
        dst: &'a std::path::Path,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            tokio::fs::create_dir_all(dst).await.map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to create target directory: {}", e),
                )
            })?;

            let mut entries = tokio::fs::read_dir(src).await.map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to read source directory: {}", e),
                )
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                Error::plugin(
                    "installer",
                    format!("Failed to read directory entry: {}", e),
                )
            })? {
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());

                if entry
                    .file_type()
                    .await
                    .map_err(|e| {
                        Error::plugin("installer", format!("Failed to get file type: {}", e))
                    })?
                    .is_dir()
                {
                    self.copy_directory(&src_path, &dst_path).await?;
                } else {
                    tokio::fs::copy(&src_path, &dst_path).await.map_err(|e| {
                        Error::plugin("installer", format!("Failed to copy file: {}", e))
                    })?;
                }
            }

            Ok(())
        }
        .boxed()
    }

    /// Create plugin context for a loaded plugin
    async fn create_plugin_context(&self, manifest: &PluginManifest) -> Result<PluginContext> {
        let plugin_id = manifest.plugin.id.clone();
        let api_client = PluginApiClient::new(plugin_id.clone());

        let event_bus = self
            .event_bus
            .as_ref()
            .ok_or_else(|| Error::plugin(&plugin_id, "Event bus not available"))?
            .clone();

        let file_system = if let Some(platform_manager) = &self.platform_manager {
            let fs_provider = platform_manager.filesystem_arc();
            PluginFileSystem::new(plugin_id.clone(), fs_provider)
        } else {
            let mock_fs: FileSystemArc = Arc::new(crate::platform::MockFileSystem::new());
            PluginFileSystem::new(plugin_id.clone(), mock_fs)
        };

        let database = if manifest.requires.contains(&"database.query".to_string()) {
            if let Some(platform_manager) = &self.platform_manager {
                let db_provider = platform_manager.database_arc();
                Some(super::PluginDatabase::new(
                    plugin_id.clone(),
                    db_provider,
                    super::DatabasePermissions {
                        can_create_tables: false,
                        can_drop_tables: false,
                        can_modify_schema: false,
                        max_table_count: Some(10),
                        max_storage_size: Some(100 * 1024 * 1024), // 100MB
                    },
                ))
            } else {
                None
            }
        } else {
            None
        };

        let config = super::PluginConfig {
            plugin_id: plugin_id.clone(),
            version: manifest.plugin.version.clone(),
            config_schema: manifest.settings.clone().unwrap_or(serde_json::json!({})),
            default_values: manifest.settings.clone().unwrap_or(serde_json::json!({})),
            user_overrides: serde_json::json!({}),
            validation_rules: vec![],
        };

        Ok(PluginContext {
            plugin_id,
            config,
            api_client,
            event_bus,
            database,
            file_system,
        })
    }

    /// Discover and auto-load plugins
    async fn discover_and_load_plugins(&self) -> Result<()> {
        let installation_manager = self.installation_manager.lock().await;
        let discovered = installation_manager.discover_plugins().await?;

        if self.auto_load_plugins {
            drop(installation_manager);
            for plugin_id in discovered {
                if let Err(e) = self.load_plugin(&plugin_id).await {
                    tracing::error!("Failed to auto-load plugin {}: {}", plugin_id, e);
                }
            }
        }

        Ok(())
    }
}

/// Plugin statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub active_plugins: usize,
    pub search_providers: usize,
    pub status_counts: HashMap<PluginStatus, usize>,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Manager for PluginManager {
    fn name(&self) -> &str {
        "plugin_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Initialize installation manager
        self.installation_manager.lock().await.initialize().await?;

        // Discover and load plugins
        self.discover_and_load_plugins().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        tracing::info!("Plugin manager initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Stop all active plugins
        let plugin_ids: Vec<String> = self.active_plugins.read().await.keys().cloned().collect();
        for plugin_id in plugin_ids {
            if let Err(e) = self.stop_plugin(&plugin_id).await {
                tracing::error!("Failed to stop plugin {}: {}", plugin_id, e);
            }
        }

        // Shutdown installation manager
        self.installation_manager.lock().await.shutdown().await?;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        tracing::info!("Plugin manager shut down");
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_plugin_stats().await;

        status.add_metadata(
            "total_plugins",
            serde_json::Value::from(stats.total_plugins),
        );
        status.add_metadata(
            "active_plugins",
            serde_json::Value::from(stats.active_plugins),
        );
        status.add_metadata(
            "search_providers",
            serde_json::Value::from(stats.search_providers),
        );
        status.add_metadata(
            "plugins_directory",
            serde_json::Value::String(self.plugins_directory.display().to_string()),
        );
        status.add_metadata(
            "auto_load_enabled",
            serde_json::Value::Bool(self.auto_load_plugins),
        );
        status.add_metadata(
            "hot_reload_enabled",
            serde_json::Value::Bool(self.hot_reload_enabled),
        );

        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec![
                "plugin.load".to_string(),
                "plugin.manage".to_string(),
                "filesystem.read".to_string(),
                "filesystem.write".to_string(),
            ],
        }
    }
}
