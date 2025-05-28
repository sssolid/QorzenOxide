// src/platform/storage.rs

use std::sync::Arc;

use async_trait::async_trait;
use crate::error::Result;

#[cfg(not(target_arch = "wasm32"))]
pub type DynStorage = dyn StorageProvider + Send + Sync;

#[cfg(target_arch = "wasm32")]
pub type DynStorage = dyn StorageProvider + Sync;

pub type StorageArc = Arc<DynStorage>;

/// Storage operations (key-value)
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait StorageProvider: StorageBounds {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    async fn clear(&self) -> Result<()>;
}

#[cfg(not(target_arch = "wasm32"))]
pub trait StorageBounds: Send + Sync {}

#[cfg(target_arch = "wasm32")]
pub trait StorageBounds: Sync {}