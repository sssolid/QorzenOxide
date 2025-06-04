// src/plugin/sdk.rs

//! Plugin Development SDK
//!
//! This module provides convenient macros and utilities for plugin development.

use super::{ApiRoute, EventHandler, MenuItem, PluginContext, PluginInfo, UIComponent};
use crate::auth::Permission;
use crate::error::Result;
use crate::event::Event;
use crate::types::Metadata;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;

/// A generic event type for plugin-emitted events
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginEvent {
    pub event_type: String,
    pub plugin_id: String,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub metadata: Metadata,
}

#[allow(dead_code)]
impl PluginEvent {
    /// Create a new plugin event
    pub fn new(
        event_type: impl Into<String>,
        plugin_id: impl Into<String>,
        source: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            plugin_id: plugin_id.into(),
            source: source.into(),
            data,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Event for PluginEvent {
    fn event_type(&self) -> &'static str {
        // Since we need to return a &'static str, we'll need to use a leaked string
        // In practice, this should be handled differently, but for the SDK this works
        Box::leak(self.event_type.clone().into_boxed_str())
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// Create a plugin with basic metadata using a more ergonomic syntax
#[macro_export]
macro_rules! plugin {
    (
        id: $id:expr,
        name: $name:expr,
        version: $version:expr,
        author: $author:expr,
        description: $description:expr,
        $(license: $license:expr,)?
        $(permissions: [$($permission:expr),*],)?
        $(dependencies: [$($dep:expr),*],)?
        impl $impl_block:tt
    ) => {
        /// Auto-generated plugin struct
        #[derive(Debug)]
        pub struct QorzenPlugin {
            context: Option<$crate::plugin::PluginContext>,
        }

        impl QorzenPlugin {
            /// Create a new plugin instance
            pub fn new() -> Self {
                Self {
                    context: None,
                }
            }
        }

        #[async_trait::async_trait]
        impl $crate::plugin::Plugin for QorzenPlugin {
            fn info(&self) -> $crate::plugin::PluginInfo {
                $crate::plugin::PluginInfo {
                    id: $id.to_string(),
                    name: $name.to_string(),
                    version: $version.to_string(),
                    description: $description.to_string(),
                    author: $author.to_string(),
                    license: plugin!(@license $($license)?).to_string(),
                    homepage: None,
                    repository: None,
                    minimum_core_version: "0.1.0".to_string(),
                    supported_platforms: vec![$crate::plugin::Platform::All],
                }
            }

            fn required_dependencies(&self) -> Vec<$crate::plugin::PluginDependency> {
                vec![
                    $($(
                        $crate::plugin::PluginDependency {
                            plugin_id: $dep.to_string(),
                            version_requirement: "*".to_string(),
                            optional: false,
                        }
                    ),*)?
                ]
            }

            fn required_permissions(&self) -> Vec<$crate::auth::Permission> {
                let mut permissions = Vec::new();
                $($(
                    let parts: Vec<&str> = $permission.split('.').collect();
                    if parts.len() == 2 {
                        permissions.push($crate::auth::Permission {
                            resource: parts[0].to_string(),
                            action: parts[1].to_string(),
                            scope: $crate::auth::PermissionScope::Global,
                        });
                    }
                )*)?
                permissions
            }

            $impl_block
        }

        $crate::export_plugin!(QorzenPlugin);
    };

    (@license) => { "MIT" };
    (@license $license:expr) => { $license };
}

/// Create a search provider for a plugin with convenient syntax
#[macro_export]
macro_rules! search_provider {
    (
        id: $id:expr,
        name: $name:expr,
        description: $desc:expr,
        $(priority: $priority:expr,)?
        $(result_types: [$($result_type:expr),*],)?
        $(supports_facets: $facets:expr,)?
        $(supports_suggestions: $suggestions:expr,)?
        search: $search_fn:expr,
        $(suggestions: $suggestions_fn:expr,)?
        $(health_check: $health_fn:expr,)?
    ) => {
        /// Auto-generated search provider struct
        #[derive(Debug)]
        pub struct PluginSearchProvider {
            id: String,
        }

        impl PluginSearchProvider {
            /// Create a new search provider
            pub fn new() -> Self {
                Self {
                    id: $id.to_string(),
                }
            }
        }

        #[async_trait::async_trait]
        impl $crate::plugin::search::SearchProvider for PluginSearchProvider {
            fn provider_id(&self) -> &str {
                $id
            }

            fn provider_name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $desc
            }

            fn priority(&self) -> i32 {
                search_provider!(@priority $($priority)?)
            }

            fn supported_result_types(&self) -> Vec<String> {
                vec![
                    $($(String::from($result_type)),*)?
                ]
            }

            fn supports_facets(&self) -> bool {
                search_provider!(@supports_facets $($facets)?)
            }

            fn supports_suggestions(&self) -> bool {
                search_provider!(@supports_suggestions $($suggestions)?)
            }

            async fn search(&self, query: &$crate::plugin::search::SearchQuery)
                -> $crate::error::Result<Vec<$crate::plugin::search::SearchResult>> {
                $search_fn(query).await
            }

            $(
                async fn get_suggestions(&self, query: &$crate::plugin::search::SearchQuery)
                    -> $crate::error::Result<Vec<$crate::plugin::search::SearchSuggestion>> {
                    $suggestions_fn(query).await
                }
            )?

            async fn health_check(&self) -> $crate::error::Result<$crate::plugin::search::ProviderHealth> {
                $(
                    return $health_fn().await;
                )?

                // Default health check implementation
                Ok($crate::plugin::search::ProviderHealth {
                    is_healthy: true,
                    response_time_ms: Some(1),
                    error_message: None,
                    last_check: chrono::Utc::now(),
                })
            }
        }
    };

    (@priority) => { 100 };
    (@priority $priority:expr) => { $priority };
    (@supports_facets) => { false };
    (@supports_facets $facets:expr) => { $facets };
    (@supports_suggestions) => { false };
    (@supports_suggestions $suggestions:expr) => { $suggestions };
}

/// Create a UI component for a plugin with validation
#[macro_export]
macro_rules! ui_component {
    (
        id: $id:expr,
        name: $name:expr,
        component_type: $comp_type:expr,
        $(permissions: [$($permission:expr),*],)?
        render: $render_fn:expr
    ) => {
        /// Component render function
        pub fn $id(props: serde_json::Value) -> $crate::error::Result<dioxus::prelude::VNode> {
            $render_fn(props)
        }

        /// Get UI component metadata
        pub fn get_ui_component() -> $crate::plugin::UIComponent {
            $crate::plugin::UIComponent {
                id: stringify!($id).to_string(),
                name: $name.to_string(),
                component_type: $comp_type,
                props: serde_json::Value::Object(serde_json::Map::new()),
                required_permissions: vec![
                    $($(
                        {
                            let parts: Vec<&str> = $permission.split('.').collect();
                            if parts.len() == 2 {
                                $crate::auth::Permission {
                                    resource: parts[0].to_string(),
                                    action: parts[1].to_string(),
                                    scope: $crate::auth::PermissionScope::Global,
                                }
                            } else {
                                $crate::auth::Permission {
                                    resource: "unknown".to_string(),
                                    action: "unknown".to_string(),
                                    scope: $crate::auth::PermissionScope::Global,
                                }
                            }
                        }
                    ),*)?
                ],
            }
        }
    };
}

/// Create a menu item for a plugin with hierarchical support
#[macro_export]
macro_rules! menu_item {
    (
        id: $id:expr,
        label: $label:expr,
        $(icon: $icon:expr,)?
        $(route: $route:expr,)?
        $(action: $action:expr,)?
        $(order: $order:expr,)?
        $(permissions: [$($permission:expr),*],)?
        $(children: [$($child:expr),*],)?
    ) => {
        $crate::plugin::MenuItem {
            id: $id.to_string(),
            label: $label.to_string(),
            icon: menu_item!(@icon $($icon)?),
            route: menu_item!(@route $($route)?),
            action: menu_item!(@action $($action)?),
            required_permissions: vec![
                $($(
                    {
                        let parts: Vec<&str> = $permission.split('.').collect();
                        if parts.len() == 2 {
                            $crate::auth::Permission {
                                resource: parts[0].to_string(),
                                action: parts[1].to_string(),
                                scope: $crate::auth::PermissionScope::Global,
                            }
                        } else {
                            $crate::auth::Permission {
                                resource: "unknown".to_string(),
                                action: "unknown".to_string(),
                                scope: $crate::auth::PermissionScope::Global,
                            }
                        }
                    }
                ),*)?
            ],
            order: menu_item!(@order $($order)?),
            children: vec![
                $($($child),*)?
            ],
        }
    };

    (@icon) => { None };
    (@icon $icon:expr) => { Some($icon.to_string()) };
    (@route) => { None };
    (@route $route:expr) => { Some($route.to_string()) };
    (@action) => { None };
    (@action $action:expr) => { Some($action.to_string()) };
    (@order) => { 0 };
    (@order $order:expr) => { $order };
}

/// Create an API route definition with documentation
#[macro_export]
macro_rules! api_route {
    (
        path: $path:expr,
        method: $method:expr,
        handler: $handler:expr,
        $(permissions: [$($permission:expr),*],)?
        $(rate_limit: {
            requests_per_minute: $rpm:expr,
            burst_limit: $burst:expr
        },)?
        documentation: {
            summary: $summary:expr,
            description: $description:expr,
            $(parameters: [$($param:expr),*],)?
            $(responses: [$($response:expr),*],)?
        }
    ) => {
        $crate::plugin::ApiRoute {
            path: $path.to_string(),
            method: $method,
            handler_id: stringify!($handler).to_string(),
            required_permissions: vec![
                $($(
                    {
                        let parts: Vec<&str> = $permission.split('.').collect();
                        if parts.len() == 2 {
                            $crate::auth::Permission {
                                resource: parts[0].to_string(),
                                action: parts[1].to_string(),
                                scope: $crate::auth::PermissionScope::Global,
                            }
                        } else {
                            $crate::auth::Permission {
                                resource: "unknown".to_string(),
                                action: "unknown".to_string(),
                                scope: $crate::auth::PermissionScope::Global,
                            }
                        }
                    }
                ),*)?
            ],
            rate_limit: api_route!(@rate_limit $($rpm, $burst)?),
            documentation: $crate::plugin::ApiDocumentation {
                summary: $summary.to_string(),
                description: $description.to_string(),
                parameters: vec![
                    $($($param),*)?
                ],
                responses: vec![
                    $($($response),*)?
                ],
                examples: vec![],
            },
        }
    };

    (@rate_limit) => { None };
    (@rate_limit $rpm:expr, $burst:expr) => {
        Some($crate::plugin::RateLimit {
            requests_per_minute: $rpm,
            burst_limit: $burst,
        })
    };
}

/// Plugin development utilities and builder pattern
#[allow(dead_code)]
pub struct PluginBuilder {
    info: PluginInfo,
    permissions: Vec<Permission>,
    ui_components: Vec<UIComponent>,
    menu_items: Vec<MenuItem>,
    api_routes: Vec<ApiRoute>,
    event_handlers: Vec<EventHandler>,
}

#[allow(dead_code)]
impl PluginBuilder {
    /// Start building a new plugin
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        Self {
            info: PluginInfo {
                id: id.to_string(),
                name: name.to_string(),
                version: version.to_string(),
                description: String::new(),
                author: String::new(),
                license: "MIT".to_string(),
                homepage: None,
                repository: None,
                minimum_core_version: "0.1.0".to_string(),
                supported_platforms: vec![super::Platform::All],
            },
            permissions: Vec::new(),
            ui_components: Vec::new(),
            menu_items: Vec::new(),
            api_routes: Vec::new(),
            event_handlers: Vec::new(),
        }
    }

    /// Set plugin description
    pub fn description(mut self, description: &str) -> Self {
        self.info.description = description.to_string();
        self
    }

    /// Set plugin author
    pub fn author(mut self, author: &str) -> Self {
        self.info.author = author.to_string();
        self
    }

    /// Set plugin license
    pub fn license(mut self, license: &str) -> Self {
        self.info.license = license.to_string();
        self
    }

    /// Set plugin homepage
    pub fn homepage(mut self, homepage: &str) -> Self {
        self.info.homepage = Some(homepage.to_string());
        self
    }

    /// Set plugin repository
    pub fn repository(mut self, repository: &str) -> Self {
        self.info.repository = Some(repository.to_string());
        self
    }

    /// Add supported platform
    pub fn platform(mut self, platform: super::Platform) -> Self {
        if !self.info.supported_platforms.contains(&platform) {
            self.info.supported_platforms.push(platform);
        }
        self
    }

    /// Add required permission
    pub fn permission(mut self, resource: &str, action: &str) -> Self {
        self.permissions.push(Permission {
            resource: resource.to_string(),
            action: action.to_string(),
            scope: crate::auth::PermissionScope::Global,
        });
        self
    }

    /// Add UI component
    pub fn ui_component(mut self, component: UIComponent) -> Self {
        self.ui_components.push(component);
        self
    }

    /// Add menu item
    pub fn menu_item(mut self, item: MenuItem) -> Self {
        self.menu_items.push(item);
        self
    }

    /// Add API route
    pub fn api_route(mut self, route: ApiRoute) -> Self {
        self.api_routes.push(route);
        self
    }

    /// Add event handler
    pub fn event_handler(mut self, handler: EventHandler) -> Self {
        self.event_handlers.push(handler);
        self
    }

    /// Build the plugin metadata
    pub fn build(self) -> PluginMetadata {
        PluginMetadata {
            info: self.info,
            permissions: self.permissions,
            ui_components: self.ui_components,
            menu_items: self.menu_items,
            api_routes: self.api_routes,
            event_handlers: self.event_handlers,
        }
    }
}

/// Plugin metadata collection for development
#[allow(dead_code)]
pub struct PluginMetadata {
    pub info: PluginInfo,
    pub permissions: Vec<Permission>,
    pub ui_components: Vec<UIComponent>,
    pub menu_items: Vec<MenuItem>,
    pub api_routes: Vec<ApiRoute>,
    pub event_handlers: Vec<EventHandler>,
}

/// Helper traits for plugin development
#[allow(dead_code)]
pub trait PluginHelper {
    /// Get plugin context
    fn context(&self) -> &PluginContext;

    /// Log a message with the plugin context
    fn log(&self, level: &str, message: &str) {
        let plugin_id = &self.context().plugin_id;
        match level {
            "trace" => tracing::trace!("[Plugin:{}] {}", plugin_id, message),
            "debug" => tracing::debug!("[Plugin:{}] {}", plugin_id, message),
            "info" => tracing::info!("[Plugin:{}] {}", plugin_id, message),
            "warn" => tracing::warn!("[Plugin:{}] {}", plugin_id, message),
            "error" => tracing::error!("[Plugin:{}] {}", plugin_id, message),
            _ => tracing::info!("[Plugin:{}] {}", plugin_id, message),
        }
    }

    /// Get configuration value
    async fn get_config(&self, key: &str) -> Result<Option<Value>> {
        self.context().api_client.get_config(key).await
    }

    /// Set configuration value
    async fn set_config(&self, key: &str, value: Value) -> Result<()> {
        self.context().api_client.set_config(key, value).await
    }

    /// Check if user has permission
    async fn check_permission(&self, resource: &str, action: &str) -> Result<bool> {
        self.context()
            .api_client
            .check_permission(resource, action)
            .await
    }

    /// Get current user
    async fn get_user(&self) -> Result<Option<crate::auth::User>> {
        self.context().api_client.get_current_user().await
    }

    /// Emit an event using the proper event system
    async fn emit_event(&self, event_type: &str, data: Value) -> Result<()> {
        let plugin_context = self.context();

        // Create a proper plugin event that implements the Event trait
        let event = PluginEvent::new(
            event_type,
            &plugin_context.plugin_id,
            format!("plugin.{}", plugin_context.plugin_id),
            data,
        )
        .with_metadata(
            "plugin_version".to_string(),
            serde_json::Value::String(plugin_context.config.version.clone()),
        )
        .with_metadata(
            "plugin_config_id".to_string(),
            serde_json::Value::String(plugin_context.config.plugin_id.clone()),
        );

        // Use the event bus to publish the event
        plugin_context.event_bus.publish(event).await
    }

    /// Emit an event with custom metadata
    async fn emit_event_with_metadata(
        &self,
        event_type: &str,
        data: Value,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let plugin_context = self.context();

        let mut event = PluginEvent::new(
            event_type,
            &plugin_context.plugin_id,
            format!("plugin.{}", plugin_context.plugin_id),
            data,
        )
        .with_metadata(
            "plugin_version".to_string(),
            serde_json::Value::String(plugin_context.config.version.clone()),
        )
        .with_metadata(
            "plugin_config_id".to_string(),
            serde_json::Value::String(plugin_context.config.plugin_id.clone()),
        );

        // Add custom metadata
        for (key, value) in metadata {
            event = event.with_metadata(key, value);
        }

        plugin_context.event_bus.publish(event).await
    }
}

/// Generate plugin template files for development
pub struct PluginTemplate;

#[allow(dead_code)]
impl PluginTemplate {
    /// Generate a basic plugin template
    pub fn generate_basic(
        plugin_id: &str,
        plugin_name: &str,
        author: &str,
    ) -> HashMap<String, String> {
        let mut files = HashMap::new();

        // Cargo.toml
        files.insert(
            "Cargo.toml".to_string(),
            format!(
                r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
qorzen-oxide = {{ path = "../../" }}
async-trait = "0.1"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["macros"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
dioxus = "0.6"
"#,
                plugin_id
            ),
        );

        // src/lib.rs
        files.insert(
            "src/lib.rs".to_string(),
            format!(
                r#"
use qorzen_oxide::plugin::*;
use qorzen_oxide::{{plugin, export_plugin}};

plugin! {{
    id: "{}",
    name: "{}",
    version: "0.1.0",
    author: "{}",
    description: "A sample plugin for the Qorzen framework",
    permissions: ["data.read", "ui.render"],

    impl {{
        async fn initialize(&mut self, context: PluginContext) -> qorzen_oxide::error::Result<()> {{
            self.context = Some(context);
            tracing::info!("Plugin '{}' initialized successfully");
            Ok(())
        }}

        async fn shutdown(&mut self) -> qorzen_oxide::error::Result<()> {{
            tracing::info!("Plugin '{}' shutting down");
            Ok(())
        }}

        fn ui_components(&self) -> Vec<UIComponent> {{
            vec![]
        }}

        fn menu_items(&self) -> Vec<MenuItem> {{
            vec![]
        }}

        fn settings_schema(&self) -> Option<qorzen_oxide::config::SettingsSchema> {{
            None
        }}

        fn api_routes(&self) -> Vec<ApiRoute> {{
            vec![]
        }}

        fn event_handlers(&self) -> Vec<EventHandler> {{
            vec![]
        }}

        fn render_component(&self, component_id: &str, props: serde_json::Value)
            -> qorzen_oxide::error::Result<dioxus::prelude::VNode> {{
            Err(qorzen_oxide::error::Error::plugin(&self.info().id, "No components implemented"))
        }}

        async fn handle_api_request(&self, route_id: &str, request: ApiRequest)
            -> qorzen_oxide::error::Result<ApiResponse> {{
            Err(qorzen_oxide::error::Error::plugin(&self.info().id, "No API routes implemented"))
        }}

        async fn handle_event(&self, handler_id: &str, event: &dyn qorzen_oxide::event::Event)
            -> qorzen_oxide::error::Result<()> {{
            Ok(())
        }}
    }}
}}
"#,
                plugin_id, plugin_name, author, plugin_name, plugin_name
            ),
        );

        // plugin.toml
        files.insert(
            "plugin.toml".to_string(),
            format!(
                r#"
[plugin]
id = "{}"
name = "{}"
version = "0.1.0"
description = "A sample plugin for the Qorzen framework"
author = "{}"
license = "MIT"
minimum_core_version = "0.1.0"
api_version = "0.1.0"

[build]
entry = "src/lib.rs"
sources = ["src/**/*.rs"]
features = ["default"]
hot_reload = true

[targets.web]
platform = "web"
arch = ["wasm32"]
features = ["web"]

[targets.desktop]
platform = "desktop"
arch = ["x86_64", "aarch64"]
os = ["windows", "macos", "linux"]
features = ["desktop"]

permissions = [
    "data.read",
    "ui.render"
]

provides = [
    "example.functionality"
]

requires = [
    "core.events"
]
"#,
                plugin_id, plugin_name, author
            ),
        );

        // README.md
        files.insert(
            "README.md".to_string(),
            format!(
                r#"
# {}

{}

## Building

```bash
cargo build --release
```

## Installation

Register the plugin factory in your application and it will be available for loading.

## Features

- Basic plugin functionality
- Safe loading without dynamic library risks
- Configurable settings
- Event handling

## Configuration

This plugin supports the following configuration options:

- `enabled`: Enable/disable the plugin
- `setting1`: Example setting

## License

MIT
"#,
                plugin_name, "A sample plugin demonstrating the Qorzen plugin system"
            ),
        );

        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_builder() {
        let metadata = PluginBuilder::new("test_plugin", "Test Plugin", "1.0.0")
            .description("A test plugin")
            .author("Test Author")
            .homepage("https://example.com")
            .repository("https://github.com/example/plugin")
            .permission("data", "read")
            .permission("ui", "render")
            .build();

        assert_eq!(metadata.info.id, "test_plugin");
        assert_eq!(metadata.info.name, "Test Plugin");
        assert_eq!(
            metadata.info.homepage,
            Some("https://example.com".to_string())
        );
        assert_eq!(metadata.permissions.len(), 2);
    }

    #[test]
    fn test_plugin_template_generation() {
        let files =
            PluginTemplate::generate_basic("example_plugin", "Example Plugin", "Test Author");

        assert!(files.contains_key("Cargo.toml"));
        assert!(files.contains_key("src/lib.rs"));
        assert!(files.contains_key("plugin.toml"));
        assert!(files.contains_key("README.md"));

        let cargo_toml = files.get("Cargo.toml").unwrap();
        assert!(cargo_toml.contains("example_plugin"));

        let lib_rs = files.get("src/lib.rs").unwrap();
        assert!(lib_rs.contains("Example Plugin"));
        assert!(lib_rs.contains("Test Author"));
    }

    #[test]
    fn test_builder_platform_support() {
        let metadata = PluginBuilder::new("test", "Test", "1.0.0")
            .platform(super::super::Platform::Web)
            .platform(super::super::Platform::Windows)
            .platform(super::super::Platform::Web) // Duplicate should be ignored
            .build();

        assert_eq!(metadata.info.supported_platforms.len(), 3); // All, Web, Windows
        assert!(metadata
            .info
            .supported_platforms
            .contains(&super::super::Platform::Web));
        assert!(metadata
            .info
            .supported_platforms
            .contains(&super::super::Platform::Windows));
    }

    #[test]
    fn test_plugin_event_creation() {
        let event = PluginEvent::new(
            "test.event",
            "test_plugin",
            "plugin.test_plugin",
            serde_json::json!({"key": "value"}),
        )
        .with_metadata("extra", serde_json::json!("metadata"));

        assert_eq!(event.plugin_id, "test_plugin");
        assert_eq!(event.source, "plugin.test_plugin");
        assert!(event.metadata.contains_key("extra"));
    }
}
