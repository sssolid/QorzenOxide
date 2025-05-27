// src/platform/database.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

/// Database query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}

/// Database row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub columns: HashMap<String, serde_json::Value>,
}

/// Database transaction
pub struct Transaction {
    // Implementation will be platform-specific
}

/// Database migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub version: u32,
    pub description: String,
    pub up_sql: String,
    pub down_sql: String,
}

/// Database operations - made dyn compatible by removing generic transaction method
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    async fn execute(&self, query: &str, params: &[serde_json::Value]) -> Result<QueryResult>;
    async fn query(&self, query: &str, params: &[serde_json::Value]) -> Result<Vec<Row>>;
    async fn migrate(&self, migrations: &[Migration]) -> Result<()>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait DatabaseProvider: Sync {
    async fn execute(&self, query: &str, params: &[serde_json::Value]) -> Result<QueryResult>;
    async fn query(&self, query: &str, params: &[serde_json::Value]) -> Result<Vec<Row>>;
    async fn migrate(&self, migrations: &[Migration]) -> Result<()>;
}