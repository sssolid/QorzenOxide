// src/plugin/mod.rs - Main plugin system module with fixed re-exports

pub mod config;
mod loader;
mod manager;
mod manifest;
mod registry;
mod sdk;
pub mod search;

// Re-export core types and traits with specific imports to avoid conflicts
pub use config::*;
pub use loader::{PluginInstallation, PluginLoader, PluginStatus, SafePluginLoader, PluginInstallationManager};
pub use manager::{PluginManager, PluginInstallRequest, InstallationSource, PluginRegistry, PluginStats, RegistryPlugin};
pub use manifest::{PluginManifest, PluginMetadata as ManifestMetadata, BuildConfig, TargetConfig, DependencySpec, SearchConfig as ManifestSearchConfig};
pub use registry::{PluginFactory, PluginFactoryRegistry, SimplePluginFactory, builtin};
pub use sdk::*;
pub use search::{SearchCoordinator, SearchProvider as PluginSearchProvider, SearchQuery, SearchResult, SearchResponse};

// Re-export with aliases to avoid conflicts
pub use registry::PluginFactory as RegistryPluginFactory;
pub use search::SearchProvider as SearchProviderTrait;

use crate::auth::{Permission};
use crate::config::SettingsSchema;
use crate::error::{Error, Result};
use crate::event::Event;
use async_trait::async_trait;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin information structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub minimum_core_version: String,
    pub supported_platforms: Vec<Platform>,
}

/// Supported platforms for plugins
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    IOS,
    Android,
    Web,
    All,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_requirement: String,
    pub optional: bool,
}

/// Plugin configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_id: String,
    pub version: String,
    pub config_schema: serde_json::Value,
    pub default_values: serde_json::Value,
    pub user_overrides: serde_json::Value,
    pub validation_rules: Vec<ValidationRule>,
}

/// Configuration validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationType,
    pub message: String,
}

/// Validation rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Range { min: f64, max: f64 },
    Custom(String),
}

/// UI component provided by a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponent {
    pub id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub props: serde_json::Value,
    pub required_permissions: Vec<Permission>,
}

/// Types of UI components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    Page,
    Widget,
    Modal,
    Sidebar,
    Header,
    Footer,
    Menu,
    Form,
}

/// Menu item for navigation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub route: Option<String>,
    pub action: Option<String>,
    pub required_permissions: Vec<Permission>,
    pub order: i32,
    pub children: Vec<MenuItem>,
}

/// API route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRoute {
    pub path: String,
    pub method: HttpMethod,
    pub handler_id: String,
    pub required_permissions: Vec<Permission>,
    pub rate_limit: Option<RateLimit>,
    pub documentation: ApiDocumentation,
}

/// HTTP methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_limit: u32,
}

/// API documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocumentation {
    pub summary: String,
    pub description: String,
    pub parameters: Vec<ApiParameter>,
    pub responses: Vec<ApiResponse>,
    pub examples: Vec<ApiExample>,
}

/// API parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub required: bool,
    pub description: String,
    pub example: Option<serde_json::Value>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Query,
    Path,
    Header,
    Body,
}

/// API example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiExample {
    pub name: String,
    pub description: String,
    pub request: serde_json::Value,
    pub response: serde_json::Value,
}

/// API response definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status_code: u16,
    pub description: String,
    pub schema: Option<serde_json::Value>,
}

/// Event handler registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandler {
    pub event_type: String,
    pub handler_id: String,
    pub priority: i32,
}

/// Plugin execution context
#[derive(Clone, Debug)]
pub struct PluginContext {
    pub plugin_id: String,
    pub config: PluginConfig,
    pub api_client: PluginApiClient,
    pub event_bus: std::sync::Arc<crate::event::EventBusManager>,
    pub database: Option<PluginDatabase>,
    pub file_system: PluginFileSystem,
}

/// API client for plugin to core communication
#[derive(Debug, Clone)]
pub struct PluginApiClient {
    plugin_id: String,
}

impl PluginApiClient {
    /// Create a new API client for a plugin
    pub fn new(plugin_id: String) -> Self {
        Self { plugin_id }
    }

    /// Get a configuration value
    pub async fn get_config(&self, _key: &str) -> Result<Option<serde_json::Value>> {
        // Implementation would call core config system
        Ok(None)
    }

    /// Set a configuration value
    pub async fn set_config(&self, _key: &str, _value: serde_json::Value) -> Result<()> {
        // Implementation would call core config system
        Ok(())
    }

    /// Get the current user
    pub async fn get_current_user(&self) -> Result<Option<crate::auth::User>> {
        // Implementation would call account manager
        Ok(None)
    }

    /// Check if current user has permission
    pub async fn check_permission(&self, _resource: &str, _action: &str) -> Result<bool> {
        // Implementation would call account manager
        Ok(false)
    }
}

/// Database access for plugins with sandboxing
#[derive(Clone, Debug)]
pub struct PluginDatabase {
    plugin_id: String,
    provider: crate::platform::database::DatabaseArc,
    permissions: DatabasePermissions,
}

/// Database permissions for plugins
#[derive(Debug, Clone)]
pub struct DatabasePermissions {
    pub can_create_tables: bool,
    pub can_drop_tables: bool,
    pub can_modify_schema: bool,
    pub max_table_count: Option<u32>,
    pub max_storage_size: Option<u64>,
}

impl PluginDatabase {
    /// Create a new database access wrapper for a plugin
    pub fn new(
        plugin_id: String,
        provider: crate::platform::database::DatabaseArc,
        permissions: DatabasePermissions,
    ) -> Self {
        Self {
            plugin_id,
            provider,
            permissions,
        }
    }

    /// Execute a database query with permission checks
    pub async fn execute(
        &self,
        query: &str,
        params: &[serde_json::Value],
    ) -> Result<crate::platform::database::QueryResult> {
        // Check permissions before executing
        if query.to_uppercase().contains("CREATE TABLE") && !self.permissions.can_create_tables {
            return Err(Error::permission(
                "database.create_table",
                "Plugin not allowed to create tables",
            ));
        }

        if query.to_uppercase().contains("DROP TABLE") && !self.permissions.can_drop_tables {
            return Err(Error::permission(
                "database.drop_table",
                "Plugin not allowed to drop tables",
            ));
        }

        // Add plugin prefix to table names to isolate data
        let prefixed_query = self.add_table_prefix(query);
        self.provider.execute(&prefixed_query, params).await
    }

    /// Query the database with permission checks
    pub async fn query(
        &self,
        query: &str,
        params: &[serde_json::Value],
    ) -> Result<Vec<crate::platform::database::Row>> {
        let prefixed_query = self.add_table_prefix(query);
        self.provider.query(&prefixed_query, params).await
    }

    fn add_table_prefix(&self, query: &str) -> String {
        query.replace("TABLE ", &format!("TABLE plugin_{}_ ", self.plugin_id))
    }
}

/// File system access for plugins with sandboxing
#[derive(Clone, Debug)]
pub struct PluginFileSystem {
    plugin_id: String,
    provider: crate::platform::filesystem::FileSystemArc,
    base_path: String,
}

impl PluginFileSystem {
    /// Create a new file system access wrapper for a plugin
    pub fn new(plugin_id: String, provider: crate::platform::filesystem::FileSystemArc) -> Self {
        Self {
            plugin_id: plugin_id.clone(),
            provider,
            base_path: format!("plugins/{}/", plugin_id),
        }
    }

    /// Read a file with sandboxing
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let safe_path = self.make_safe_path(path)?;
        self.provider.read_file(&safe_path).await
    }

    /// Write a file with sandboxing
    pub async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let safe_path = self.make_safe_path(path)?;
        self.provider.write_file(&safe_path, data).await
    }

    fn make_safe_path(&self, path: &str) -> Result<String> {
        if path.contains("..") || path.starts_with('/') {
            return Err(Error::permission("file.access", "Invalid file path"));
        }
        Ok(format!("{}{}", self.base_path, path))
    }
}

/// API request structure
#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub user: Option<crate::auth::User>,
}

/// Main plugin trait that all plugins must implement
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Get plugin information
    fn info(&self) -> PluginInfo;

    /// Get required dependencies
    fn required_dependencies(&self) -> Vec<PluginDependency>;

    /// Get required permissions
    fn required_permissions(&self) -> Vec<Permission>;

    /// Initialize the plugin
    async fn initialize(&mut self, context: PluginContext) -> Result<()>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()>;

    /// Get UI components provided by this plugin
    fn ui_components(&self) -> Vec<UIComponent>;

    /// Get menu items provided by this plugin
    fn menu_items(&self) -> Vec<MenuItem>;

    /// Get settings schema for configuration
    fn settings_schema(&self) -> Option<SettingsSchema>;

    /// Get API routes provided by this plugin
    fn api_routes(&self) -> Vec<ApiRoute>;

    /// Get event handlers provided by this plugin
    fn event_handlers(&self) -> Vec<EventHandler>;

    /// Render a UI component
    fn render_component(&self, component_id: &str, props: serde_json::Value) -> Result<VNode>;

    /// Handle an API request
    async fn handle_api_request(&self, route_id: &str, request: ApiRequest) -> Result<ApiResponse>;

    /// Handle an event
    async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()>;
}

/// Plugin validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Macro to export a plugin for dynamic loading
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        use std::ffi::c_void;
        use std::mem;

        /// Export function for dynamic loading - creates a new plugin instance
        #[no_mangle]
        pub extern "C" fn create_plugin() -> *mut dyn $crate::plugin::Plugin {
            let plugin = <$plugin_type>::default();
            let boxed: Box<dyn $crate::plugin::Plugin> = Box::new(plugin);
            Box::into_raw(boxed)
        }

        /// Export function for dynamic loading - destroys a plugin instance
        #[no_mangle]
        pub extern "C" fn destroy_plugin(plugin: *mut dyn $crate::plugin::Plugin) {
            if !plugin.is_null() {
                unsafe {
                    let _boxed = Box::from_raw(plugin);
                    // Plugin will be dropped automatically
                }
            }
        }

        /// Export function to get plugin info without creating instance
        #[no_mangle]
        pub extern "C" fn get_plugin_info() -> *mut std::ffi::c_char {
            let plugin = <$plugin_type>::default();
            let info = plugin.info();
            let json = match serde_json::to_string(&info) {
                Ok(json) => json,
                Err(_) => return std::ptr::null_mut(),
            };

            let c_string = match std::ffi::CString::new(json) {
                Ok(c_str) => c_str,
                Err(_) => return std::ptr::null_mut(),
            };

            c_string.into_raw()
        }

        /// Free the string returned by get_plugin_info
        #[no_mangle]
        pub extern "C" fn free_plugin_info_string(s: *mut std::ffi::c_char) {
            if !s.is_null() {
                unsafe {
                    let _ = std::ffi::CString::from_raw(s);
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info_creation() {
        let info = PluginInfo {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            minimum_core_version: "0.1.0".to_string(),
            supported_platforms: vec![Platform::All],
        };

        assert_eq!(info.id, "test_plugin");
        assert_eq!(info.name, "Test Plugin");
    }

    #[test]
    fn test_platform_serialization() {
        let platform = Platform::Web;
        let serialized = serde_json::to_string(&platform).unwrap();
        let deserialized: Platform = serde_json::from_str(&serialized).unwrap();
        assert_eq!(platform, deserialized);
    }
}