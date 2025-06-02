// src/plugin/loader.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use super::manifest::PluginManifest;
use super::{Plugin, PluginContext, ValidationResult};
use crate::error::{Error, Result};
use crate::manager::{ManagedState, Manager, ManagerStatus};
use crate::platform::filesystem::FileSystemProvider;

/// Plugin installation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PluginStatus {
    Discovered,
    Installing,
    Installed,
    Loading,
    Loaded,
    Running,
    Stopping,
    Stopped,
    Uninstalling,
    Failed,
}

/// Plugin installation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallation {
    pub id: String,
    pub manifest: PluginManifest,
    pub install_path: PathBuf,
    pub status: PluginStatus,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub last_loaded: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub settings: serde_json::Value,
}

/// Plugin factory function type
pub type PluginFactory = fn() -> Box<dyn Plugin>;

/// Plugin loader trait for different loading mechanisms
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait PluginLoader: Send + Sync + std::fmt::Debug {
    /// Load a plugin from the given installation
    async fn load_plugin(&self, installation: &PluginInstallation) -> Result<Box<dyn Plugin>>;

    /// Validate a plugin before loading
    async fn validate_plugin(&self, installation: &PluginInstallation) -> Result<ValidationResult>;

    /// Unload a plugin
    async fn unload_plugin(&self, plugin_id: &str) -> Result<()>;

    /// Check if hot reloading is supported
    #[allow(dead_code)]
    fn supports_hot_reload(&self) -> bool;

    /// Hot reload a plugin
    #[allow(dead_code)]
    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>>;
}

/// Safe plugin loader using registered factories
#[derive(Debug)]
pub struct SafePluginLoader {
    registered_plugins: Arc<Mutex<HashMap<String, PluginFactory>>>,
    loaded_plugins: Arc<Mutex<HashMap<String, String>>>, // plugin_id -> factory_name
}

impl SafePluginLoader {
    /// Create a new safe plugin loader
    pub fn new() -> Self {
        Self {
            registered_plugins: Arc::new(Mutex::new(HashMap::new())),
            loaded_plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a plugin factory
    #[allow(dead_code)]
    pub async fn register_plugin_factory(&self, name: String, factory: PluginFactory) {
        let mut registered = self.registered_plugins.lock().await;
        registered.insert(name, factory);
    }

    /// Get available plugin factories
    #[allow(dead_code)]
    pub async fn list_available_plugins(&self) -> Vec<String> {
        let registered = self.registered_plugins.lock().await;
        registered.keys().cloned().collect()
    }
}

impl Default for SafePluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PluginLoader for SafePluginLoader {
    async fn load_plugin(&self, installation: &PluginInstallation) -> Result<Box<dyn Plugin>> {
        let registered = self.registered_plugins.lock().await;

        // Try to find a factory for this plugin
        if let Some(factory) = registered.get(&installation.id) {
            let plugin = factory();

            // Store the loaded plugin reference
            let mut loaded = self.loaded_plugins.lock().await;
            loaded.insert(installation.id.clone(), installation.id.clone());

            Ok(plugin)
        } else {
            Err(Error::plugin(
                &installation.id,
                "Plugin factory not found. Plugin must be registered at compile time.",
            ))
        }
    }

    async fn validate_plugin(&self, installation: &PluginInstallation) -> Result<ValidationResult> {
        let registered = self.registered_plugins.lock().await;

        if registered.contains_key(&installation.id) {
            // Basic validation for manifest structure
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            if installation.manifest.plugin.id.is_empty() {
                errors.push("Plugin ID is empty".to_string());
            }

            if installation.manifest.plugin.name.is_empty() {
                warnings.push("Plugin name is empty".to_string());
            }

            if installation.manifest.plugin.version.is_empty() {
                errors.push("Plugin version is empty".to_string());
            }

            // For native platforms, check if the plugin directory exists
            #[cfg(not(target_arch = "wasm32"))]
            {
                if !installation.install_path.exists() {
                    errors.push("Plugin installation directory does not exist".to_string());
                }
            }

            // For WASM, we can't check file system directly
            #[cfg(target_arch = "wasm32")]
            {
                // Add WASM-specific validation warnings
                warnings.push("File system validation skipped in WASM environment".to_string());
            }

            Ok(ValidationResult {
                is_valid: errors.is_empty(),
                errors,
                warnings,
            })
        } else {
            Ok(ValidationResult {
                is_valid: false,
                errors: vec![format!("Plugin '{}' is not registered", installation.id)],
                warnings: vec![],
            })
        }
    }

    async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut loaded = self.loaded_plugins.lock().await;
        loaded.remove(plugin_id);

        // In a real implementation, we might need to do additional cleanup
        // For safety, we can't actually unload code that's been compiled in

        Ok(())
    }

    fn supports_hot_reload(&self) -> bool {
        false // Safe loading doesn't support hot reload
    }

    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>> {
        Err(Error::plugin(
            plugin_id,
            "Hot reload not supported with safe plugin loading",
        ))
    }
}

/// WASM plugin loader for web environment
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct WasmPluginLoader {
    loaded_modules: Arc<RwLock<HashMap<String, String>>>, // Just track loaded plugin IDs
    registered_plugins: Arc<Mutex<HashMap<String, PluginFactory>>>,
}

#[cfg(target_arch = "wasm32")]
impl WasmPluginLoader {
    /// Create a new WASM plugin loader
    pub fn new() -> Self {
        Self {
            loaded_modules: Arc::new(RwLock::new(HashMap::new())),
            registered_plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a plugin factory for WASM
    pub async fn register_plugin_factory(&self, name: String, factory: PluginFactory) {
        let mut registered = self.registered_plugins.lock().await;
        registered.insert(name, factory);
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WasmPluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl PluginLoader for WasmPluginLoader {
    async fn load_plugin(&self, installation: &PluginInstallation) -> Result<Box<dyn Plugin>> {
        let registered = self.registered_plugins.lock().await;

        if let Some(factory) = registered.get(&installation.id) {
            let plugin = factory();

            // Track the loaded plugin
            let mut modules = self.loaded_modules.write().await;
            modules.insert(installation.id.clone(), installation.id.clone());

            Ok(plugin)
        } else {
            Err(Error::plugin(
                &installation.id,
                "Plugin factory not found for WASM environment",
            ))
        }
    }

    async fn validate_plugin(&self, installation: &PluginInstallation) -> Result<ValidationResult> {
        let registered = self.registered_plugins.lock().await;

        if registered.contains_key(&installation.id) {
            Ok(ValidationResult {
                is_valid: true,
                errors: vec![],
                warnings: vec!["WASM plugin validation is basic".to_string()],
            })
        } else {
            Ok(ValidationResult {
                is_valid: false,
                errors: vec![format!(
                    "Plugin '{}' not registered for WASM",
                    installation.id
                )],
                warnings: vec![],
            })
        }
    }

    async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut modules = self.loaded_modules.write().await;
        modules.remove(plugin_id);
        Ok(())
    }

    fn supports_hot_reload(&self) -> bool {
        false // WASM hot reload is complex and not supported in this implementation
    }

    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>> {
        Err(Error::plugin(plugin_id, "WASM hot reload not supported"))
    }
}

/// Plugin installation manager
#[derive(Debug)]
pub struct PluginInstallationManager {
    state: ManagedState,
    installations: Arc<RwLock<HashMap<String, PluginInstallation>>>,
    plugin_loader: Arc<dyn PluginLoader>,
    plugins_directory: PathBuf,
    #[allow(dead_code)]
    filesystem_provider: Option<Arc<dyn FileSystemProvider>>,
}

#[allow(dead_code)]
impl PluginInstallationManager {
    /// Create a new plugin installation manager
    pub fn new(plugins_directory: PathBuf) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let loader: Arc<dyn PluginLoader> = Arc::new(SafePluginLoader::new());

        #[cfg(target_arch = "wasm32")]
        let loader: Arc<dyn PluginLoader> = Arc::new(WasmPluginLoader::new());

        Self {
            state: ManagedState::new(Uuid::new_v4(), "plugin_installation_manager"),
            installations: Arc::new(RwLock::new(HashMap::new())),
            plugin_loader: loader,
            plugins_directory,
            filesystem_provider: None,
        }
    }

    /// Create with a custom loader and filesystem provider
    pub fn with_loader_and_filesystem(
        plugins_directory: PathBuf,
        loader: Arc<dyn PluginLoader>,
        filesystem_provider: Arc<dyn FileSystemProvider>,
    ) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "plugin_installation_manager"),
            installations: Arc::new(RwLock::new(HashMap::new())),
            plugin_loader: loader,
            plugins_directory,
            filesystem_provider: Some(filesystem_provider),
        }
    }

    /// Set filesystem provider (required for WASM)
    pub fn set_filesystem_provider(&mut self, provider: Arc<dyn FileSystemProvider>) {
        self.filesystem_provider = Some(provider);
    }

    /// Discover plugins in the plugins directory
    pub async fn discover_plugins(&self) -> Result<Vec<String>> {
        #[allow(unused_assignments)]
        let mut discovered = Vec::new();

        // Use platform-specific discovery
        #[cfg(not(target_arch = "wasm32"))]
        {
            discovered = self.discover_plugins_native().await?;
        }

        #[cfg(target_arch = "wasm32")]
        {
            discovered = self.discover_plugins_wasm().await?;
        }

        Ok(discovered)
    }

    /// Native plugin discovery using direct file system access
    #[cfg(not(target_arch = "wasm32"))]
    async fn discover_plugins_native(&self) -> Result<Vec<String>> {
        use tokio::fs;

        let mut discovered = Vec::new();

        if !self.plugins_directory.exists() {
            fs::create_dir_all(&self.plugins_directory)
                .await
                .map_err(|e| {
                    Error::file(
                        self.plugins_directory.display().to_string(),
                        crate::error::FileOperation::CreateDirectory,
                        format!("Failed to create plugins directory: {}", e),
                    )
                })?;
            return Ok(discovered);
        }

        let mut dir_entries = fs::read_dir(&self.plugins_directory).await.map_err(|e| {
            Error::file(
                self.plugins_directory.display().to_string(),
                crate::error::FileOperation::Read,
                format!("Failed to read plugins directory: {}", e),
            )
        })?;

        while let Some(entry) = dir_entries.next_entry().await.map_err(|e| {
            Error::file(
                self.plugins_directory.display().to_string(),
                crate::error::FileOperation::Read,
                format!("Failed to read directory entry: {}", e),
            )
        })? {
            if entry
                .file_type()
                .await
                .map_err(|e| {
                    Error::file(
                        entry.path().display().to_string(),
                        crate::error::FileOperation::Metadata,
                        format!("Failed to get file type: {}", e),
                    )
                })?
                .is_dir()
            {
                let plugin_dir = entry.path();
                let manifest_path = plugin_dir.join("plugin.toml");

                if manifest_path.exists() {
                    if let Ok(manifest) = PluginManifest::load_from_file(&manifest_path).await {
                        let installation = PluginInstallation {
                            id: manifest.plugin.id.clone(),
                            manifest,
                            install_path: plugin_dir,
                            status: PluginStatus::Discovered,
                            installed_at: chrono::Utc::now(),
                            last_loaded: None,
                            error_message: None,
                            settings: serde_json::Value::Object(serde_json::Map::new()),
                        };

                        self.installations
                            .write()
                            .await
                            .insert(installation.id.clone(), installation);

                        discovered.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// WASM plugin discovery using platform filesystem provider
    #[cfg(target_arch = "wasm32")]
    async fn discover_plugins_wasm(&self) -> Result<Vec<String>> {
        let mut discovered = Vec::new();

        if let Some(ref fs_provider) = self.filesystem_provider {
            let plugins_path = self.plugins_directory.to_string_lossy();

            // Try to list the plugins directory
            match fs_provider.list_directory(&plugins_path).await {
                Ok(entries) => {
                    for entry in entries {
                        if entry.is_directory {
                            let manifest_path =
                                format!("{}/{}/plugin.toml", plugins_path, entry.name);

                            // Try to load the manifest
                            if let Ok(manifest) = PluginManifest::load_from_platform(
                                &manifest_path,
                                fs_provider.as_ref(),
                            )
                            .await
                            {
                                let installation = PluginInstallation {
                                    id: manifest.plugin.id.clone(),
                                    manifest,
                                    install_path: PathBuf::from(&entry.path),
                                    status: PluginStatus::Discovered,
                                    installed_at: chrono::Utc::now(),
                                    last_loaded: None,
                                    error_message: None,
                                    settings: serde_json::Value::Object(serde_json::Map::new()),
                                };

                                self.installations
                                    .write()
                                    .await
                                    .insert(installation.id.clone(), installation);

                                discovered.push(entry.name);
                            }
                        }
                    }
                }
                Err(_) => {
                    // Plugins directory doesn't exist or can't be read in WASM
                    // This is normal - return empty list
                }
            }
        } else {
            tracing::warn!("No filesystem provider set for WASM plugin discovery");
        }

        Ok(discovered)
    }

    /// Install a plugin from a package file or URL
    pub async fn install_plugin(&self, source: &str, _force: bool) -> Result<String> {
        // This is a simplified implementation
        // In a real system, this would handle downloading, extracting, and validating plugins
        Err(Error::plugin(
            "installer",
            format!("Plugin installation from '{}' not implemented", source),
        ))
    }

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut installations = self.installations.write().await;

        if let Some(installation) = installations.get_mut(plugin_id) {
            installation.status = PluginStatus::Uninstalling;

            // Unload the plugin first
            self.plugin_loader.unload_plugin(plugin_id).await?;

            // Remove the plugin directory (platform-specific)
            #[cfg(not(target_arch = "wasm32"))]
            {
                if installation.install_path.exists() {
                    tokio::fs::remove_dir_all(&installation.install_path)
                        .await
                        .map_err(|e| {
                            Error::file(
                                installation.install_path.display().to_string(),
                                crate::error::FileOperation::Delete,
                                format!("Failed to remove plugin directory: {}", e),
                            )
                        })?;
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                // In WASM, we would need to use the filesystem provider to remove files
                if let Some(ref fs_provider) = self.filesystem_provider {
                    let plugin_path = installation.install_path.to_string_lossy();
                    let _ = fs_provider.delete_file(&plugin_path).await;
                }
            }

            installations.remove(plugin_id);
        }

        Ok(())
    }

    /// Load a plugin
    pub async fn load_plugin(
        &self,
        plugin_id: &str,
        context: PluginContext,
    ) -> Result<Box<dyn Plugin>> {
        let mut installations = self.installations.write().await;

        if let Some(installation) = installations.get_mut(plugin_id) {
            installation.status = PluginStatus::Loading;

            // Validate plugin first
            let validation = self.plugin_loader.validate_plugin(installation).await?;
            if !validation.is_valid {
                installation.status = PluginStatus::Failed;
                installation.error_message = Some(validation.errors.join("; "));
                return Err(Error::plugin(
                    plugin_id,
                    format!("Plugin validation failed: {:?}", validation.errors),
                ));
            }

            // Load the plugin
            match self.plugin_loader.load_plugin(installation).await {
                Ok(mut plugin) => {
                    // Initialize the plugin
                    plugin.initialize(context).await?;

                    installation.status = PluginStatus::Loaded;
                    installation.last_loaded = Some(chrono::Utc::now());
                    installation.error_message = None;

                    Ok(plugin)
                }
                Err(e) => {
                    installation.status = PluginStatus::Failed;
                    installation.error_message = Some(e.to_string());
                    Err(e)
                }
            }
        } else {
            Err(Error::plugin(plugin_id, "Plugin not found"))
        }
    }

    /// Get plugin installation info
    pub async fn get_installation(&self, plugin_id: &str) -> Option<PluginInstallation> {
        self.installations.read().await.get(plugin_id).cloned()
    }

    /// List all plugin installations
    pub async fn list_installations(&self) -> Vec<PluginInstallation> {
        self.installations.read().await.values().cloned().collect()
    }

    /// Update plugin status
    pub async fn update_status(&self, plugin_id: &str, status: PluginStatus) {
        if let Some(installation) = self.installations.write().await.get_mut(plugin_id) {
            installation.status = status;
        }
    }

    /// Update plugin settings
    pub async fn update_settings(
        &self,
        plugin_id: &str,
        settings: serde_json::Value,
    ) -> Result<()> {
        if let Some(installation) = self.installations.write().await.get_mut(plugin_id) {
            installation.settings = settings;

            // Save settings to file (platform-specific)
            let settings_json = serde_json::to_string_pretty(&installation.settings)
                .map_err(|e| Error::new(crate::error::ErrorKind::Serialization, e.to_string()))?;

            #[cfg(not(target_arch = "wasm32"))]
            {
                let settings_path = installation.install_path.join("settings.json");
                tokio::fs::write(settings_path, settings_json)
                    .await
                    .map_err(|e| {
                        Error::file(
                            installation
                                .install_path
                                .join("settings.json")
                                .display()
                                .to_string(),
                            crate::error::FileOperation::Write,
                            format!("Failed to write settings: {}", e),
                        )
                    })?;
            }

            #[cfg(target_arch = "wasm32")]
            {
                if let Some(ref fs_provider) = self.filesystem_provider {
                    let settings_path =
                        format!("{}/settings.json", installation.install_path.display());
                    fs_provider
                        .write_file(&settings_path, settings_json.as_bytes())
                        .await
                        .map_err(|e| {
                            Error::file(
                                settings_path,
                                crate::error::FileOperation::Write,
                                format!("Failed to write settings via platform: {}", e),
                            )
                        })?;
                }
            }

            Ok(())
        } else {
            Err(Error::plugin(plugin_id, "Plugin not found"))
        }
    }

    /// Get the plugin loader
    pub fn loader(&self) -> Arc<dyn PluginLoader> {
        Arc::clone(&self.plugin_loader)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Manager for PluginInstallationManager {
    fn name(&self) -> &str {
        "plugin_installation_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Discover existing plugins
        self.discover_plugins().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Unload all plugins
        let plugin_ids: Vec<String> = self.installations.read().await.keys().cloned().collect();
        for plugin_id in plugin_ids {
            let _ = self.plugin_loader.unload_plugin(&plugin_id).await;
        }

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let installations = self.installations.read().await;

        status.add_metadata(
            "total_plugins",
            serde_json::Value::from(installations.len()),
        );

        let loaded_count = installations
            .values()
            .filter(|i| matches!(i.status, PluginStatus::Loaded | PluginStatus::Running))
            .count();
        status.add_metadata("loaded_plugins", serde_json::Value::from(loaded_count));

        let failed_count = installations
            .values()
            .filter(|i| i.status == PluginStatus::Failed)
            .count();
        status.add_metadata("failed_plugins", serde_json::Value::from(failed_count));

        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().to_path_buf();

        let manager = PluginInstallationManager::new(plugins_dir);
        let discovered = manager.discover_plugins().await.unwrap();
        assert!(discovered.is_empty());
    }

    #[tokio::test]
    async fn test_installation_manager_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let plugins_dir = temp_dir.path().to_path_buf();

        let mut manager = PluginInstallationManager::new(plugins_dir);
        manager.initialize().await.unwrap();

        let status = manager.status().await;
        assert_eq!(status.state, crate::manager::ManagerState::Running);

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_safe_plugin_loader() {
        let loader = SafePluginLoader::new();

        // Test that we can list available plugins (should be empty initially)
        let available = loader.list_available_plugins().await;
        assert!(available.is_empty());

        // Test validation without registered plugin
        let installation = PluginInstallation {
            id: "test_plugin".to_string(),
            manifest: PluginManifest::example(),
            install_path: PathBuf::from("/tmp/test"),
            status: PluginStatus::Discovered,
            installed_at: chrono::Utc::now(),
            last_loaded: None,
            error_message: None,
            settings: serde_json::json!({}),
        };

        let validation = loader.validate_plugin(&installation).await.unwrap();
        assert!(!validation.is_valid);
    }
}
