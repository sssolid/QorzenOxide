use std::any::Any;
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

/// Plugin status enumeration
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

impl std::fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovered => write!(f, "Discovered"),
            Self::Installing => write!(f, "Installing"),
            Self::Installed => write!(f, "Installed"),
            Self::Loading => write!(f, "Loading"),
            Self::Loaded => write!(f, "Loaded"),
            Self::Running => write!(f, "Running"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Uninstalling => write!(f, "Uninstalling"),
            Self::Failed => write!(f, "Failed"),
        }
    }
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

/// Plugin loader trait for different loading strategies
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait PluginLoader: Send + Sync + std::fmt::Debug {
    /// Load a plugin from an installation
    async fn load_plugin(&self, installation: &PluginInstallation) -> Result<Box<dyn Plugin>>;

    /// Validate a plugin installation
    async fn validate_plugin(&self, installation: &PluginInstallation) -> Result<ValidationResult>;

    /// Unload a plugin by ID
    async fn unload_plugin(&self, plugin_id: &str) -> Result<()>;

    /// Check if hot reload is supported
    fn supports_hot_reload(&self) -> bool;

    /// Hot reload a plugin (if supported)
    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>>;

    fn as_any(&self) -> &dyn Any;
}

/// Safe plugin loader that uses compile-time registration
#[derive(Debug)]
pub struct SafePluginLoader {
    registered_plugins: Arc<Mutex<HashMap<String, PluginFactory>>>,
    loaded_plugins: Arc<Mutex<HashMap<String, String>>>,
}

impl SafePluginLoader {
    /// Create a new safe plugin loader
    pub fn new() -> Self {
        Self {
            registered_plugins: Arc::new(Mutex::new(HashMap::new())),
            loaded_plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a plugin factory function
    pub async fn register_plugin_factory(&self, name: String, factory: PluginFactory) {
        let mut registered = self.registered_plugins.lock().await;
        registered.insert(name.clone(), factory);
        tracing::info!("Registered plugin factory: {}", name);
    }

    /// List available plugin names
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

        if let Some(factory) = registered.get(&installation.id) {
            let plugin = factory();

            let mut loaded = self.loaded_plugins.lock().await;
            loaded.insert(installation.id.clone(), installation.id.clone());

            tracing::info!("Loaded plugin using factory: {}", installation.id);
            Ok(plugin)
        } else {
            Err(Error::plugin(
                &installation.id,
                "Plugin factory not found. Plugin must be registered at compile time."
            ))
        }
    }

    async fn validate_plugin(&self, installation: &PluginInstallation) -> Result<ValidationResult> {
        let registered = self.registered_plugins.lock().await;

        if registered.contains_key(&installation.id) {
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            // Basic manifest validation
            if installation.manifest.plugin.id.is_empty() {
                errors.push("Plugin ID is empty".to_string());
            }

            if installation.manifest.plugin.name.is_empty() {
                warnings.push("Plugin name is empty".to_string());
            }

            if installation.manifest.plugin.version.is_empty() {
                errors.push("Plugin version is empty".to_string());
            }

            // Platform-specific validation
            #[cfg(not(target_arch = "wasm32"))]
            {
                if !installation.install_path.exists() {
                    errors.push("Plugin installation directory does not exist".to_string());
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
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
        tracing::info!("Unloaded plugin: {}", plugin_id);
        Ok(())
    }

    fn supports_hot_reload(&self) -> bool {
        false // Safe loading doesn't support hot reload
    }

    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>> {
        Err(Error::plugin(
            plugin_id,
            "Hot reload not supported with safe plugin loading"
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dynamic plugin loader for native platforms
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct DynamicPluginLoader {
    loaded_libraries: Arc<Mutex<HashMap<String, libloading::Library>>>,
    loaded_plugins: Arc<Mutex<HashMap<String, String>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl DynamicPluginLoader {
    /// Create a new dynamic plugin loader
    pub fn new() -> Self {
        Self {
            loaded_libraries: Arc::new(Mutex::new(HashMap::new())),
            loaded_plugins: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Find plugin library file in installation directory
    fn find_plugin_library(&self, installation: &PluginInstallation) -> Result<PathBuf> {
        let install_path = &installation.install_path;

        // Common library extensions by platform
        let extensions = if cfg!(windows) {
            vec!["dll"]
        } else if cfg!(target_os = "macos") {
            vec!["dylib", "so"]
        } else {
            vec!["so"]
        };

        // Look for library files
        for extension in extensions {
            let lib_name = format!("lib{}.{}", installation.id, extension);
            let lib_path = install_path.join(&lib_name);
            if lib_path.exists() {
                return Ok(lib_path);
            }

            // Also try without 'lib' prefix
            let lib_name = format!("{}.{}", installation.id, extension);
            let lib_path = install_path.join(&lib_name);
            if lib_path.exists() {
                return Ok(lib_path);
            }
        }

        Err(Error::plugin(
            &installation.id,
            "No plugin library found in installation directory"
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
#[allow(unsafe_code)]
impl PluginLoader for DynamicPluginLoader {
    async fn load_plugin(&self, installation: &PluginInstallation) -> Result<Box<dyn Plugin>> {
        let lib_path = self.find_plugin_library(installation)?;

        // Load the dynamic library
        let library = unsafe {
            libloading::Library::new(&lib_path).map_err(|e| {
                Error::plugin(
                    &installation.id,
                    format!("Failed to load plugin library: {}", e)
                )
            })?
        };

        // Get the plugin creation function
        let create_plugin: libloading::Symbol<extern "C" fn() -> *mut dyn Plugin> = unsafe {
            library.get(b"create_plugin").map_err(|e| {
                Error::plugin(
                    &installation.id,
                    format!("Failed to find create_plugin symbol: {}", e)
                )
            })?
        };

        // Create the plugin instance
        let plugin_ptr = create_plugin();
        let plugin = unsafe { Box::from_raw(plugin_ptr) };

        // Store the library to keep it loaded
        self.loaded_libraries
            .lock()
            .await
            .insert(installation.id.clone(), library);

        self.loaded_plugins
            .lock()
            .await
            .insert(installation.id.clone(), installation.id.clone());

        tracing::info!("Dynamically loaded plugin: {}", installation.id);
        Ok(plugin)
    }

    async fn validate_plugin(&self, installation: &PluginInstallation) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if library file exists
        match self.find_plugin_library(installation) {
            Ok(_) => {
                // Library exists, try to validate symbols
                warnings.push("Dynamic symbol validation not implemented".to_string());
            }
            Err(_) => {
                errors.push("Plugin library file not found".to_string());
            }
        }

        // Basic manifest validation
        if installation.manifest.plugin.id.is_empty() {
            errors.push("Plugin ID is empty".to_string());
        }

        if installation.manifest.plugin.version.is_empty() {
            errors.push("Plugin version is empty".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        // Remove from loaded plugins
        self.loaded_plugins.lock().await.remove(plugin_id);

        // Unload the library
        if let Some(_library) = self.loaded_libraries.lock().await.remove(plugin_id) {
            // Library will be dropped and unloaded automatically
            tracing::info!("Unloaded dynamic plugin: {}", plugin_id);
        }

        Ok(())
    }

    fn supports_hot_reload(&self) -> bool {
        true
    }

    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>> {
        // First unload the existing plugin
        self.unload_plugin(plugin_id).await?;

        // This would require reloading the installation info
        // For now, return an error indicating more context is needed
        Err(Error::plugin(
            plugin_id,
            "Hot reload requires installation context"
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// WASM-specific plugin loader
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct WasmPluginLoader {
    loaded_modules: Arc<RwLock<HashMap<String, String>>>,
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

    /// Register a plugin factory (required for WASM)
    pub async fn register_plugin_factory(&self, name: String, factory: PluginFactory) {
        let mut registered = self.registered_plugins.lock().await;
        registered.insert(name.clone(), factory);
        tracing::info!("Registered WASM plugin factory: {}", name);
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

            let mut modules = self.loaded_modules.write().await;
            modules.insert(installation.id.clone(), installation.id.clone());

            tracing::info!("Loaded WASM plugin: {}", installation.id);
            Ok(plugin)
        } else {
            Err(Error::plugin(
                &installation.id,
                "Plugin factory not found for WASM environment"
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
                errors: vec![format!("Plugin '{}' not registered for WASM", installation.id)],
                warnings: vec![],
            })
        }
    }

    async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut modules = self.loaded_modules.write().await;
        modules.remove(plugin_id);
        tracing::info!("Unloaded WASM plugin: {}", plugin_id);
        Ok(())
    }

    fn supports_hot_reload(&self) -> bool {
        false // WASM doesn't support hot reload
    }

    async fn hot_reload_plugin(&self, plugin_id: &str) -> Result<Box<dyn Plugin>> {
        Err(Error::plugin(plugin_id, "WASM hot reload not supported"))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Plugin installation manager
#[derive(Debug)]
pub struct PluginInstallationManager {
    state: ManagedState,
    installations: Arc<RwLock<HashMap<String, PluginInstallation>>>,
    plugin_loader: Arc<dyn PluginLoader>,
    plugins_directory: PathBuf,
    filesystem_provider: Option<Arc<dyn FileSystemProvider>>,
}

impl PluginInstallationManager {
    /// Create a new plugin installation manager
    pub fn new(plugins_directory: PathBuf) -> Self {
        // Choose loader based on platform and configuration
        #[cfg(not(target_arch = "wasm32"))]
        let loader: Arc<dyn PluginLoader> = if std::env::var("QORZEN_SAFE_PLUGINS").is_ok() {
            Arc::new(SafePluginLoader::new())
        } else {
            Arc::new(DynamicPluginLoader::new())
        };

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

    /// Set filesystem provider for WASM environments
    pub fn set_filesystem_provider(&mut self, provider: Arc<dyn FileSystemProvider>) {
        self.filesystem_provider = Some(provider);
    }

    /// Get the plugin loader
    pub fn loader(&self) -> Arc<dyn PluginLoader> {
        Arc::clone(&self.plugin_loader)
    }

    /// Discover plugins in the plugins directory
    pub async fn discover_plugins(&self) -> Result<Vec<String>> {
        #[allow(unused_assignments)]
        let mut discovered = Vec::new();

        #[cfg(not(target_arch = "wasm32"))]
        {
            discovered = self.discover_plugins_native().await?;
        }

        #[cfg(target_arch = "wasm32")]
        {
            discovered = self.discover_plugins_wasm().await?;
        }

        tracing::info!("Discovered {} plugins", discovered.len());
        Ok(discovered)
    }

    /// Native plugin discovery
    #[cfg(not(target_arch = "wasm32"))]
    async fn discover_plugins_native(&self) -> Result<Vec<String>> {
        use tokio::fs;

        let mut discovered = Vec::new();

        if !self.plugins_directory.exists() {
            fs::create_dir_all(&self.plugins_directory).await.map_err(|e| {
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
            if entry.file_type().await.map_err(|e| {
                Error::file(
                    entry.path().display().to_string(),
                    crate::error::FileOperation::Metadata,
                    format!("Failed to get file type: {}", e),
                )
            })?.is_dir() {
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

    /// WASM plugin discovery (uses registered plugins)
    #[cfg(target_arch = "wasm32")]
    async fn discover_plugins_wasm(&self) -> Result<Vec<String>> {
        // For WASM, we need to discover from registered plugins
        if let Some(_wasm_loader) = self.plugin_loader.as_any().downcast_ref::<WasmPluginLoader>() {
            // This would require exposing list_available_plugins from WasmPluginLoader
            // For now, return empty list
            Ok(vec![])
        } else {
            Ok(vec![])
        }
    }

    /// Load a plugin
    pub async fn load_plugin(&self, plugin_id: &str, context: PluginContext) -> Result<Box<dyn Plugin>> {
        let mut installations = self.installations.write().await;

        if let Some(installation) = installations.get_mut(plugin_id) {
            installation.status = PluginStatus::Loading;

            // Validate the plugin first
            let validation = self.plugin_loader.validate_plugin(installation).await?;
            if !validation.is_valid {
                installation.status = PluginStatus::Failed;
                installation.error_message = Some(validation.errors.join("; "));
                return Err(Error::plugin(
                    plugin_id,
                    format!("Plugin validation failed: {:?}", validation.errors)
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

    /// Get installation info for a plugin
    pub async fn get_installation(&self, plugin_id: &str) -> Option<PluginInstallation> {
        self.installations.read().await.get(plugin_id).cloned()
    }

    /// List all installations
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
    pub async fn update_settings(&self, plugin_id: &str, settings: serde_json::Value) -> Result<()> {
        if let Some(installation) = self.installations.write().await.get_mut(plugin_id) {
            installation.settings = settings;

            // Save settings to file
            let settings_json = serde_json::to_string_pretty(&installation.settings)
                .map_err(|e| Error::new(crate::error::ErrorKind::Serialization, e.to_string()))?;

            #[cfg(not(target_arch = "wasm32"))]
            {
                let settings_path = installation.install_path.join("settings.json");
                tokio::fs::write(settings_path, settings_json).await.map_err(|e| {
                    Error::file(
                        installation.install_path.join("settings.json").display().to_string(),
                        crate::error::FileOperation::Write,
                        format!("Failed to write settings: {}", e),
                    )
                })?;
            }

            #[cfg(target_arch = "wasm32")]
            {
                if let Some(ref fs_provider) = self.filesystem_provider {
                    let settings_path = format!("{}/settings.json", installation.install_path.display());
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

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut installations = self.installations.write().await;

        if let Some(installation) = installations.get_mut(plugin_id) {
            installation.status = PluginStatus::Uninstalling;

            // Unload from loader first
            self.plugin_loader.unload_plugin(plugin_id).await?;

            // Remove plugin directory
            #[cfg(not(target_arch = "wasm32"))]
            {
                if installation.install_path.exists() {
                    tokio::fs::remove_dir_all(&installation.install_path).await.map_err(|e| {
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
                if let Some(ref fs_provider) = self.filesystem_provider {
                    let plugin_path = installation.install_path.to_string_lossy();
                    let _ = fs_provider.delete_file(&plugin_path).await;
                }
            }

            installations.remove(plugin_id);
        }

        Ok(())
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
        self.state.set_state(crate::manager::ManagerState::Initializing).await;

        // Discover existing plugins
        self.discover_plugins().await?;

        self.state.set_state(crate::manager::ManagerState::Running).await;
        tracing::info!("Plugin installation manager initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::ShuttingDown).await;

        // Unload all plugins
        let plugin_ids: Vec<String> = self.installations.read().await.keys().cloned().collect();
        for plugin_id in plugin_ids {
            if let Err(e) = self.plugin_loader.unload_plugin(&plugin_id).await {
                tracing::error!("Failed to unload plugin {}: {}", plugin_id, e);
            }
        }

        self.state.set_state(crate::manager::ManagerState::Shutdown).await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let installations = self.installations.read().await;

        status.add_metadata("total_plugins", serde_json::Value::from(installations.len()));

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