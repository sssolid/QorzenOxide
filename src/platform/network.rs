// src/platform/network.rs

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Network request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: Option<u64>,
}

/// Network response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// Unified network provider trait bounds
pub trait NetworkBounds: Send + Sync + std::fmt::Debug {}

pub type DynNetwork = dyn NetworkProvider;
pub type NetworkArc = Arc<DynNetwork>;

/// Network operations - unified across platforms
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait NetworkProvider: NetworkBounds {
    /// Make a network request
    async fn request(&self, request: NetworkRequest) -> Result<NetworkResponse>;

    /// Upload a file
    async fn upload_file(&self, url: &str, file_data: &[u8]) -> Result<NetworkResponse>;

    /// Download a file
    async fn download_file(&self, url: &str) -> Result<Vec<u8>>;
}