// src/platform/filesystem.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: chrono::DateTime<chrono::Utc>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub is_directory: bool,
    pub is_readonly: bool,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub accessed: Option<chrono::DateTime<chrono::Utc>>,
}

/// File system operations
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait FileSystemProvider: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;
    async fn delete_file(&self, path: &str) -> Result<()>;
    async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>>;
    async fn create_directory(&self, path: &str) -> Result<()>;
    async fn file_exists(&self, path: &str) -> bool;
    async fn get_metadata(&self, path: &str) -> Result<FileMetadata>;
}

// #[cfg(target_arch = "wasm32")]
// #[async_trait(?Send)]
// pub trait FileSystemProvider: Sync {
//     async fn read_file(&self, path: &str) -> Result<Vec<u8>>;
//     async fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;
//     async fn delete_file(&self, path: &str) -> Result<()>;
//     async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>>;
//     async fn create_directory(&self, path: &str) -> Result<()>;
//     async fn file_exists(&self, path: &str) -> bool;
//     async fn get_metadata(&self, path: &str) -> Result<FileMetadata>;
// }