// src/plugin/registry.rs - Enhanced Plugin Registry

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

use crate::plugin::{Plugin, PluginInfo, Platform, PluginContext};
use crate::error::{Error, Result};

static PLUGIN_FACTORIES: OnceLock<RwLock<HashMap<String, Box<dyn PluginFactory>>>> = OnceLock::new();

/// Plugin factory trait for creating plugin instances
pub trait PluginFactory: Send + Sync {
    fn create(&self) -> Box<dyn Plugin>;
    fn info(&self) -> PluginInfo;
    fn id(&self) -> String {
        self.info().id.clone()
    }
}

/// Simple plugin factory implementation
pub struct SimplePluginFactory<T>
where
    T: Plugin + Default + 'static,
{
    info: PluginInfo,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> SimplePluginFactory<T>
where
    T: Plugin + Default + 'static,
{
    pub fn new(info: PluginInfo) -> Self {
        Self {
            info,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> PluginFactory for SimplePluginFactory<T>
where
    T: Plugin + Default + 'static,
{
    fn create(&self) -> Box<dyn Plugin> {
        Box::new(T::default())
    }

    fn info(&self) -> PluginInfo {
        self.info.clone()
    }
}

/// Global plugin factory registry
pub struct PluginFactoryRegistry;

impl PluginFactoryRegistry {
    /// Initialize the registry
    pub fn initialize() {
        PLUGIN_FACTORIES.get_or_init(|| RwLock::new(HashMap::new()));
    }

    /// Register a plugin factory
    pub async fn register<F>(factory: F) -> Result<()>
    where
        F: PluginFactory + 'static,
    {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap();
        let mut factories = registry.write().await;
        let plugin_id = factory.id().to_string();

        // Check if already registered and return Ok instead of error
        if factories.contains_key(&plugin_id) {
            tracing::debug!("Plugin factory '{}' already registered, skipping", plugin_id);
            return Ok(());
        }

        factories.insert(plugin_id.clone(), Box::new(factory));
        tracing::info!("Registered plugin factory: {}", plugin_id);
        Ok(())
    }

    /// List all registered plugins
    pub async fn list_plugins() -> Vec<String> {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap(); // SAFETY: We just initialized it
        let factories = registry.read().await;
        factories.keys().cloned().collect()
    }

    /// Get plugin info by ID
    pub async fn get_plugin_info(plugin_id: &str) -> Option<PluginInfo> {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap(); // SAFETY: We just initialized it
        let factories = registry.read().await;
        factories.get(plugin_id).map(|f| f.info())
    }

    /// Create a plugin instance by ID
    pub async fn create_plugin(plugin_id: &str) -> Option<Box<dyn Plugin>> {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap(); // SAFETY: We just initialized it
        let factories = registry.read().await;
        factories.get(plugin_id).map(|f| f.create())
    }

    /// Get all plugin info
    pub async fn get_all_plugin_info() -> Vec<PluginInfo> {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap(); // SAFETY: We just initialized it
        let factories = registry.read().await;
        factories.values().map(|f| f.info()).collect()
    }

    /// Check if a plugin is registered
    pub async fn is_registered(plugin_id: &str) -> bool {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap(); // SAFETY: We just initialized it
        let factories = registry.read().await;
        factories.contains_key(plugin_id)
    }

    /// Unregister a plugin factory
    pub async fn unregister(plugin_id: &str) -> Result<()> {
        Self::initialize();
        let registry = PLUGIN_FACTORIES.get().unwrap(); // SAFETY: We just initialized it
        let mut factories = registry.write().await;

        if factories.remove(plugin_id).is_some() {
            tracing::info!("Unregistered plugin factory: {}", plugin_id);
            Ok(())
        } else {
            Err(Error::plugin(plugin_id, "Plugin factory not found"))
        }
    }
}

/// Macro for registering plugins at compile time
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty, $info:expr) => {
        #[cfg(any(test, feature = "plugin_registration"))]
        #[ctor::ctor]
        fn register_plugin() {
            use $crate::plugin::registry::*;

            tokio::runtime::Handle::try_current()
                .unwrap_or_else(|| {
                    tokio::runtime::Runtime::new().unwrap().handle().clone()
                })
                .spawn(async move {
                    let factory = SimplePluginFactory::<$plugin_type>::new($info);
                    if let Err(e) = PluginFactoryRegistry::register(factory).await {
                        tracing::error!("Failed to register plugin: {}", e);
                    }
                });
        }
    };
}

/// Built-in plugins module
pub mod builtin {
    use super::*;
    use crate::plugin::*;
    use crate::auth::{Permission, PermissionScope};
    use crate::config::SettingsSchema;
    use crate::error::Result;
    use crate::event::Event;
    use dioxus::prelude::*;
    use serde_json::Value;

    /// System Monitor Plugin
    #[derive(Debug, Default)]
    pub struct SystemMonitorPlugin {
        context: Option<PluginContext>,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl Plugin for SystemMonitorPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "system_monitor".to_string(),
                name: "System Monitor".to_string(),
                version: "1.0.0".to_string(),
                description: "Monitor system performance and health".to_string(),
                author: "Qorzen Team".to_string(),
                license: "MIT".to_string(),
                homepage: Some("https://qorzen.com/plugins/system-monitor".to_string()),
                repository: Some("https://github.com/sssolid/system-monitor-plugin".to_string()),
                minimum_core_version: "0.1.0".to_string(),
                supported_platforms: vec![Platform::All],
            }
        }

        fn required_dependencies(&self) -> Vec<PluginDependency> {
            vec![]
        }

        fn required_permissions(&self) -> Vec<Permission> {
            vec![
                Permission {
                    resource: "system".to_string(),
                    action: "monitor".to_string(),
                    scope: PermissionScope::Global,
                },
                Permission {
                    resource: "ui".to_string(),
                    action: "render".to_string(),
                    scope: PermissionScope::Global,
                },
            ]
        }

        async fn initialize(&mut self, context: PluginContext) -> Result<()> {
            tracing::info!("Initializing System Monitor plugin");
            self.context = Some(context);
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            tracing::info!("Shutting down System Monitor plugin");
            Ok(())
        }

        fn ui_components(&self) -> Vec<UIComponent> {
            vec![UIComponent {
                id: "system_metrics".to_string(),
                name: "System Metrics".to_string(),
                component_type: ComponentType::Widget,
                props: serde_json::json!({
                    "refresh_interval": 5000,
                    "show_cpu": true,
                    "show_memory": true,
                    "show_disk": true
                }),
                required_permissions: vec![Permission {
                    resource: "system".to_string(),
                    action: "monitor".to_string(),
                    scope: PermissionScope::Global,
                }],
            }]
        }

        fn menu_items(&self) -> Vec<MenuItem> {
            vec![MenuItem {
                id: "system_monitor".to_string(),
                label: "System Monitor".to_string(),
                icon: Some("🖥️".to_string()),
                route: Some("/plugins/system_monitor".to_string()),
                action: None,
                required_permissions: vec![Permission {
                    resource: "system".to_string(),
                    action: "monitor".to_string(),
                    scope: PermissionScope::Global,
                }],
                order: 300,
                children: vec![],
            }]
        }

        fn settings_schema(&self) -> Option<SettingsSchema> {
            Some(SettingsSchema {
                version: "1.0".to_string(),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "refresh_interval": {
                            "type": "integer",
                            "title": "Refresh Interval (ms)",
                            "description": "How often to update system metrics",
                            "default": 5000,
                            "minimum": 1000,
                            "maximum": 60000
                        },
                        "show_cpu": {
                            "type": "boolean",
                            "title": "Show CPU Usage",
                            "default": true
                        },
                        "show_memory": {
                            "type": "boolean",
                            "title": "Show Memory Usage",
                            "default": true
                        },
                        "alert_threshold": {
                            "type": "number",
                            "title": "Alert Threshold (%)",
                            "description": "Send alerts when usage exceeds this percentage",
                            "default": 80.0,
                            "minimum": 50.0,
                            "maximum": 95.0
                        }
                    }
                }),
                defaults: serde_json::json!({
                    "refresh_interval": 5000,
                    "show_cpu": true,
                    "show_memory": true,
                    "alert_threshold": 80.0
                }),
            })
        }

        fn api_routes(&self) -> Vec<ApiRoute> {
            vec![ApiRoute {
                path: "/api/plugins/system_monitor/metrics".to_string(),
                method: HttpMethod::GET,
                handler_id: "get_system_metrics".to_string(),
                required_permissions: vec![Permission {
                    resource: "system".to_string(),
                    action: "monitor".to_string(),
                    scope: PermissionScope::Global,
                }],
                rate_limit: None,
                documentation: ApiDocumentation {
                    summary: "Get system metrics".to_string(),
                    description: "Retrieve current system performance metrics".to_string(),
                    parameters: vec![],
                    responses: vec![ApiResponse {
                        status_code: 200,
                        description: "System metrics".to_string(),
                        schema: Some(serde_json::json!({
                            "type": "object",
                            "properties": {
                                "cpu_usage": {"type": "number"},
                                "memory_usage": {"type": "number"},
                                "disk_usage": {"type": "number"},
                                "timestamp": {"type": "string"}
                            }
                        })),
                    }],
                    examples: vec![],
                },
            }]
        }

        fn event_handlers(&self) -> Vec<crate::plugin::EventHandler> {
            vec![crate::plugin::EventHandler {
                event_type: "system.alert".to_string(),
                handler_id: "handle_system_alert".to_string(),
                priority: 100,
            }]
        }

        fn render_component(&self, component_id: &str, props: Value) -> Result<VNode> {
            match component_id {
                "system_metrics" => {
                    let refresh_interval = props
                        .get("refresh_interval")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5000);

                    Ok(VNode::from(rsx! {
                        div { class: "system-metrics-widget",
                            h3 { class: "text-lg font-semibold mb-4", "System Metrics" }
                            div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                                div { class: "bg-blue-50 p-4 rounded-lg",
                                    div { class: "flex items-center justify-between",
                                        div {
                                            h4 { class: "text-sm font-medium text-blue-900", "CPU Usage" }
                                            p { class: "text-2xl font-bold text-blue-700", "23%" }
                                        }
                                        div { class: "text-blue-500 text-xl", "🖥️" }
                                    }
                                }
                                div { class: "bg-green-50 p-4 rounded-lg",
                                    div { class: "flex items-center justify-between",
                                        div {
                                            h4 { class: "text-sm font-medium text-green-900", "Memory" }
                                            p { class: "text-2xl font-bold text-green-700", "67%" }
                                        }
                                        div { class: "text-green-500 text-xl", "💾" }
                                    }
                                }
                                div { class: "bg-purple-50 p-4 rounded-lg",
                                    div { class: "flex items-center justify-between",
                                        div {
                                            h4 { class: "text-sm font-medium text-purple-900", "Disk" }
                                            p { class: "text-2xl font-bold text-purple-700", "45%" }
                                        }
                                        div { class: "text-purple-500 text-xl", "💿" }
                                    }
                                }
                            }
                            p { class: "text-xs text-gray-500 mt-2", "Refreshes every {refresh_interval}ms" }
                        }
                    }))
                }
                _ => Err(Error::plugin("system_monitor", "Unknown component")),
            }
        }

        async fn handle_api_request(&self, route_id: &str, _request: ApiRequest) -> Result<ApiResponse> {
            match route_id {
                "get_system_metrics" => {
                    let metrics = serde_json::json!({
                        "cpu_usage": 23.5,
                        "memory_usage": 67.2,
                        "disk_usage": 45.8,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });

                    Ok(ApiResponse {
                        status_code: 200,
                        description: "Success".to_string(),
                        schema: Some(metrics),
                    })
                }
                _ => Err(Error::plugin("system_monitor", "Unknown API route")),
            }
        }

        async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()> {
            match handler_id {
                "handle_system_alert" => {
                    tracing::warn!("System alert received: {}", event.event_type());
                    Ok(())
                }
                _ => Err(Error::plugin("system_monitor", "Unknown event handler")),
            }
        }
    }

    /// Notification Plugin
    #[derive(Debug, Default)]
    pub struct NotificationPlugin {
        context: Option<PluginContext>,
    }

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl Plugin for NotificationPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "notifications".to_string(),
                name: "Smart Notifications".to_string(),
                version: "1.0.0".to_string(),
                description: "Advanced notification system with multiple delivery channels".to_string(),
                author: "Qorzen Team".to_string(),
                license: "MIT".to_string(),
                homepage: Some("https://qorzen.com/plugins/notifications".to_string()),
                repository: Some("https://github.com/sssolid/notification-plugin".to_string()),
                minimum_core_version: "0.1.0".to_string(),
                supported_platforms: vec![Platform::All],
            }
        }

        fn required_dependencies(&self) -> Vec<PluginDependency> {
            vec![]
        }

        fn required_permissions(&self) -> Vec<Permission> {
            vec![
                Permission {
                    resource: "notifications".to_string(),
                    action: "send".to_string(),
                    scope: PermissionScope::Global,
                },
                Permission {
                    resource: "ui".to_string(),
                    action: "render".to_string(),
                    scope: PermissionScope::Global,
                },
            ]
        }

        async fn initialize(&mut self, context: PluginContext) -> Result<()> {
            tracing::info!("Initializing Notification plugin");
            self.context = Some(context);
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            tracing::info!("Shutting down Notification plugin");
            Ok(())
        }

        fn ui_components(&self) -> Vec<UIComponent> {
            vec![UIComponent {
                id: "notification_center".to_string(),
                name: "Notification Center".to_string(),
                component_type: ComponentType::Widget,
                props: serde_json::json!({
                    "max_visible": 5,
                    "auto_dismiss": true,
                    "dismiss_timeout": 5000
                }),
                required_permissions: vec![Permission {
                    resource: "notifications".to_string(),
                    action: "view".to_string(),
                    scope: PermissionScope::Global,
                }],
            }]
        }

        fn menu_items(&self) -> Vec<MenuItem> {
            vec![MenuItem {
                id: "notifications".to_string(),
                label: "Notifications".to_string(),
                icon: Some("🔔".to_string()),
                route: Some("/plugins/notifications".to_string()),
                action: None,
                required_permissions: vec![Permission {
                    resource: "notifications".to_string(),
                    action: "manage".to_string(),
                    scope: PermissionScope::Global,
                }],
                order: 200,
                children: vec![],
            }]
        }

        fn settings_schema(&self) -> Option<SettingsSchema> {
            Some(SettingsSchema {
                version: "1.0".to_string(),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "email_enabled": {
                            "type": "boolean",
                            "title": "Enable Email Notifications",
                            "default": true
                        },
                        "push_enabled": {
                            "type": "boolean",
                            "title": "Enable Push Notifications",
                            "default": false
                        },
                        "default_priority": {
                            "type": "string",
                            "title": "Default Priority",
                            "enum": ["low", "normal", "high", "urgent"],
                            "default": "normal"
                        }
                    }
                }),
                defaults: serde_json::json!({
                    "email_enabled": true,
                    "push_enabled": false,
                    "default_priority": "normal"
                }),
            })
        }

        fn api_routes(&self) -> Vec<ApiRoute> {
            vec![ApiRoute {
                path: "/api/plugins/notifications/send".to_string(),
                method: HttpMethod::POST,
                handler_id: "send_notification".to_string(),
                required_permissions: vec![Permission {
                    resource: "notifications".to_string(),
                    action: "send".to_string(),
                    scope: PermissionScope::Global,
                }],
                rate_limit: Some(RateLimit {
                    requests_per_minute: 60,
                    burst_limit: 10,
                }),
                documentation: ApiDocumentation {
                    summary: "Send notification".to_string(),
                    description: "Send a notification through configured channels".to_string(),
                    parameters: vec![ApiParameter {
                        name: "message".to_string(),
                        parameter_type: ParameterType::Body,
                        required: true,
                        description: "Notification message content".to_string(),
                        example: Some(serde_json::json!({
                            "title": "Alert",
                            "message": "System requires attention",
                            "priority": "high"
                        })),
                    }],
                    responses: vec![ApiResponse {
                        status_code: 200,
                        description: "Notification sent successfully".to_string(),
                        schema: Some(serde_json::json!({
                            "type": "object",
                            "properties": {
                                "id": {"type": "string"},
                                "status": {"type": "string"}
                            }
                        })),
                    }],
                    examples: vec![],
                },
            }]
        }

        fn event_handlers(&self) -> Vec<crate::plugin::EventHandler> {
            vec![crate::plugin::EventHandler {
                event_type: "*".to_string(),
                handler_id: "process_event_notification".to_string(),
                priority: 50,
            }]
        }

        fn render_component(&self, component_id: &str, props: Value) -> Result<VNode> {
            match component_id {
                "notification_center" => {
                    let max_visible = props
                        .get("max_visible")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5);

                    Ok(VNode::from(rsx! {
                        div { class: "notification-center",
                            h3 { class: "text-lg font-semibold mb-4", "Notifications" }
                            div { class: "space-y-2",
                                div { class: "bg-blue-50 border-l-4 border-blue-400 p-3 rounded-r",
                                    div { class: "flex justify-between items-start",
                                        div {
                                            h4 { class: "text-sm font-medium text-blue-800", "System Update" }
                                            p { class: "text-sm text-blue-700", "New features available" }
                                        }
                                        button { class: "text-blue-500 hover:text-blue-700", "×" }
                                    }
                                }
                                div { class: "bg-yellow-50 border-l-4 border-yellow-400 p-3 rounded-r",
                                    div { class: "flex justify-between items-start",
                                        div {
                                            h4 { class: "text-sm font-medium text-yellow-800", "Warning" }
                                            p { class: "text-sm text-yellow-700", "High CPU usage detected" }
                                        }
                                        button { class: "text-yellow-500 hover:text-yellow-700", "×" }
                                    }
                                }
                            }
                            p { class: "text-xs text-gray-500 mt-2", "Showing up to {max_visible} notifications" }
                        }
                    }))
                }
                _ => Err(Error::plugin("notifications", "Unknown component")),
            }
        }

        async fn handle_api_request(&self, route_id: &str, request: ApiRequest) -> Result<ApiResponse> {
            match route_id {
                "send_notification" => {
                    let notification_id = uuid::Uuid::new_v4().to_string();
                    tracing::info!("Sending notification: {:?}", request.body);

                    Ok(ApiResponse {
                        status_code: 200,
                        description: "Notification sent".to_string(),
                        schema: Some(serde_json::json!({
                            "id": notification_id,
                            "status": "sent"
                        })),
                    })
                }
                _ => Err(Error::plugin("notifications", "Unknown API route")),
            }
        }

        async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()> {
            match handler_id {
                "process_event_notification" => {
                    if event.event_type().contains("error") || event.event_type().contains("alert") {
                        tracing::info!("Processing notification for event: {}", event.event_type());
                    }
                    Ok(())
                }
                _ => Err(Error::plugin("notifications", "Unknown event handler")),
            }
        }
    }

    /// Register all built-in plugins
    pub async fn register_builtin_plugins() -> Result<()> {
        // Register System Monitor
        let system_monitor_factory = SimplePluginFactory::<SystemMonitorPlugin>::new(
            SystemMonitorPlugin::default().info()
        );
        PluginFactoryRegistry::register(system_monitor_factory).await?;

        // Register Notifications
        let notification_factory = SimplePluginFactory::<NotificationPlugin>::new(
            NotificationPlugin::default().info()
        );
        PluginFactoryRegistry::register(notification_factory).await?;

        tracing::info!("Registered all built-in plugins");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_factory_registry() {
        use builtin::SystemMonitorPlugin;

        let factory = SimplePluginFactory::<SystemMonitorPlugin>::new(
            SystemMonitorPlugin::default().info()
        );
        PluginFactoryRegistry::register(factory).await.unwrap();

        let plugins = PluginFactoryRegistry::list_plugins().await;
        assert!(plugins.contains(&"system_monitor".to_string()));

        let info = PluginFactoryRegistry::get_plugin_info("system_monitor").await;
        assert!(info.is_some());

        let plugin = PluginFactoryRegistry::create_plugin("system_monitor").await;
        assert!(plugin.is_some());
    }

    #[test]
    fn test_plugin_info() {
        use builtin::SystemMonitorPlugin;

        let plugin = SystemMonitorPlugin::default();
        let info = plugin.info();

        assert_eq!(info.id, "system_monitor");
        assert_eq!(info.name, "System Monitor");
        assert_eq!(info.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_builtin_plugins_registration() {
        builtin::register_builtin_plugins().await.unwrap();

        let plugins = PluginFactoryRegistry::list_plugins().await;
        assert!(plugins.contains(&"system_monitor".to_string()));
        assert!(plugins.contains(&"notifications".to_string()));
    }
}