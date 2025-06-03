// src/platform/storage.rs

use crate::error::Result;
use std::sync::Arc;

/// Unified storage provider trait bounds
pub trait StorageBounds: Send + Sync + std::fmt::Debug {}

pub type DynStorage = dyn StorageProvider;
pub type StorageArc = Arc<DynStorage>;

/// Storage operations - unified across platforms
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait StorageProvider: StorageBounds {
    /// Get a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set a value by key
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Delete a value by key
    async fn delete(&self, key: &str) -> Result<()>;

    /// List all keys with a given prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;

    /// Clear all storage
    async fn clear(&self) -> Result<()>;
}