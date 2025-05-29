// src/config/mod.rs

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
use crate::utils::Time;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::event::{Event, EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus};
use crate::types::Metadata;

pub mod tiered;
pub use tiered::{ConfigurationTier, TieredConfigManager, MemoryConfigStore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    pub key: String,
    pub value: Value,
    pub old_value: Option<Value>,
    pub timestamp: DateTime<Utc>,
    pub source: String,
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

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsSchema {
    pub version: String,
    pub schema: Value,
    pub defaults: Value,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub key: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error for '{}': {}", self.key, self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

impl ConfigFormat {
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConfigSource {
    File { path: PathBuf, format: ConfigFormat },
    Environment { prefix: String },
    Memory { data: Value },
}

#[derive(Debug, Clone)]
pub struct ConfigLayer {
    pub name: String,
    pub source: ConfigSource,
    pub priority: u32,
    pub hot_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub logging: LoggingConfig,
    pub event_bus: EventBusConfig,
    pub files: FileConfig,
    pub tasks: TaskConfig,
    pub concurrency: ConcurrencyConfig,
    pub plugins: PluginConfig,
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub version: String,
    pub description: String,
    pub environment: String,
    pub debug: bool,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub log_dir: PathBuf,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub console: ConsoleLogConfig,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLogConfig {
    pub enabled: bool,
    pub level: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLogConfig {
    pub path: PathBuf,
    pub max_size: u64,
    pub max_files: u32,
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

/// Get default CPU count for the platform
fn get_default_cpu_count() -> usize {
    #[cfg(not(target_arch = "wasm32"))]
    {
        num_cpus::get()
    }
    #[cfg(target_arch = "wasm32")]
    {
        1 // Default to 1 for WASM
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfig {
    pub worker_count: usize,
    pub queue_size: usize,
    pub publish_timeout_ms: u64,
    pub enable_persistence: bool,
    pub enable_metrics: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            worker_count: get_default_cpu_count().max(1) * 2,
            queue_size: 10000,
            publish_timeout_ms: 5000,
            enable_persistence: false,
            enable_metrics: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub default_permissions: u32,
    pub max_file_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_dir: Option<PathBuf>,
    pub operation_timeout_secs: u64,
    pub enable_watching: bool,
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
        use std::fs;
        use uuid::Uuid;

        let mut temp = std::env::temp_dir();
        temp.push(format!("qorzen_{}", Uuid::new_v4()));

        if let Err(e) = fs::create_dir_all(&temp) {
            eprintln!("Failed to create temp dir: {}", e);
            return None;
        }

        Some(temp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub max_concurrent: usize,
    pub default_timeout_ms: u64,
    pub keep_completed: bool,
    pub progress_update_interval_ms: u64,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            max_concurrent: get_default_cpu_count().max(1) * 2,
            default_timeout_ms: 300_000, // 5 minutes
            keep_completed: true,
            progress_update_interval_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    pub thread_pool_size: usize,
    pub io_thread_pool_size: usize,
    pub blocking_thread_pool_size: usize,
    pub max_queue_size: usize,
    pub thread_keep_alive_secs: u64,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        let cpu_count = get_default_cpu_count();
        Self {
            thread_pool_size: cpu_count,
            io_thread_pool_size: cpu_count * 2,
            blocking_thread_pool_size: cpu_count.max(4),
            max_queue_size: 1000,
            thread_keep_alive_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin_dir: PathBuf,
    pub auto_load: bool,
    pub load_timeout_secs: u64,
    pub max_plugins: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub query_timeout_secs: u64,
    pub enable_pooling: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub port: u16,
    pub enable_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub request_timeout_secs: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiration_secs: u64,
    pub api_key: Option<String>,
    pub enable_rate_limiting: bool,
    pub rate_limit_rpm: u64,
    pub enable_cors: bool,
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

pub struct ConfigManager {
    state: ManagedState,
    layers: Vec<ConfigLayer>,
    merged_config: Arc<RwLock<Value>>,
    change_notifier: broadcast::Sender<ConfigChangeEvent>,
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
    pub fn new() -> Self {
        let (change_notifier, _) = broadcast::channel(100);

        Self {
            state: ManagedState::new(Uuid::new_v4(), "config_manager"),
            layers: Vec::new(),
            merged_config: Arc::new(RwLock::new(Value::Object(Map::new()))),
            change_notifier,
            watch_enabled: true,
            env_prefix: "QORZEN".to_string(),
            event_bus: None,
        }
    }

    pub fn with_config_file<P: AsRef<Path>>(config_path: P) -> Self {
        let mut manager = Self::new();
        let _ = manager.add_file_layer("default", config_path, 0, true);
        manager
    }

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

    pub fn add_memory_layer(&mut self, name: impl Into<String>, data: Value, priority: u32) {
        let layer = ConfigLayer {
            name: name.into(),
            source: ConfigSource::Memory { data },
            priority,
            hot_reload: false,
        };

        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority);
    }

    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

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
            timestamp: Time::now(),
            source: "config_manager".to_string(),
            metadata: HashMap::new(),
        };

        if let Some(event_bus) = &self.event_bus {
            let _ = event_bus.publish(change_event.clone()).await;
        }

        let _ = self.change_notifier.send(change_event);

        Ok(())
    }

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

    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_notifier.subscribe()
    }

    pub async fn reload(&self) -> Result<()> {
        self.merge_configurations().await?;

        // Publish reload event
        let reload_event = ConfigChangeEvent {
            key: "_reload".to_string(),
            value: Value::String("reloaded".to_string()),
            old_value: None,
            timestamp: Time::now(),
            source: "config_manager".to_string(),
            metadata: HashMap::new(),
        };

        if let Some(event_bus) = &self.event_bus {
            let _ = event_bus.publish(reload_event.clone()).await;
        }

        let _ = self.change_notifier.send(reload_event);

        Ok(())
    }

    pub async fn validate(&self) -> Result<Vec<ValidationError>> {
        let _config = self.merged_config.read().await;
        let errors = Vec::new();

        // Add validation logic here
        // This is a simplified example
        // In practice, you'd implement comprehensive validation

        Ok(errors)
    }

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

    async fn load_layer_config(&self, layer: &ConfigLayer) -> Result<Value> {
        match &layer.source {
            #[cfg(not(target_arch = "wasm32"))]
            ConfigSource::File { path, format } => {
                {
                    let content = std::fs::read_to_string(path)
                        .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;

                    match format {
                        ConfigFormat::Json => serde_json::from_str(&content)
                            .map_err(|e| Error::config(format!("Failed to parse JSON config: {}", e))),
                        ConfigFormat::Yaml => serde_yaml::from_str(&content)
                            .map_err(|e| Error::config(format!("Failed to parse YAML config: {}", e))),
                        ConfigFormat::Toml => toml::from_str(&content)
                            .map_err(|e| Error::config(format!("Failed to parse TOML config: {}", e))),
                    }
                }
            },
            
            #[cfg(target_arch = "wasm32")]
            ConfigSource::File { .. } => {
                Err(Error::config("File loading not supported in web platform"))
            },

            #[cfg(not(target_arch = "wasm32"))]
            ConfigSource::Environment { prefix } => {
                let mut env_config = serde_json::Map::new();

                for (key, value) in std::env::vars() {
                    if key.starts_with(prefix) {
                        let config_key = key
                            .strip_prefix(prefix)
                            .unwrap()
                            .trim_start_matches('_')
                            .to_lowercase();
                        let nested_keys: Vec<&str> = config_key.split('_').collect();
                        self.set_nested_env_value(&mut env_config, &nested_keys, value);
                    }
                }

                Ok(Value::Object(env_config))
            },

            #[cfg(target_arch = "wasm32")]
            ConfigSource::Environment { .. } => {
                Ok(Value::Object(serde_json::Map::new()))
            },
            
            ConfigSource::Memory { data } => Ok(data.clone()),
        }
    }

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

    fn get_nested_value(&self, config: &Value, key: &str) -> Option<Value> {
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = config;

        for k in keys {
            current = current.get(k)?;
        }

        Some(current.clone())
    }

    fn set_nested_value(&self, config: &mut Value, key: &str, value: Value) {
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
                    map.insert(k.to_string(), Value::Object(Map::new()));
                }

                current = map.get_mut(*k).unwrap();
            }
        }
    }

    fn set_nested_env_value(&self, config: &mut Map<String, Value>, keys: &[&str], value: String) {
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
                config.insert(first_key.to_string(), Value::Object(Map::new()));
            }

            if let Some(Value::Object(nested_map)) = config.get_mut(first_key) {
                self.set_nested_env_value(nested_map, &keys[1..], value);
            }
        }
    }

    pub async fn debug_config(&self) -> Value {
        let config = self.merged_config.read().await;
        config.clone()
    }

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

/// Conditional Manager implementation for ConfigManager
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Manager for ConfigManager {
    fn name(&self) -> &str {
        "config_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
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
        status.add_metadata("env_prefix", Value::String(self.env_prefix.clone()));
        status
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Manager for ConfigManager {
    fn name(&self) -> &str {
        "config_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
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
        status.add_metadata("env_prefix", Value::String(self.env_prefix.clone()));
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
        temp_file
            .write_all(b"app:\n  name: \"Test App\"\n  debug: true")
            .unwrap();

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
            timestamp: Time::now(),
            source: "test".to_string(),
            metadata: HashMap::new(),
        };

        assert_eq!(change.value, Value::Bool(false));
    }
}