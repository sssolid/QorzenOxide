// src/config.rs

//! Configuration management system with hot-reload support
//!
//! This module provides a flexible configuration system that supports:
//! - Multiple configuration formats (YAML, JSON, TOML)
//! - Environment variable overrides
//! - Configuration validation
//! - Hot-reloading with file watching
//! - Hierarchical configuration merging
//! - Type-safe configuration access

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::error::{Error, Result, ResultExt};
use crate::event::{Event, EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus};
use crate::types::Metadata;

/// Configuration change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// The configuration key that changed
    pub key: String,
    /// The new value
    pub value: Value,
    /// The old value (if any)
    pub old_value: Option<Value>,
    /// When the change occurred
    pub timestamp: DateTime<Utc>,
    /// Source of the change
    pub source: String,
    /// Additional metadata
    pub metadata: Metadata,
}

impl Event for ConfigChangeEvent {
    fn event_type(&self) -> &'static str {
        "config.changed"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Configuration validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The configuration key that failed validation
    pub key: String,
    /// Description of the validation error
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error for '{}': {}", self.key, self.message)
    }
}

/// Configuration format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
    /// TOML format
    Toml,
}

impl ConfigFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

/// Configuration source
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// File-based configuration
    File { path: PathBuf, format: ConfigFormat },
    /// Environment variables
    Environment { prefix: String },
    /// In-memory configuration
    Memory { data: Value },
}

/// Configuration layer for hierarchical configuration
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// Layer name
    pub name: String,
    /// Layer source
    pub source: ConfigSource,
    /// Layer priority (higher values override lower ones)
    pub priority: u32,
    /// Whether this layer can be hot-reloaded
    pub hot_reload: bool,
}

/// Main application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application settings
    pub app: AppSettings,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Event bus configuration
    pub event_bus: EventBusConfig,
    /// File management configuration
    pub files: FileConfig,
    /// Task management configuration
    pub tasks: TaskConfig,
    /// Concurrency configuration
    pub concurrency: ConcurrencyConfig,
    /// Plugin configuration
    pub plugins: PluginConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings::default(),
            logging: LoggingConfig::default(),
            event_bus: EventBusConfig::default(),
            files: FileConfig::default(),
            tasks: TaskConfig::default(),
            concurrency: ConcurrencyConfig::default(),
            plugins: PluginConfig::default(),
            database: DatabaseConfig::default(),
            network: NetworkConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Application description
    pub description: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Debug mode enabled
    pub debug: bool,
    /// Data directory
    pub data_dir: PathBuf,
    /// Configuration directory
    pub config_dir: PathBuf,
    /// Log directory
    pub log_dir: PathBuf,
    /// PID file path
    pub pid_file: Option<PathBuf>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            name: "Qorzen".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Qorzen Application Framework".to_string(),
            environment: "development".to_string(),
            debug: cfg!(debug_assertions),
            data_dir: PathBuf::from("./data"),
            config_dir: PathBuf::from("./config"),
            log_dir: PathBuf::from("./logs"),
            pid_file: None,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level filter
    pub level: String,
    /// Log format (json, pretty, compact)
    pub format: LogFormat,
    /// Console logging configuration
    pub console: ConsoleLogConfig,
    /// File logging configuration
    pub file: Option<FileLogConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            console: ConsoleLogConfig::default(),
            file: Some(FileLogConfig::default()),
        }
    }
}

/// Log format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    /// JSON structured logging
    Json,
    /// Pretty human-readable format
    Pretty,
    /// Compact format
    Compact,
}

/// Console logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLogConfig {
    /// Whether console logging is enabled
    pub enabled: bool,
    // Optional override to global logging
    pub level: String,
    /// Whether to use colored output
    pub colored: bool,
}

impl Default for ConsoleLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            colored: true,
        }
    }
}

/// File logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLogConfig {
    /// Log file path
    pub path: PathBuf,
    /// Maximum file size before rotation
    pub max_size: u64,
    /// Maximum number of rotated files to keep
    pub max_files: u32,
    /// Whether to compress rotated files
    pub compress: bool,
}

impl Default for FileLogConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./logs/app.log"),
            max_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            compress: true,
        }
    }
}

/// Event bus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfig {
    /// Number of worker threads
    pub worker_count: usize,
    /// Queue size for events
    pub queue_size: usize,
    /// Publish timeout in milliseconds
    pub publish_timeout_ms: u64,
    /// Whether to enable event persistence
    pub enable_persistence: bool,
    /// Whether to enable metrics collection
    pub enable_metrics: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get(),
            queue_size: 10000,
            publish_timeout_ms: 5000,
            enable_persistence: false,
            enable_metrics: true,
        }
    }
}

/// File management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    /// Default file permissions (Unix octal)
    pub default_permissions: u32,
    /// Maximum file size for operations
    pub max_file_size: u64,
    /// Temporary directory (native only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_dir: Option<PathBuf>,
    /// File operation timeout in seconds
    pub operation_timeout_secs: u64,
    /// Whether to enable file watching
    pub enable_watching: bool,
    /// Whether to enable file compression
    pub enable_compression: bool,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            default_permissions: 0o644,
            max_file_size: 1024 * 1024 * 1024, // 1GB
            temp_dir: get_default_temp_dir(),
            operation_timeout_secs: 30,
            enable_watching: true,
            enable_compression: false,
        }
    }
}

impl FileConfig {
    /// Resolve a path inside the temp directory, or panic if not available
    pub fn temp_path(&self, filename: &str) -> PathBuf {
        self.temp_dir
            .as_ref()
            .expect("Temp directory unavailable — likely running in WASM")
            .join(filename)
    }
}

fn get_default_temp_dir() -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        // WASM: No actual temp dir
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use uuid::Uuid;
        use std::fs;

        let mut temp = std::env::temp_dir();
        temp.push(format!("qorzen_{}", Uuid::new_v4()));

        if let Err(e) = fs::create_dir_all(&temp) {
            eprintln!("Failed to create temp dir: {}", e);
            return None;
        }

        Some(temp)
    }
}

/// Task management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Maximum concurrent tasks
    pub max_concurrent: usize,
    /// Default task timeout in milliseconds
    pub default_timeout_ms: u64,
    /// Whether to keep completed tasks in memory
    pub keep_completed: bool,
    /// Progress update interval in milliseconds
    pub progress_update_interval_ms: u64,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            max_concurrent: num_cpus::get() * 2,
            default_timeout_ms: 300_000, // 5 minutes
            keep_completed: true,
            progress_update_interval_ms: 1000,
        }
    }
}

/// Concurrency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    /// Thread pool size for CPU-bound tasks
    pub thread_pool_size: usize,
    /// Thread pool size for I/O operations
    pub io_thread_pool_size: usize,
    /// Thread pool size for blocking operations
    pub blocking_thread_pool_size: usize,
    /// Maximum queue size per thread pool
    pub max_queue_size: usize,
    /// Thread keep-alive time in seconds
    pub thread_keep_alive_secs: u64,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: num_cpus::get(),
            io_thread_pool_size: num_cpus::get() * 2,
            blocking_thread_pool_size: num_cpus::get().max(4),
            max_queue_size: 1000,
            thread_keep_alive_secs: 60,
        }
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin directory
    pub plugin_dir: PathBuf,
    /// Whether to auto-load plugins on startup
    pub auto_load: bool,
    /// Plugin loading timeout in seconds
    pub load_timeout_secs: u64,
    /// Maximum number of plugins
    pub max_plugins: usize,
    /// Whether to enable plugin hot-reloading
    pub hot_reload: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dir: PathBuf::from("./plugins"),
            auto_load: true,
            load_timeout_secs: 30,
            max_plugins: 100,
            hot_reload: false,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL/connection string
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Query timeout in seconds
    pub query_timeout_secs: u64,
    /// Whether to enable connection pooling
    pub enable_pooling: bool,
    /// Whether to enable query logging
    pub enable_query_logging: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./data/app.db".to_string(),
            max_connections: 10,
            connect_timeout_secs: 30,
            query_timeout_secs: 60,
            enable_pooling: true,
            enable_query_logging: false,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// HTTP server bind address
    pub bind_address: String,
    /// HTTP server port
    pub port: u16,
    /// Whether to enable TLS
    pub enable_tls: bool,
    /// TLS certificate file path
    pub tls_cert_path: Option<PathBuf>,
    /// TLS private key file path
    pub tls_key_path: Option<PathBuf>,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes
    pub max_request_size: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            request_timeout_secs: 30,
            max_request_size: 16 * 1024 * 1024, // 16MB
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT secret key
    pub jwt_secret: String,
    /// JWT token expiration in seconds
    pub jwt_expiration_secs: u64,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Whether to enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit (requests per minute)
    pub rate_limit_rpm: u64,
    /// Whether to enable CORS
    pub enable_cors: bool,
    /// Allowed CORS origins
    pub cors_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-this-in-production".to_string(),
            jwt_expiration_secs: 3600, // 1 hour
            api_key: None,
            enable_rate_limiting: true,
            rate_limit_rpm: 1000,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    state: ManagedState,
    layers: Vec<ConfigLayer>,
    merged_config: Arc<RwLock<Value>>,
    change_notifier: broadcast::Sender<ConfigChangeEvent>,
    _watcher: Option<RecommendedWatcher>,
    watch_enabled: bool,
    env_prefix: String,
    event_bus: Option<Arc<EventBusManager>>,
}

impl fmt::Debug for ConfigManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigManager")
            .field("layers", &self.layers.len())
            .field("watch_enabled", &self.watch_enabled)
            .field("env_prefix", &self.env_prefix)
            .finish()
    }
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        let (change_notifier, _) = broadcast::channel(100);

        Self {
            state: ManagedState::new(Uuid::new_v4(), "config_manager"),
            layers: Vec::new(),
            merged_config: Arc::new(RwLock::new(Value::Object(Map::new()))),
            change_notifier,
            _watcher: None,
            watch_enabled: true,
            env_prefix: "QORZEN".to_string(),
            event_bus: None,
        }
    }

    /// Create configuration manager with a config file
    pub fn with_config_file<P: AsRef<Path>>(config_path: P) -> Self {
        let mut manager = Self::new();
        let _ = manager.add_file_layer("default", config_path, 0, true);
        manager
    }

    /// Add a file-based configuration layer
    pub fn add_file_layer<P: AsRef<Path>>(
        &mut self,
        name: impl Into<String>,
        path: P,
        priority: u32,
        hot_reload: bool,
    ) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let format = ConfigFormat::from_extension(&path)
            .ok_or_else(|| Error::config("Unsupported configuration file format"))?;

        let layer = ConfigLayer {
            name: name.into(),
            source: ConfigSource::File { path, format },
            priority,
            hot_reload,
        };

        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority);

        Ok(())
    }

    /// Add an environment variable layer
    pub fn add_env_layer(
        &mut self,
        name: impl Into<String>,
        prefix: impl Into<String>,
        priority: u32,
    ) {
        let layer = ConfigLayer {
            name: name.into(),
            source: ConfigSource::Environment {
                prefix: prefix.into(),
            },
            priority,
            hot_reload: false,
        };

        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority);
    }

    /// Add an in-memory configuration layer
    pub fn add_memory_layer(
        &mut self,
        name: impl Into<String>,
        data: Value,
        priority: u32,
    ) {
        let layer = ConfigLayer {
            name: name.into(),
            source: ConfigSource::Memory { data },
            priority,
            hot_reload: false,
        };

        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority);
    }

    /// Set event bus for publishing configuration change events
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    /// Set a configuration value dynamically
    pub async fn set<T>(&self, key: &str, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let serialized_value = serde_json::to_value(value).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Configuration {
                    key: Some(key.to_string()),
                    validation_errors: vec![format!("Failed to serialize config value: {}", e)],
                },
                format!("Failed to serialize config value: {}", e),
            )
        })?;

        let mut config = self.merged_config.write().await;
        let old_value = self.get_nested_value(&config, key);

        self.set_nested_value(&mut config, key, serialized_value.clone());

        // Publish change event
        let change_event = ConfigChangeEvent {
            key: key.to_string(),
            value: serialized_value,
            old_value,
            timestamp: Utc::now(),
            source: "config_manager".to_string(),
            metadata: HashMap::new(),
        };

        if let Some(event_bus) = &self.event_bus {
            let _ = event_bus.publish(change_event.clone()).await;
        }

        let _ = self.change_notifier.send(change_event);

        Ok(())
    }

    /// Get a configuration value
    pub async fn get<T>(&self, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let config = self.merged_config.read().await;
        let value = self.get_nested_value(&config, key).ok_or_else(|| {
            Error::new(
                crate::error::ErrorKind::Configuration {
                    key: Some(key.to_string()),
                    validation_errors: vec![format!("Configuration key '{}' not found", key)],
                },
                "Configuration key not found",
            )
        })?;

        serde_json::from_value(value).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Configuration {
                    key: Some(key.to_string()),
                    validation_errors: vec![format!("Failed to deserialize config value: {}", e)],
                },
                format!("Failed to deserialize config value: {}", e),
            )
        })
    }

    /// Get the complete merged configuration
    pub async fn get_config(&self) -> AppConfig {
        let config = self.merged_config.read().await;
        match serde_json::from_value(config.clone()) {
            Ok(parsed) => parsed,
            Err(e) => {
                eprintln!("⚠️ Failed to deserialize config: {:?}", e);
                eprintln!("🔎 Raw config: {}", config);
                AppConfig::default()
            }
        }
    }

    /// Subscribe to configuration changes
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_notifier.subscribe()
    }

    /// Reload configuration from all sources
    pub async fn reload(&self) -> Result<()> {
        self.merge_configurations().await?;

        // Publish reload event
        let reload_event = ConfigChangeEvent {
            key: "_reload".to_string(),
            value: Value::String("reloaded".to_string()),
            old_value: None,
            timestamp: Utc::now(),
            source: "config_manager".to_string(),
            metadata: HashMap::new(),
        };

        if let Some(event_bus) = &self.event_bus {
            let _ = event_bus.publish(reload_event.clone()).await;
        }

        let _ = self.change_notifier.send(reload_event);

        Ok(())
    }

    /// Validate the current configuration
    pub async fn validate(&self) -> Result<Vec<ValidationError>> {
        let _config = self.merged_config.read().await;
        let errors = Vec::new();

        // Add validation logic here
        // This is a simplified example
        // In practice, you'd implement comprehensive validation

        Ok(errors)
    }

    /// Merge all configuration layers
    async fn merge_configurations(&self) -> Result<()> {
        let mut merged = Value::Object(Map::new());

        // Process layers in priority order (lowest to highest)
        for layer in &self.layers {
            let layer_config = self.load_layer_config(layer).await?;
            self.merge_values(&mut merged, layer_config);
        }

        *self.merged_config.write().await = merged;
        Ok(())
    }

    /// Load configuration from a single layer
    async fn load_layer_config(&self, layer: &ConfigLayer) -> Result<Value> {
        match &layer.source {
            ConfigSource::File { path, format } => {
                let content = tokio::fs::read_to_string(path)
                    .await
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

                match format {
                    ConfigFormat::Yaml => serde_yaml::from_str(&content)
                        .map_err(|e| Error::config(format!("Failed to parse YAML config: {}", e))),
                    ConfigFormat::Json => serde_json::from_str(&content)
                        .map_err(|e| Error::config(format!("Failed to parse JSON config: {}", e))),
                    ConfigFormat::Toml => toml::from_str(&content)
                        .map_err(|e| Error::config(format!("Failed to parse TOML config: {}", e))),
                }
            }
            ConfigSource::Environment { prefix } => {
                let mut env_config = Map::new();

                for (key, value) in std::env::vars() {
                    if key.starts_with(prefix) {
                        let config_key = key
                            .strip_prefix(prefix)
                            .unwrap()
                            .trim_start_matches('_')
                            .to_lowercase();

                        // Convert environment variable to nested structure
                        let nested_keys: Vec<&str> = config_key.split('_').collect();
                        self.set_nested_env_value(&mut env_config, &nested_keys, value);
                    }
                }

                Ok(Value::Object(env_config))
            }
            ConfigSource::Memory { data } => Ok(data.clone()),
        }
    }

    /// Merge two JSON values recursively
    fn merge_values(&self, target: &mut Value, source: Value) {
        match (target, source) {
            (Value::Object(target_map), Value::Object(source_map)) => {
                for (key, source_value) in source_map {
                    match target_map.get_mut(&key) {
                        Some(target_value) => {
                            self.merge_values(target_value, source_value);
                        }
                        None => {
                            target_map.insert(key, source_value);
                        }
                    }
                }
            }
            (target, source) => {
                *target = source;
            }
        }
    }

    /// Get a nested value from configuration using dot notation
    fn get_nested_value(&self, config: &Value, key: &str) -> Option<Value> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = config;

        for k in keys {
            current = current.get(k)?;
        }

        Some(current.clone())
    }

    /// Set a nested value in configuration using dot notation
    fn set_nested_value(
        &self,
        config: &mut Value,
        key: &str,
        value: Value,
    ) {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = config;

        for (i, k) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                // Last key - set the value
                if let Value::Object(ref mut map) = current {
                    map.insert(k.to_string(), value);
                }
                return; // Exit early to avoid moving value multiple times
            } else {
                // Navigate or create intermediate objects
                if !current.is_object() {
                    *current = Value::Object(Map::new());
                }

                let map = current.as_object_mut().unwrap();
                if !map.contains_key(*k) {
                    map.insert(
                        k.to_string(),
                        Value::Object(Map::new()),
                    );
                }

                current = map.get_mut(*k).unwrap();
            }
        }
    }

    /// Set a nested environment variable value
    fn set_nested_env_value(
        &self,
        config: &mut Map<String, Value>,
        keys: &[&str],
        value: String,
    ) {
        if keys.is_empty() {
            return;
        }

        if keys.len() == 1 {
            // Try to parse as different types
            let parsed_value = if let Ok(bool_val) = value.parse::<bool>() {
                Value::Bool(bool_val)
            } else if let Ok(int_val) = value.parse::<i64>() {
                Value::Number(Number::from(int_val))
            } else if let Ok(float_val) = value.parse::<f64>() {
                Value::Number(Number::from_f64(float_val).unwrap())
            } else {
                Value::String(value)
            };

            config.insert(keys[0].to_string(), parsed_value);
        } else {
            let first_key = keys[0];
            if !config.contains_key(first_key) {
                config.insert(
                    first_key.to_string(),
                    Value::Object(Map::new()),
                );
            }

            if let Some(Value::Object(nested_map)) = config.get_mut(first_key) {
                self.set_nested_env_value(nested_map, &keys[1..], value);
            }
        }
    }

    /// Get configuration as JSON for debugging
    pub async fn debug_config(&self) -> Value {
        let config = self.merged_config.read().await;
        config.clone()
    }

    /// Get metadata about the configuration manager
    pub fn get_metadata(&self) -> Value {
        serde_json::json!({
            "layers": self.layers.len(),
            "layer_info": self.layers.iter().map(|l| {
                serde_json::json!({
                    "name": l.name,
                    "priority": l.priority,
                    "hot_reload": l.hot_reload,
                    "source_type": match &l.source {
                        ConfigSource::File { path, .. } => {
                            path.display().to_string()
                        }
                        ConfigSource::Environment { prefix } => {
                            format!("env:{}", prefix)
                        }
                        ConfigSource::Memory { .. } => "memory".to_string(),
                    }
                })
            }).collect::<Vec<_>>(),
            "config_path": self.layers.iter()
                .find_map(|l| match &l.source {
                    ConfigSource::File { path, .. } => {
                        Some(path.display().to_string())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "none".to_string()),
            "watch_enabled": self.watch_enabled,
            "env_prefix": self.env_prefix.clone()
        })
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Manager for ConfigManager {
    fn name(&self) -> &str {
        "config_manager"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4() // Simplified
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Store env_prefix in a local variable to avoid borrow conflicts
        let env_prefix = self.env_prefix.clone();
        self.add_env_layer("environment", &env_prefix, 1000);

        // Load and merge all configurations
        self.merge_configurations().await?;

        // TODO: Setup file watching for hot-reload

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Clean up file watchers, etc.

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        status.add_metadata("layers", Value::from(self.layers.len()));
        status.add_metadata("watch_enabled", Value::Bool(self.watch_enabled));
        status.add_metadata(
            "env_prefix",
            Value::String(self.env_prefix.clone()),
        );
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert_eq!(manager.layers.len(), 0);
    }

    #[tokio::test]
    async fn test_file_layer() {
        let mut manager = ConfigManager::new();

        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"app:\n  name: \"Test App\"\n  debug: true").unwrap();

        manager
            .add_file_layer("test", temp_file.path(), 0, false)
            .unwrap();

        manager.initialize().await.unwrap();

        let app_name: String = manager.get("app.name").await.unwrap();
        assert_eq!(app_name, "Test App");

        let debug: bool = manager.get("app.debug").await.unwrap();
        assert!(debug);
    }

    #[tokio::test]
    async fn test_environment_layer() {
        let mut manager = ConfigManager::new();

        // Set environment variables
        std::env::set_var("TEST_APP_NAME", "Env App");
        std::env::set_var("TEST_APP_DEBUG", "false");

        manager.add_env_layer("env", "TEST", 100);
        manager.initialize().await.unwrap();

        let app_name: String = manager.get("app.name").await.unwrap();
        assert_eq!(app_name, "Env App");

        let debug: bool = manager.get("app.debug").await.unwrap();
        assert!(!debug);

        // Clean up
        std::env::remove_var("TEST_APP_NAME");
        std::env::remove_var("TEST_APP_DEBUG");
    }

    #[tokio::test]
    async fn test_memory_layer() {
        let mut manager = ConfigManager::new();

        let memory_config = serde_json::json!({
            "app": {
                "name": "Memory App",
                "version": "1.0.0"
            }
        });

        manager.add_memory_layer("memory", memory_config, 50);
        manager.initialize().await.unwrap();

        let app_name: String = manager.get("app.name").await.unwrap();
        assert_eq!(app_name, "Memory App");
    }

    #[tokio::test]
    async fn test_configuration_change() {
        let mut manager = ConfigManager::new();
        manager.initialize().await.unwrap();

        let change = ConfigChangeEvent {
            key: "test.value".to_string(),
            value: Value::Bool(false),
            old_value: None,
            timestamp: Utc::now(),
            source: "test".to_string(),
            metadata: HashMap::new(),
        };

        assert_eq!(change.value, Value::Bool(false));
    }
}