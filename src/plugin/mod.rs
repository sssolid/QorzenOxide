// src/plugin/mod.rs - Plugin system with hot-reloading and sandboxing

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use dioxus::prelude::*;

use crate::auth::{Permission, User};
use crate::error::{Error, Result};
use crate::event::{Event, EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};
use crate::platform::{DatabaseProvider, FileSystemProvider};
use crate::config::SettingsSchema;
use crate::platform::database::DatabaseArc;
use crate::platform::filesystem::FileSystemArc;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    IOS,
    Android,
    Web,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_requirement: String, // SemVer
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_id: String,
    pub version: String,
    pub config_schema: serde_json::Value, // JSON Schema
    pub default_values: serde_json::Value,
    pub user_overrides: serde_json::Value,
    pub validation_rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Range { min: f64, max: f64 },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponent {
    pub id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub props: serde_json::Value,
    pub required_permissions: Vec<Permission>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRoute {
    pub path: String,
    pub method: HttpMethod,
    pub handler_id: String,
    pub required_permissions: Vec<Permission>,
    pub rate_limit: Option<RateLimit>,
    pub documentation: ApiDocumentation,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocumentation {
    pub summary: String,
    pub description: String,
    pub parameters: Vec<ApiParameter>,
    pub responses: Vec<ApiResponse>,
    pub examples: Vec<ApiExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub required: bool,
    pub description: String,
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Query,
    Path,
    Header,
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiExample {
    pub name: String,
    pub description: String,
    pub request: serde_json::Value,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status_code: u16,
    pub description: String,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandler {
    pub event_type: String,
    pub handler_id: String,
    pub priority: i32,
}

#[derive(Clone)]
pub struct PluginContext {
    pub plugin_id: String,
    pub config: PluginConfig,
    pub api_client: PluginApiClient,
    pub event_bus: Arc<EventBusManager>,
    pub database: Option<PluginDatabase>,
    pub file_system: PluginFileSystem,
}

#[derive(Debug, Clone)]
pub struct PluginApiClient {
    plugin_id: String,
    // Internal API endpoints
}

impl PluginApiClient {
    pub fn new(plugin_id: String) -> Self {
        Self { plugin_id }
    }

    pub async fn get_config(&self, _key: &str) -> Result<Option<serde_json::Value>> {
        // Implementation would call core config system
        Ok(None)
    }

    pub async fn set_config(&self, _key: &str, _value: serde_json::Value) -> Result<()> {
        // Implementation would call core config system
        Ok(())
    }

    pub async fn get_current_user(&self) -> Result<Option<User>> {
        // Implementation would call account manager
        Ok(None)
    }

    pub async fn check_permission(&self, _resource: &str, _action: &str) -> Result<bool> {
        // Implementation would call account manager
        Ok(false)
    }
}

#[derive(Clone)]
pub struct PluginDatabase {
    plugin_id: String,
    provider: DatabaseArc,
    permissions: DatabasePermissions,
}

#[derive(Debug, Clone)]
pub struct DatabasePermissions {
    pub can_create_tables: bool,
    pub can_drop_tables: bool,
    pub can_modify_schema: bool,
    pub max_table_count: Option<u32>,
    pub max_storage_size: Option<u64>,
}

impl PluginDatabase {
    pub fn new(
        plugin_id: String,
        provider: DatabaseArc,
        permissions: DatabasePermissions,
    ) -> Self {
        Self {
            plugin_id,
            provider,
            permissions,
        }
    }

    pub async fn execute(
        &self,
        query: &str,
        params: &[serde_json::Value],
    ) -> Result<crate::platform::QueryResult> {
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

    fn add_table_prefix(&self, query: &str) -> String {
        // Simple implementation - in practice would need proper SQL parsing
        query.replace("TABLE ", &format!("TABLE plugin_{}_ ", self.plugin_id))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct PluginFileSystem {
    plugin_id: String,
    provider: FileSystemArc,
    base_path: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct PluginFileSystem {
    plugin_id: String,
    provider: FileSystemArc,
    base_path: String,
}

impl PluginFileSystem {
    pub fn new(plugin_id: String, provider: FileSystemArc) -> Self {
        Self {
            plugin_id: plugin_id.clone(),
            provider,
            base_path: format!("plugins/{}/", plugin_id),
        }
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let safe_path = self.make_safe_path(path)?;
        self.provider.read_file(&safe_path).await
    }

    pub async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let safe_path = self.make_safe_path(path)?;
        self.provider.write_file(&safe_path, data).await
    }

    fn make_safe_path(&self, path: &str) -> Result<String> {
        // Prevent directory traversal
        if path.contains("..") || path.starts_with('/') {
            return Err(Error::permission("file.access", "Invalid file path"));
        }

        Ok(format!("{}{}", self.base_path, path))
    }
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_time_ms: u64,
    pub max_file_size_mb: u64,
    pub max_network_requests_per_minute: u32,
    pub max_database_queries_per_minute: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 100,
            max_cpu_time_ms: 5000,
            max_file_size_mb: 10,
            max_network_requests_per_minute: 60,
            max_database_queries_per_minute: 100,
        }
    }
}

pub struct PluginSandbox {
    resource_limits: ResourceLimits,
    allowed_permissions: Vec<Permission>,
}

impl PluginSandbox {
    pub fn new(resource_limits: ResourceLimits, allowed_permissions: Vec<Permission>) -> Self {
        Self {
            resource_limits,
            allowed_permissions,
        }
    }

    pub fn check_operation(&self, operation: &str, resource: &str) -> bool {
        self.allowed_permissions.iter().any(|p| {
            (p.resource == resource || p.resource == "*")
                && (p.action == operation || p.action == "*")
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    fn info(&self) -> PluginInfo;
    fn required_dependencies(&self) -> Vec<PluginDependency>;
    fn required_permissions(&self) -> Vec<Permission>;
    async fn initialize(&mut self, context: PluginContext) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    fn ui_components(&self) -> Vec<UIComponent>;
    fn menu_items(&self) -> Vec<MenuItem>;
    fn settings_schema(&self) -> Option<SettingsSchema>;
    fn api_routes(&self) -> Vec<ApiRoute>;
    fn event_handlers(&self) -> Vec<EventHandler>;
    fn render_component(&self, component_id: &str, props: serde_json::Value) -> Result<VNode>;
    async fn handle_api_request(&self, route_id: &str, request: ApiRequest) -> Result<ApiResponse>;
    async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait Plugin: Sync + std::fmt::Debug {
    fn info(&self) -> PluginInfo;
    fn required_dependencies(&self) -> Vec<PluginDependency>;
    fn required_permissions(&self) -> Vec<Permission>;
    async fn initialize(&mut self, context: PluginContext) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    fn ui_components(&self) -> Vec<UIComponent>;
    fn menu_items(&self) -> Vec<MenuItem>;
    fn settings_schema(&self) -> Option<SettingsSchema>;
    fn api_routes(&self) -> Vec<ApiRoute>;
    fn event_handlers(&self) -> Vec<EventHandler>;
    fn render_component(&self, component_id: &str, props: serde_json::Value) -> Result<VNode>;
    async fn handle_api_request(&self, route_id: &str, request: ApiRequest) -> Result<ApiResponse>;
    async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()>;
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait PluginLoader: Send + Sync {
    async fn load_plugin(&self, path: &str) -> Result<Box<dyn Plugin>>;
    async fn validate_plugin(&self, plugin: &dyn Plugin) -> Result<ValidationResult>;
    async fn unload_plugin(&self, plugin_id: &str) -> Result<()>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait PluginLoader: Sync {
    async fn load_plugin(&self, path: &str) -> Result<Box<dyn Plugin>>;
    async fn validate_plugin(&self, plugin: &dyn Plugin) -> Result<ValidationResult>;
    async fn unload_plugin(&self, plugin_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub user: Option<User>,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ApiResponse {
//     pub status_code: u16,
//     pub headers: HashMap<String, String>,
//     pub body: Option<serde_json::Value>,
// }

#[derive(Debug)]
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
    dependencies: HashMap<String, Vec<String>>,
    load_order: Vec<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            dependencies: HashMap::new(),
            load_order: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let info = plugin.info();
        let deps = plugin
            .required_dependencies()
            .into_iter()
            .map(|d| d.plugin_id)
            .collect();

        if self.plugins.contains_key(&info.id) {
            return Err(Error::plugin(&info.id, "Plugin already registered"));
        }

        self.plugins.insert(info.id.clone(), plugin);
        self.dependencies.insert(info.id.clone(), deps);

        self.calculate_load_order()?;

        Ok(())
    }

    pub fn get(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        self.plugins.get(plugin_id).map(|p| p.as_ref())
    }

    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    fn calculate_load_order(&mut self) -> Result<()> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for plugin_id in self.plugins.keys() {
            if !visited.contains(plugin_id) {
                self.visit_plugin(plugin_id, &mut order, &mut visited, &mut visiting)?;
            }
        }

        self.load_order = order;
        Ok(())
    }

    fn visit_plugin(
        &self,
        plugin_id: &str,
        order: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visiting.contains(plugin_id) {
            return Err(Error::plugin(plugin_id, "Circular dependency detected"));
        }

        if visited.contains(plugin_id) {
            return Ok(());
        }

        visiting.insert(plugin_id.to_string());

        if let Some(deps) = self.dependencies.get(plugin_id) {
            for dep in deps {
                if !self.plugins.contains_key(dep) {
                    return Err(Error::plugin(
                        plugin_id,
                        format!("Missing dependency: {}", dep),
                    ));
                }
                self.visit_plugin(dep, order, visited, visiting)?;
            }
        }

        visiting.remove(plugin_id);
        visited.insert(plugin_id.to_string());
        order.push(plugin_id.to_string());

        Ok(())
    }

    pub fn load_order(&self) -> &[String] {
        &self.load_order
    }
}

pub struct DependencyResolver {
    // Implementation for resolving plugin dependencies
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {}
    }

    pub fn resolve(&self, plugin: &dyn Plugin, registry: &PluginRegistry) -> Result<Vec<String>> {
        let deps = plugin.required_dependencies();
        let mut resolved = Vec::new();

        for dep in deps {
            if registry.get(&dep.plugin_id).is_some() {
                resolved.push(dep.plugin_id);
            } else if !dep.optional {
                return Err(Error::plugin(
                    &plugin.info().id,
                    format!("Required dependency not found: {}", dep.plugin_id),
                ));
            }
        }

        Ok(resolved)
    }

    pub fn check_version_compatibility(&self, required: &str, available: &str) -> bool {
        // Simple version check - in practice would use semver
        required == available || required == "*"
    }
}

pub struct PluginManager {
    state: ManagedState,
    registry: PluginRegistry,
    loader: Box<dyn PluginLoader + Send + Sync>,
    sandbox: PluginSandbox,
    api_provider: PluginApiProvider,
    dependency_resolver: DependencyResolver,
    plugin_contexts: HashMap<String, PluginContext>,
}

impl std::fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginManager")
            .field("registry", &self.registry)
            .finish()
    }
}

pub struct PluginApiProvider {
    // Provides API access to plugins
}

impl PluginApiProvider {
    pub fn new() -> Self {
        Self {}
    }

    pub fn create_client(&self, plugin_id: String) -> PluginApiClient {
        PluginApiClient::new(plugin_id)
    }
}

impl PluginManager {
    pub fn new(loader: Box<dyn PluginLoader + Send + Sync>) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "plugin_manager"),
            registry: PluginRegistry::new(),
            loader,
            sandbox: PluginSandbox::new(ResourceLimits::default(), Vec::new()),
            api_provider: PluginApiProvider::new(),
            dependency_resolver: DependencyResolver::new(),
            plugin_contexts: HashMap::new(),
        }
    }

    pub async fn load_plugin(&mut self, path: &str) -> Result<()> {
        let plugin = self.loader.load_plugin(path).await?;

        // Validate plugin
        let validation = self.loader.validate_plugin(plugin.as_ref()).await?;
        if !validation.is_valid {
            return Err(Error::plugin(
                &plugin.info().id,
                format!("Plugin validation failed: {:?}", validation.errors),
            ));
        }

        // Check dependencies
        let _resolved_deps = self
            .dependency_resolver
            .resolve(plugin.as_ref(), &self.registry)?;

        // Register plugin
        let plugin_id = plugin.info().id.clone();
        self.registry.register(plugin)?;

        // Create plugin context
        let context = self.create_plugin_context(&plugin_id).await?;
        self.plugin_contexts.insert(plugin_id, context);

        Ok(())
    }

    pub async fn unload_plugin(&mut self, plugin_id: &str) -> Result<()> {
        if let Some(plugin) = self.registry.plugins.get_mut(plugin_id) {
            plugin.shutdown().await?;
        }

        self.registry.plugins.remove(plugin_id);
        self.plugin_contexts.remove(plugin_id);
        self.loader.unload_plugin(plugin_id).await?;

        Ok(())
    }

    pub async fn initialize_plugins(&mut self) -> Result<()> {
        let load_order = self.registry.load_order().to_vec();

        for plugin_id in load_order {
            if let (Some(plugin), Some(context)) = (
                self.registry.plugins.get_mut(&plugin_id),
                self.plugin_contexts.get(&plugin_id).cloned(),
            ) {
                plugin.initialize(context).await.map_err(|e| {
                    Error::plugin(&plugin_id, format!("Plugin initialization failed: {}", e))
                })?;
            }
        }

        Ok(())
    }

    pub fn get_ui_components(&self) -> Vec<(String, UIComponent)> {
        let mut components = Vec::new();

        for (plugin_id, plugin) in &self.registry.plugins {
            for component in plugin.ui_components() {
                components.push((plugin_id.clone(), component));
            }
        }

        components
    }

    pub fn get_menu_items(&self) -> Vec<(String, MenuItem)> {
        let mut items = Vec::new();

        for (plugin_id, plugin) in &self.registry.plugins {
            for item in plugin.menu_items() {
                items.push((plugin_id.clone(), item));
            }
        }

        items
    }

    pub fn render_component(
        &self,
        plugin_id: &str,
        component_id: &str,
        props: serde_json::Value,
    ) -> Result<VNode> {
        let plugin = self
            .registry
            .get(plugin_id)
            .ok_or_else(|| Error::plugin(plugin_id, "Plugin not found"))?;

        plugin.render_component(component_id, props)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn create_plugin_context(&self, plugin_id: &str) -> Result<PluginContext> {
        let filesystem_provider: Arc<dyn FileSystemProvider + Send + Sync> =
            Arc::new(crate::platform::native::NativeFileSystem::new()?);

        Ok(PluginContext {
            plugin_id: plugin_id.to_string(),
            config: PluginConfig {
                plugin_id: plugin_id.to_string(),
                version: "1.0.0".to_string(),
                config_schema: serde_json::json!({}),
                default_values: serde_json::json!({}),
                user_overrides: serde_json::json!({}),
                validation_rules: Vec::new(),
            },
            api_client: self.api_provider.create_client(plugin_id.to_string()),
            event_bus: Arc::new(EventBusManager::new(
                crate::event::EventBusConfig::default(),
            )),
            database: None,
            file_system: PluginFileSystem::new(
                plugin_id.to_string(),
                filesystem_provider,
            ),
        })
    }

    #[cfg(target_arch = "wasm32")]
    async fn create_plugin_context(&self, plugin_id: &str) -> Result<PluginContext> {
        let filesystem_provider: FileSystemArc =
            Arc::new(crate::platform::web::WebFileSystem::new()?);

        Ok(PluginContext {
            plugin_id: plugin_id.to_string(),
            config: PluginConfig {
                plugin_id: plugin_id.to_string(),
                version: "1.0.0".to_string(),
                config_schema: serde_json::json!({}),
                default_values: serde_json::json!({}),
                user_overrides: serde_json::json!({}),
                validation_rules: Vec::new(),
            },
            api_client: self.api_provider.create_client(plugin_id.to_string()),
            event_bus: Arc::new(EventBusManager::new(
                crate::event::EventBusConfig::default(),
            )),
            database: None,
            file_system: PluginFileSystem::new(
                plugin_id.to_string(),
                filesystem_provider,
            ),
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait(?Send)]
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

        // Load plugins from plugin directory
        // Initialize all loaded plugins
        self.initialize_plugins().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Shutdown all plugins in reverse order
        let mut load_order = self.registry.load_order().to_vec();
        load_order.reverse();

        for plugin_id in load_order {
            if let Err(e) = self.unload_plugin(&plugin_id).await {
                web_sys::console::error_1(&format!("Failed to unload plugin {}: {}", plugin_id, e).into());
            }
        }

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        status.add_metadata(
            "loaded_plugins",
            serde_json::Value::from(self.registry.plugins.len()),
        );
        status.add_metadata(
            "plugin_list",
            serde_json::Value::Array(
                self.registry
                    .list()
                    .into_iter()
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect(),
            ),
        );

        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec!["plugin.load".to_string(), "plugin.manage".to_string()],
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
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

        // Load plugins from plugin directory
        // Initialize all loaded plugins
        self.initialize_plugins().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Shutdown all plugins in reverse order
        let mut load_order = self.registry.load_order().to_vec();
        load_order.reverse();

        for plugin_id in load_order {
            if let Err(e) = self.unload_plugin(&plugin_id).await {
                tracing::error!("Failed to unload plugin {}: {}", plugin_id, e);
            }
        }

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        status.add_metadata(
            "loaded_plugins",
            serde_json::Value::from(self.registry.plugins.len()),
        );
        status.add_metadata(
            "plugin_list",
            serde_json::Value::Array(
                self.registry
                    .list()
                    .into_iter()
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect(),
            ),
        );

        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec!["plugin.load".to_string(), "plugin.manage".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestPlugin {
        info: PluginInfo,
    }

    impl TestPlugin {
        fn new(id: String) -> Self {
            Self {
                info: PluginInfo {
                    id,
                    name: "Test Plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "A test plugin".to_string(),
                    author: "Test Author".to_string(),
                    license: "MIT".to_string(),
                    homepage: None,
                    repository: None,
                    minimum_core_version: "1.0.0".to_string(),
                    supported_platforms: vec![Platform::All],
                },
            }
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn info(&self) -> PluginInfo {
            self.info.clone()
        }

        fn required_dependencies(&self) -> Vec<PluginDependency> {
            Vec::new()
        }

        fn required_permissions(&self) -> Vec<Permission> {
            Vec::new()
        }

        async fn initialize(&mut self, _context: PluginContext) -> Result<()> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            Ok(())
        }

        fn ui_components(&self) -> Vec<UIComponent> {
            Vec::new()
        }

        fn menu_items(&self) -> Vec<MenuItem> {
            Vec::new()
        }

        fn settings_schema(&self) -> Option<SettingsSchema> {
            None
        }

        fn api_routes(&self) -> Vec<ApiRoute> {
            Vec::new()
        }

        fn event_handlers(&self) -> Vec<EventHandler> {
            Vec::new()
        }

        fn render_component(
            &self,
            _component_id: &str,
            _props: serde_json::Value,
        ) -> Result<VNode> {
            Err(Error::plugin(
                &self.info.id,
                "Component rendering not implemented",
            ))
        }

        async fn handle_api_request(
            &self,
            _route_id: &str,
            _request: ApiRequest,
        ) -> Result<ApiResponse> {
            Err(Error::plugin(&self.info.id, "API handling not implemented"))
        }

        async fn handle_event(&self, _handler_id: &str, _event: &dyn Event) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new("test_plugin".to_string()));

        registry.register(plugin).unwrap();

        assert!(registry.get("test_plugin").is_some());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_dependency_resolution() {
        let resolver = DependencyResolver::new();
        let registry = PluginRegistry::new();
        let plugin = TestPlugin::new("test_plugin".to_string());

        let resolved = resolver.resolve(&plugin, &registry).unwrap();
        assert!(resolved.is_empty()); // No dependencies
    }
}