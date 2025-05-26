// src/platform/storage.rs

use async_trait::async_trait;

use crate::error::Result;

/// Storage operations (key-value)
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn clear(&self) -> Result<()>;
}
