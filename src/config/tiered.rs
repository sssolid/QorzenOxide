// src/config/tiered.rs - Enhanced tiered configuration system

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::utils::Time;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};

/// Configuration tiers in order of precedence (lowest to highest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigurationTier {
    System,  // System defaults (built-in)
    Global,  // Organization-wide (from server)
    User,    // User preferences (synced)
    Local,   // Device-specific overrides
    Runtime, // Temporary runtime changes
}

impl ConfigurationTier {
    /// Returns all tiers in precedence order
    pub fn all_tiers() -> Vec<Self> {
        vec![
            Self::System,
            Self::Global,
            Self::User,
            Self::Local,
            Self::Runtime,
        ]
    }

    /// Returns the precedence value (higher = more important)
    pub fn precedence(&self) -> u8 {
        match self {
            Self::System => 0,
            Self::Global => 1,
            Self::User => 2,
            Self::Local => 3,
            Self::Runtime => 4,
        }
    }
}

/// Configuration store trait - conditional Send requirement
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait ConfigStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn set(&self, key: &str, value: Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn watch(&self, key: &str) -> Result<ConfigWatcher>;
    fn tier(&self) -> ConfigurationTier;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait ConfigStore: Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>>;
    async fn set(&self, key: &str, value: Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn watch(&self, key: &str) -> Result<ConfigWatcher>;
    fn tier(&self) -> ConfigurationTier;
}

/// Configuration watcher for change notifications
pub struct ConfigWatcher {
    receiver: broadcast::Receiver<ConfigChangeEvent>,
}

impl ConfigWatcher {
    pub fn new(receiver: broadcast::Receiver<ConfigChangeEvent>) -> Self {
        Self { receiver }
    }

    pub async fn recv(&mut self) -> Result<ConfigChangeEvent> {
        self.receiver
            .recv()
            .await
            .map_err(|_| Error::config("Config watch channel closed"))
    }
}

/// Configuration change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    pub key: String,
    pub value: Option<Value>,
    pub old_value: Option<Value>,
    pub tier: ConfigurationTier,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub correlation_id: Option<Uuid>,
}

/// Configuration merger handles merging values from multiple tiers
pub struct ConfigMerger {
    // Strategies for merging different value types
}

impl Default for ConfigMerger {
    fn default() -> Self {
        Self::new()
    }
}

fn merge_values(base: Value, override_value: Value) -> Value {
    match (base, override_value) {
        // If override is null, keep base
        (base, Value::Null) => base,

        // If base is null, use override
        (Value::Null, override_val) => override_val,

        // Merge objects recursively
        (Value::Object(mut base_obj), Value::Object(override_obj)) => {
            for (key, value) in override_obj {
                match base_obj.get(&key) {
                    Some(base_value) => {
                        base_obj.insert(key, merge_values(base_value.clone(), value));
                    }
                    None => {
                        base_obj.insert(key, value);
                    }
                }
            }
            Value::Object(base_obj)
        }

        // For arrays, override completely (could be made configurable)
        (_, Value::Array(override_arr)) => Value::Array(override_arr),

        // For primitive values, override completely
        (_, override_val) => override_val,
    }
}

impl ConfigMerger {
    pub fn new() -> Self {
        Self {}
    }

    /// Merges configuration values from multiple tiers
    pub fn merge(&self, values: Vec<(ConfigurationTier, Value)>) -> Value {
        if values.is_empty() {
            return Value::Null;
        }

        // Sort by precedence (lowest first)
        let mut sorted_values = values;
        sorted_values.sort_by_key(|(tier, _)| tier.precedence());

        // Start with the lowest precedence value
        let mut result = sorted_values[0].1.clone();

        // Merge higher precedence values
        for (_, value) in sorted_values.into_iter().skip(1) {
            result = merge_values(result, value);
        }

        result
    }
}

/// Configuration change detector
pub struct ConfigChangeDetector {
    previous_values: HashMap<String, Value>,
    change_sender: broadcast::Sender<ConfigChangeEvent>,
}

impl Default for ConfigChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigChangeDetector {
    pub fn new() -> Self {
        let (change_sender, _) = broadcast::channel(1000);
        Self {
            previous_values: HashMap::new(),
            change_sender,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_sender.subscribe()
    }

    pub fn detect_change(
        &mut self,
        key: &str,
        new_value: &Value,
        tier: ConfigurationTier,
        source: &str,
    ) {
        let old_value = self.previous_values.get(key).cloned();

        if old_value.as_ref() != Some(new_value) {
            let change_event = ConfigChangeEvent {
                key: key.to_string(),
                value: Some(new_value.clone()),
                old_value,
                tier,
                timestamp: Time::now(),
                source: source.to_string(),
                correlation_id: None,
            };

            let _ = self.change_sender.send(change_event);
            self.previous_values
                .insert(key.to_string(), new_value.clone());
        }
    }
}

/// Configuration synchronization manager
pub struct ConfigSyncManager {
    #[allow(dead_code)]
    sync_interval: Duration,
    last_sync: RwLock<DateTime<Utc>>,
    sync_enabled: bool,
}

impl ConfigSyncManager {
    pub fn new(sync_interval: Duration) -> Self {
        Self {
            sync_interval,
            last_sync: RwLock::new(Time::now()),
            sync_enabled: true,
        }
    }

    pub async fn sync_with_server(&self, _store: &dyn ConfigStore) -> Result<()> {
        if !self.sync_enabled {
            return Ok(());
        }

        // Implementation would sync with remote server
        *self.last_sync.write().await = Time::now();
        Ok(())
    }

    pub async fn last_sync_time(&self) -> DateTime<Utc> {
        *self.last_sync.read().await
    }
}

/// Validation rule set for configuration values
pub struct ValidationRuleSet {
    rules: HashMap<String, Vec<ValidationRule>>,
}

/// Configuration validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    Required,
    Type(String),
    Range { min: f64, max: f64 },
    Length { min: usize, max: usize },
    Pattern(String),
    Enum(Vec<String>),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

impl Default for ValidationRuleType {
    fn default() -> Self {
        Self::Required
    }
}

impl Default for ValidationRuleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationRuleSet {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, key: String, rule: ValidationRule) {
        self.rules.entry(key).or_default().push(rule);
    }

    pub fn validate(&self, key: &str, value: &Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if let Some(rules) = self.rules.get(key) {
            for rule in rules {
                if let Some(error) = self.validate_rule(key, value, rule) {
                    errors.push(error);
                }
            }
        }

        errors
    }

    fn validate_rule(
        &self,
        key: &str,
        value: &Value,
        rule: &ValidationRule,
    ) -> Option<ValidationError> {
        let is_valid = match &rule.rule_type {
            ValidationRuleType::Required => !value.is_null(),
            ValidationRuleType::Type(expected_type) => self.check_type(value, expected_type),
            ValidationRuleType::Range { min, max } => self.check_range(value, *min, *max),
            ValidationRuleType::Length { min, max } => self.check_length(value, *min, *max),
            ValidationRuleType::Pattern(pattern) => self.check_pattern(value, pattern),
            ValidationRuleType::Enum(options) => self.check_enum(value, options),
            ValidationRuleType::Custom(_) => true, // Custom validation would be implemented
        };

        if !is_valid {
            Some(ValidationError {
                key: key.to_string(),
                message: rule.message.clone(),
                severity: rule.severity.clone(),
                rule_type: rule.rule_type.clone(),
            })
        } else {
            None
        }
    }

    fn check_type(&self, value: &Value, expected_type: &str) -> bool {
        match expected_type {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => true,
        }
    }

    fn check_range(&self, value: &Value, min: f64, max: f64) -> bool {
        if let Some(num) = value.as_f64() {
            num >= min && num <= max
        } else {
            false
        }
    }

    fn check_length(&self, value: &Value, min: usize, max: usize) -> bool {
        let length = if let Some(s) = value.as_str() {
            s.len()
        } else if let Some(arr) = value.as_array() {
            arr.len()
        } else if let Some(obj) = value.as_object() {
            obj.len()
        } else {
            return false;
        };

        length >= min && length <= max
    }

    fn check_pattern(&self, value: &Value, _pattern: &str) -> bool {
        // Would implement regex pattern matching
        value.is_string()
    }

    fn check_enum(&self, value: &Value, options: &[String]) -> bool {
        if let Some(s) = value.as_str() {
            options.contains(&s.to_string())
        } else {
            false
        }
    }
}

/// Configuration validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub key: String,
    pub message: String,
    pub severity: ValidationSeverity,
    pub rule_type: ValidationRuleType,
}

/// Main tiered configuration manager
pub struct TieredConfigManager {
    state: ManagedState,
    stores: HashMap<ConfigurationTier, Box<dyn ConfigStore>>,
    merger: ConfigMerger,
    sync_manager: Option<ConfigSyncManager>,
    change_detector: ConfigChangeDetector,
    validation_rules: ValidationRuleSet,
    cache: Arc<RwLock<HashMap<String, Value>>>,
    cache_ttl: Duration,
}

impl std::fmt::Debug for TieredConfigManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TieredConfigManager")
            .field("stores", &self.stores.len())
            .field("cache_ttl", &self.cache_ttl)
            .finish()
    }
}

impl Default for TieredConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TieredConfigManager {
    /// Creates a new tiered configuration manager
    pub fn new() -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "tiered_config_manager"),
            stores: HashMap::new(),
            merger: ConfigMerger::new(),
            sync_manager: Some(ConfigSyncManager::new(Duration::from_secs(300))), // 5 minutes
            change_detector: ConfigChangeDetector::new(),
            validation_rules: ValidationRuleSet::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60),
        }
    }

    /// Adds a configuration store for a specific tier
    pub fn add_store(&mut self, tier: ConfigurationTier, store: Box<dyn ConfigStore>) {
        self.stores.insert(tier, store);
    }

    /// Gets a configuration value by merging across all tiers
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Check cache first
        if let Some(cached_value) = self.cache.read().await.get(key) {
            return Ok(Some(serde_json::from_value(cached_value.clone()).map_err(
                |e| Error::config(format!("Deserialization failed: {e}")),
            )?));
        }

        // Collect values from all tiers
        let mut tier_values = Vec::new();

        for tier in ConfigurationTier::all_tiers() {
            if let Some(store) = self.stores.get(&tier) {
                if let Some(value) = store.get(key).await? {
                    tier_values.push((tier, value));
                }
            }
        }

        if tier_values.is_empty() {
            return Ok(None);
        }

        // Merge values
        let merged_value = self.merger.merge(tier_values);

        // Cache the result
        self.cache
            .write()
            .await
            .insert(key.to_string(), merged_value.clone());

        // Deserialize and return
        let result: T = serde_json::from_value(merged_value)
            .map_err(|_e| Error::config("Failed to deserialize merged value"))?;
        Ok(Some(result))
    }

    /// Sets a configuration value in a specific tier
    pub async fn set(&mut self, key: &str, value: Value, tier: ConfigurationTier) -> Result<()> {
        // Validate the value
        let validation_errors = self.validation_rules.validate(key, &value);
        if validation_errors
            .iter()
            .any(|e| matches!(e.severity, ValidationSeverity::Error))
        {
            return Err(Error::config(format!(
                "Validation failed for key '{}': {:?}",
                key, validation_errors
            )));
        }

        // Get the store for this tier
        let store = self
            .stores
            .get(&tier)
            .ok_or_else(|| Error::config(format!("No store configured for tier {:?}", tier)))?;

        // Set the value
        store.set(key, value.clone()).await?;

        // Invalidate cache
        self.cache.write().await.remove(key);

        // Detect and broadcast change
        self.change_detector
            .detect_change(key, &value, tier, "tiered_config_manager");

        Ok(())
    }

    /// Deletes a configuration value from a specific tier
    pub async fn delete(&self, key: &str, tier: ConfigurationTier) -> Result<()> {
        let store = self
            .stores
            .get(&tier)
            .ok_or_else(|| Error::config(format!("No store configured for tier {:?}", tier)))?;

        store.delete(key).await?;

        // Invalidate cache
        self.cache.write().await.remove(key);

        Ok(())
    }

    /// Lists all keys with a given prefix
    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut all_keys = std::collections::HashSet::new();

        for store in self.stores.values() {
            let keys = store.list_keys(prefix).await?;
            all_keys.extend(keys);
        }

        Ok(all_keys.into_iter().collect())
    }

    /// Subscribes to configuration changes
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_detector.subscribe()
    }

    /// Adds a validation rule
    pub fn add_validation_rule(&mut self, key: String, rule: ValidationRule) {
        self.validation_rules.add_rule(key, rule);
    }

    /// Validates all configuration values
    pub async fn validate_all(&self) -> Result<Vec<ValidationError>> {
        let mut all_errors = Vec::new();

        for store in self.stores.values() {
            let keys = store.list_keys("").await?;
            for key in keys {
                if let Some(value) = store.get(&key).await? {
                    let errors = self.validation_rules.validate(&key, &value);
                    all_errors.extend(errors);
                }
            }
        }

        Ok(all_errors)
    }

    /// Clears the configuration cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Syncs configuration with remote server
    pub async fn sync(&self) -> Result<()> {
        if let Some(sync_manager) = &self.sync_manager {
            for store in self.stores.values() {
                sync_manager.sync_with_server(store.as_ref()).await?;
            }
        }
        Ok(())
    }
}

/// Conditional Manager implementation for TieredConfigManager
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Manager for TieredConfigManager {
    fn name(&self) -> &str {
        "tiered_config_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;
        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;
        let _ = self.sync().await;
        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        status.add_metadata("stores_count", Value::from(self.stores.len()));
        status.add_metadata("cache_size", Value::from(self.cache.read().await.len()));
        status.add_metadata("cache_ttl_seconds", Value::from(self.cache_ttl.as_secs()));
        if let Some(sync_manager) = &self.sync_manager {
            status.add_metadata(
                "last_sync",
                Value::String(sync_manager.last_sync_time().await.to_rfc3339()),
            );
        }
        status
    }

    fn supports_runtime_reload(&self) -> bool {
        true
    }

    async fn reload_config(&mut self, config: serde_json::Value) -> Result<()> {
        if let serde_json::Value::Object(config_obj) = config {
            for (key, value) in config_obj {
                self.set(&key, value, ConfigurationTier::Runtime).await?;
            }
        }
        Ok(())
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: true,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec!["config.read".to_string(), "config.write".to_string()],
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Manager for TieredConfigManager {
    fn name(&self) -> &str {
        "tiered_config_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;
        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;
        let _ = self.sync().await;
        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        status.add_metadata("stores_count", Value::from(self.stores.len()));
        status.add_metadata("cache_size", Value::from(self.cache.read().await.len()));
        status.add_metadata("cache_ttl_seconds", Value::from(self.cache_ttl.as_secs()));
        if let Some(sync_manager) = &self.sync_manager {
            status.add_metadata(
                "last_sync",
                Value::String(sync_manager.last_sync_time().await.to_rfc3339()),
            );
        }
        status
    }

    fn supports_runtime_reload(&self) -> bool {
        true
    }

    async fn reload_config(&mut self, config: serde_json::Value) -> Result<()> {
        if let serde_json::Value::Object(config_obj) = config {
            for (key, value) in config_obj {
                self.set(&key, value, ConfigurationTier::Runtime).await?;
            }
        }
        Ok(())
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: true,
            requires_network: true,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec!["config.read".to_string(), "config.write".to_string()],
        }
    }
}

/// Memory-based configuration store for runtime values
pub struct MemoryConfigStore {
    tier: ConfigurationTier,
    data: Arc<RwLock<HashMap<String, Value>>>,
    change_sender: broadcast::Sender<ConfigChangeEvent>,
}

impl MemoryConfigStore {
    pub fn new(tier: ConfigurationTier) -> Self {
        let (change_sender, _) = broadcast::channel(100);
        Self {
            tier,
            data: Arc::new(RwLock::new(HashMap::new())),
            change_sender,
        }
    }
}

/// Conditional ConfigStore implementation for MemoryConfigStore
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl ConfigStore for MemoryConfigStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        Ok(self.data.read().await.get(key).cloned())
    }

    async fn set(&self, key: &str, value: Value) -> Result<()> {
        let old_value = self
            .data
            .write()
            .await
            .insert(key.to_string(), value.clone());

        let change_event = ConfigChangeEvent {
            key: key.to_string(),
            value: Some(value),
            old_value,
            tier: self.tier,
            timestamp: Time::now(),
            source: "memory_store".to_string(),
            correlation_id: None,
        };

        let _ = self.change_sender.send(change_event);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.write().await.remove(key);
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let data = self.data.read().await;
        let keys: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn watch(&self, _key: &str) -> Result<ConfigWatcher> {
        Ok(ConfigWatcher::new(self.change_sender.subscribe()))
    }

    fn tier(&self) -> ConfigurationTier {
        self.tier
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl ConfigStore for MemoryConfigStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        Ok(self.data.read().await.get(key).cloned())
    }

    async fn set(&self, key: &str, value: Value) -> Result<()> {
        let old_value = self
            .data
            .write()
            .await
            .insert(key.to_string(), value.clone());

        let change_event = ConfigChangeEvent {
            key: key.to_string(),
            value: Some(value),
            old_value,
            tier: self.tier,
            timestamp: Time::now(),
            source: "memory_store".to_string(),
            correlation_id: None,
        };

        let _ = self.change_sender.send(change_event);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.write().await.remove(key);
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let data = self.data.read().await;
        let keys: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn watch(&self, _key: &str) -> Result<ConfigWatcher> {
        Ok(ConfigWatcher::new(self.change_sender.subscribe()))
    }

    fn tier(&self) -> ConfigurationTier {
        self.tier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_merging() {
        let merger = ConfigMerger::new();

        let base = serde_json::json!({
            "app": {
                "name": "Test App",
                "version": "1.0.0"
            },
            "features": {
                "feature1": true
            }
        });

        let override_val = serde_json::json!({
            "app": {
                "version": "1.1.0"
            },
            "features": {
                "feature2": true
            }
        });

        let values = vec![
            (ConfigurationTier::System, base),
            (ConfigurationTier::User, override_val),
        ];

        let merged = merger.merge(values);

        assert_eq!(merged["app"]["name"], "Test App");
        assert_eq!(merged["app"]["version"], "1.1.0");
        assert_eq!(merged["features"]["feature1"], true);
        assert_eq!(merged["features"]["feature2"], true);
    }

    #[tokio::test]
    async fn test_tiered_config_manager() {
        let mut manager = TieredConfigManager::new();

        // Add memory stores for testing
        manager.add_store(
            ConfigurationTier::System,
            Box::new(MemoryConfigStore::new(ConfigurationTier::System)),
        );
        manager.add_store(
            ConfigurationTier::User,
            Box::new(MemoryConfigStore::new(ConfigurationTier::User)),
        );

        // Set values in different tiers
        manager
            .set(
                "app.name",
                Value::String("System App".to_string()),
                ConfigurationTier::System,
            )
            .await
            .unwrap();
        manager
            .set(
                "app.name",
                Value::String("User App".to_string()),
                ConfigurationTier::User,
            )
            .await
            .unwrap();

        // Get merged value (user tier should override system)
        let app_name: Option<String> = manager.get("app.name").await.unwrap();
        assert_eq!(app_name, Some("User App".to_string()));
    }

    #[test]
    fn test_validation_rules() {
        let mut rule_set = ValidationRuleSet::new();

        rule_set.add_rule(
            "app.port".to_string(),
            ValidationRule {
                rule_type: ValidationRuleType::Range {
                    min: 1.0,
                    max: 65535.0,
                },
                message: "Port must be between 1 and 65535".to_string(),
                severity: ValidationSeverity::Error,
            },
        );

        // Valid value
        let valid_errors =
            rule_set.validate("app.port", &Value::Number(serde_json::Number::from(8080)));
        assert!(valid_errors.is_empty());

        // Invalid value
        let invalid_errors =
            rule_set.validate("app.port", &Value::Number(serde_json::Number::from(70000)));
        assert!(!invalid_errors.is_empty());
    }
}
