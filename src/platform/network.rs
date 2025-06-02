// src/platform/network.rs

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Network request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: Option<u64>,
}

/// Network response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
pub type DynNetwork = dyn NetworkProvider + Send + Sync;

#[cfg(target_arch = "wasm32")]
pub type DynNetwork = dyn NetworkProvider + Sync;

pub type NetworkArc = Arc<DynNetwork>;

/// Network operations
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait NetworkProvider: NetworkBounds {
    async fn request(&self, request: NetworkRequest) -> Result<NetworkResponse>;
    async fn upload_file(&self, url: &str, file_data: &[u8]) -> Result<NetworkResponse>;
    async fn download_file(&self, url: &str) -> Result<Vec<u8>>;
}

#[cfg(not(target_arch = "wasm32"))]
pub trait NetworkBounds: Send + Sync {}

#[cfg(target_arch = "wasm32")]
pub trait NetworkBounds: Sync {}
