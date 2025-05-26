// src/platform/network.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

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

/// Network operations
#[async_trait]
pub trait NetworkProvider: Send + Sync {
    async fn request(&self, request: NetworkRequest) -> Result<NetworkResponse>;
    async fn upload_file(&self, url: &str, file_data: &[u8]) -> Result<NetworkResponse>;
    async fn download_file(&self, url: &str) -> Result<Vec<u8>>;
}
