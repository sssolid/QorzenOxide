// src/plugin/config.rs - Plugin system configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{Error, Result};

/// Plugin configuration loaded from config files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManagerConfig {
    /// Whether to auto-load plugins on startup
    pub auto_load: bool,
    /// Whether hot reload is enabled (desktop only)
    pub hot_reload: bool,
    /// List of plugins to load by default
    pub default_plugins: Vec<String>,
    /// Plugin-specific configurations
    pub plugin_configs: HashMap<String, serde_json::Value>,
    /// Registry URL for downloading plugins
    pub registry_url: String,
    /// Maximum number of concurrent plugin operations
    pub max_concurrent_operations: usize,
    /// Plugin loading timeout in seconds
    pub loading_timeout_secs: u64,
    /// Enable plugin sandboxing
    pub enable_sandboxing: bool,
    /// Plugin cache settings
    pub cache_settings: PluginCacheSettings,
}

/// Built-in plugin registry helper for initialization
pub struct BuiltinPluginRegistry;

impl BuiltinPluginRegistry {
    /// Initialize all built-in plugins
    pub async fn register_builtin_plugins() -> Result<()> {
        // This function is now a wrapper around the builtin module's function
        super::registry::builtin::register_builtin_plugins().await
    }
}

/// Plugin cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCacheSettings {
    /// Enable caching of plugin metadata
    pub enable_metadata_cache: bool,
    /// Cache TTL in seconds
    pub metadata_cache_ttl_secs: u64,
    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,
    /// Cache directory path (relative to plugins directory)
    pub cache_directory: String,
}

impl Default for PluginCacheSettings {
    fn default() -> Self {
        Self {
            enable_metadata_cache: true,
            metadata_cache_ttl_secs: 3600, // 1 hour
            max_cache_size_mb: 100,
            cache_directory: "cache".to_string(),
        }
    }
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            auto_load: true,
            hot_reload: cfg!(debug_assertions) && !cfg!(target_arch = "wasm32"),
            default_plugins: vec![
                "system_monitor".to_string(),
                "notifications".to_string(),
            ],
            plugin_configs: {
                let mut configs = HashMap::new();

                // System monitor default config
                configs.insert("system_monitor".to_string(), serde_json::json!({
                    "refresh_interval": 5000,
                    "show_cpu": true,
                    "show_memory": true,
                    "alert_threshold": 80.0
                }));

                // Notifications default config
                configs.insert("notifications".to_string(), serde_json::json!({
                    "email_enabled": true,
                    "push_enabled": false,
                    "default_priority": "normal"
                }));

                configs
            },
            registry_url: "https://registry.qorzen.com".to_string(),
            max_concurrent_operations: 10,
            loading_timeout_secs: 30,
            enable_sandboxing: true,
            cache_settings: PluginCacheSettings::default(),
        }
    }
}

impl PluginManagerConfig {
    /// Load configuration from a TOML file
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_from_file(path: &str) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            Error::file(
                path.to_string(),
                crate::error::FileOperation::Read,
                format!("Failed to read plugin config file: {}", e),
            )
        })?;

        Self::load_from_str(&content)
    }

    /// Load configuration from a TOML string
    pub fn load_from_str(content: &str) -> Result<Self> {
        let config: PluginManagerConfig = toml::from_str(content).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Failed to parse plugin config TOML: {}", e),
            )
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from platform file system (for WASM)
    pub async fn load_from_platform(
        path: &str,
        filesystem: &dyn crate::platform::filesystem::FileSystemProvider,
    ) -> Result<Self> {
        let content_bytes = filesystem.read_file(path).await.map_err(|e| {
            Error::file(
                path.to_string(),
                crate::error::FileOperation::Read,
                format!("Failed to read plugin config via platform: {}", e),
            )
        })?;

        let content = String::from_utf8(content_bytes).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Invalid UTF-8 in plugin config file: {}", e),
            )
        })?;

        Self::load_from_str(&content)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate registry URL
        if self.registry_url.is_empty() {
            return Err(Error::new(
                crate::error::ErrorKind::Validation,
                "Registry URL cannot be empty".to_string(),
            ));
        }

        // Validate URL format
        if !self.registry_url.starts_with("http://") && !self.registry_url.starts_with("https://") {
            return Err(Error::new(
                crate::error::ErrorKind::Validation,
                "Registry URL must be a valid HTTP/HTTPS URL".to_string(),
            ));
        }

        // Validate concurrent operations limit
        if self.max_concurrent_operations == 0 {
            return Err(Error::new(
                crate::error::ErrorKind::Validation,
                "Max concurrent operations must be greater than 0".to_string(),
            ));
        }

        if self.max_concurrent_operations > 100 {
            return Err(Error::new(
                crate::error::ErrorKind::Validation,
                "Max concurrent operations should not exceed 100".to_string(),
            ));
        }

        // Validate loading timeout
        if self.loading_timeout_secs == 0 {
            return Err(Error::new(
                crate::error::ErrorKind::Validation,
                "Loading timeout must be greater than 0".to_string(),
            ));
        }

        // Validate cache settings
        if self.cache_settings.max_cache_size_mb == 0 {
            return Err(Error::new(
                crate::error::ErrorKind::Validation,
                "Max cache size must be greater than 0".to_string(),
            ));
        }

        // Validate plugin IDs in default_plugins list
        for plugin_id in &self.default_plugins {
            if plugin_id.is_empty() {
                return Err(Error::new(
                    crate::error::ErrorKind::Validation,
                    "Plugin ID in default_plugins cannot be empty".to_string(),
                ));
            }

            if !plugin_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(Error::new(
                    crate::error::ErrorKind::Validation,
                    format!("Invalid plugin ID '{}': must contain only alphanumeric characters, underscores, and hyphens", plugin_id),
                ));
            }
        }

        // Validate plugin configurations
        for (plugin_id, _config) in &self.plugin_configs {
            if plugin_id.is_empty() {
                return Err(Error::new(
                    crate::error::ErrorKind::Validation,
                    "Plugin ID in plugin_configs cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Serialize configuration to TOML string
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string(self).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Failed to serialize plugin config to TOML: {}", e),
            )
        })
    }

    /// Save configuration to file
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn save_to_file(&self, path: &str) -> Result<()> {
        let content = self.to_toml_string()?;
        tokio::fs::write(path, content).await.map_err(|e| {
            Error::file(
                path.to_string(),
                crate::error::FileOperation::Write,
                format!("Failed to write plugin config file: {}", e),
            )
        })
    }

    /// Get configuration for a specific plugin
    pub fn get_plugin_config(&self, plugin_id: &str) -> Option<&serde_json::Value> {
        self.plugin_configs.get(plugin_id)
    }

    /// Set configuration for a specific plugin
    pub fn set_plugin_config(&mut self, plugin_id: String, config: serde_json::Value) {
        self.plugin_configs.insert(plugin_id, config);
    }

    /// Remove configuration for a specific plugin
    pub fn remove_plugin_config(&mut self, plugin_id: &str) -> Option<serde_json::Value> {
        self.plugin_configs.remove(plugin_id)
    }

    /// Add a plugin to the default plugins list
    pub fn add_default_plugin(&mut self, plugin_id: String) {
        if !self.default_plugins.contains(&plugin_id) {
            self.default_plugins.push(plugin_id);
        }
    }

    /// Remove a plugin from the default plugins list
    pub fn remove_default_plugin(&mut self, plugin_id: &str) {
        self.default_plugins.retain(|id| id != plugin_id);
    }

    /// Check if a plugin is in the default plugins list
    pub fn is_default_plugin(&self, plugin_id: &str) -> bool {
        self.default_plugins.contains(&plugin_id.to_string())
    }

    /// Get all configured plugin IDs
    pub fn get_configured_plugin_ids(&self) -> Vec<&str> {
        self.plugin_configs.keys().map(|s| s.as_str()).collect()
    }

    /// Create a minimal configuration for testing
    pub fn minimal() -> Self {
        Self {
            auto_load: false,
            hot_reload: false,
            default_plugins: vec![],
            plugin_configs: HashMap::new(),
            registry_url: "https://registry.example.com".to_string(),
            max_concurrent_operations: 5,
            loading_timeout_secs: 10,
            enable_sandboxing: true,
            cache_settings: PluginCacheSettings {
                enable_metadata_cache: false,
                metadata_cache_ttl_secs: 60,
                max_cache_size_mb: 10,
                cache_directory: "cache".to_string(),
            },
        }
    }

    /// Create an example configuration
    pub fn example() -> Self {
        Self::default()
    }

    /// Merge another configuration into this one
    pub fn merge(&mut self, other: PluginManagerConfig) {
        // Merge plugin configs
        for (plugin_id, config) in other.plugin_configs {
            self.plugin_configs.insert(plugin_id, config);
        }

        // Merge default plugins (avoiding duplicates)
        for plugin_id in other.default_plugins {
            if !self.default_plugins.contains(&plugin_id) {
                self.default_plugins.push(plugin_id);
            }
        }

        // Override other settings (last one wins)
        self.auto_load = other.auto_load;
        self.hot_reload = other.hot_reload;
        self.registry_url = other.registry_url;
        self.max_concurrent_operations = other.max_concurrent_operations;
        self.loading_timeout_secs = other.loading_timeout_secs;
        self.enable_sandboxing = other.enable_sandboxing;
        self.cache_settings = other.cache_settings;
    }

    /// Check if configuration has changed compared to another config
    pub fn has_changed(&self, other: &PluginManagerConfig) -> bool {
        self.auto_load != other.auto_load
            || self.hot_reload != other.hot_reload
            || self.default_plugins != other.default_plugins
            || self.plugin_configs != other.plugin_configs
            || self.registry_url != other.registry_url
            || self.max_concurrent_operations != other.max_concurrent_operations
            || self.loading_timeout_secs != other.loading_timeout_secs
            || self.enable_sandboxing != other.enable_sandboxing
    }
}

/// Built-in plugin registry helper for configuration integration
pub struct PluginConfigurationRegistry;

impl PluginConfigurationRegistry {
    /// Initialize default configurations for built-in plugins
    pub fn initialize_builtin_configs() -> HashMap<String, serde_json::Value> {
        let mut configs = HashMap::new();

        // System monitor plugin configuration
        configs.insert("system_monitor".to_string(), serde_json::json!({
            "refresh_interval": 5000,
            "show_cpu": true,
            "show_memory": true,
            "show_disk": true,
            "alert_threshold": 80.0,
            "enable_alerts": true,
            "enable_logging": true
        }));

        // Notification plugin configuration
        configs.insert("notifications".to_string(), serde_json::json!({
            "email_enabled": true,
            "push_enabled": false,
            "sms_enabled": false,
            "default_priority": "normal",
            "rate_limit": {
                "max_per_minute": 10,
                "max_per_hour": 100
            },
            "channels": {
                "email": {
                    "smtp_server": "",
                    "smtp_port": 587,
                    "use_tls": true
                },
                "push": {
                    "service": "fcm",
                    "api_key": ""
                }
            }
        }));

        configs
    }

    /// Get default configuration for a specific built-in plugin
    pub fn get_builtin_config(plugin_id: &str) -> Option<serde_json::Value> {
        let configs = Self::initialize_builtin_configs();
        configs.get(plugin_id).cloned()
    }

    /// Update configuration with built-in plugin defaults
    pub fn update_with_builtin_defaults(config: &mut PluginManagerConfig) {
        let builtin_configs = Self::initialize_builtin_configs();

        for (plugin_id, default_config) in builtin_configs {
            // Only add if not already configured
            if !config.plugin_configs.contains_key(&plugin_id) {
                config.plugin_configs.insert(plugin_id, default_config);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PluginManagerConfig::default();
        assert!(config.auto_load);
        assert!(!config.default_plugins.is_empty());
        assert!(config.plugin_configs.contains_key("system_monitor"));
        assert!(config.plugin_configs.contains_key("notifications"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = PluginManagerConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid registry URL
        config.registry_url = "invalid-url".to_string();
        assert!(config.validate().is_err());

        // Test empty registry URL
        config.registry_url = String::new();
        assert!(config.validate().is_err());

        // Test invalid concurrent operations
        config.registry_url = "https://example.com".to_string();
        config.max_concurrent_operations = 0;
        assert!(config.validate().is_err());

        config.max_concurrent_operations = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_minimal_config() {
        let config = PluginManagerConfig::minimal();
        assert!(!config.auto_load);
        assert!(!config.hot_reload);
        assert!(config.default_plugins.is_empty());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_plugin_config_management() {
        let mut config = PluginManagerConfig::minimal();

        // Test adding plugin config
        let plugin_config = serde_json::json!({"setting": "value"});
        config.set_plugin_config("test_plugin".to_string(), plugin_config.clone());
        assert_eq!(config.get_plugin_config("test_plugin"), Some(&plugin_config));

        // Test removing plugin config
        let removed = config.remove_plugin_config("test_plugin");
        assert_eq!(removed, Some(plugin_config));
        assert!(config.get_plugin_config("test_plugin").is_none());
    }

    #[test]
    fn test_default_plugin_management() {
        let mut config = PluginManagerConfig::minimal();

        // Test adding default plugin
        config.add_default_plugin("test_plugin".to_string());
        assert!(config.is_default_plugin("test_plugin"));

        // Test not adding duplicate
        config.add_default_plugin("test_plugin".to_string());
        assert_eq!(config.default_plugins.len(), 1);

        // Test removing default plugin
        config.remove_default_plugin("test_plugin");
        assert!(!config.is_default_plugin("test_plugin"));
    }

    #[test]
    fn test_config_serialization() {
        let config = PluginManagerConfig::example();
        let toml_str = config.to_toml_string().unwrap();
        let parsed = PluginManagerConfig::load_from_str(&toml_str).unwrap();

        assert_eq!(config.auto_load, parsed.auto_load);
        assert_eq!(config.default_plugins, parsed.default_plugins);
        assert_eq!(config.registry_url, parsed.registry_url);
    }

    #[tokio::test]
    async fn test_config_merge() {
        let mut config1 = PluginManagerConfig::minimal();
        config1.add_default_plugin("plugin1".to_string());
        config1.set_plugin_config("plugin1".to_string(), serde_json::json!({"a": 1}));

        let mut config2 = PluginManagerConfig::minimal();
        config2.add_default_plugin("plugin2".to_string());
        config2.set_plugin_config("plugin2".to_string(), serde_json::json!({"b": 2}));
        config2.auto_load = true;

        config1.merge(config2);

        assert!(config1.auto_load);
        assert_eq!(config1.default_plugins.len(), 2);
        assert!(config1.plugin_configs.contains_key("plugin1"));
        assert!(config1.plugin_configs.contains_key("plugin2"));
    }

    #[test]
    fn test_builtin_configs() {
        let configs = PluginConfigurationRegistry::initialize_builtin_configs();
        assert!(configs.contains_key("system_monitor"));
        assert!(configs.contains_key("notifications"));

        let system_config = PluginConfigurationRegistry::get_builtin_config("system_monitor");
        assert!(system_config.is_some());
    }

    #[test]
    fn test_config_change_detection() {
        let config1 = PluginManagerConfig::default();
        let mut config2 = config1.clone();

        assert!(!config1.has_changed(&config2));

        config2.auto_load = false;
        assert!(config1.has_changed(&config2));
    }

    #[test]
    fn test_cache_settings() {
        let config = PluginManagerConfig::default();
        assert!(config.cache_settings.enable_metadata_cache);
        assert_eq!(config.cache_settings.metadata_cache_ttl_secs, 3600);
        assert_eq!(config.cache_settings.max_cache_size_mb, 100);
    }
}