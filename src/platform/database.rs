// src/platform/database.rs

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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

#[cfg(not(target_arch = "wasm32"))]
pub type DynDatabase = dyn DatabaseProvider + Send + Sync;

#[cfg(target_arch = "wasm32")]
pub type DynDatabase = dyn DatabaseProvider + Sync;

pub type DatabaseArc = Arc<DynDatabase>;

/// Database operations - made dyn compatible by removing generic transaction method
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait DatabaseProvider: DatabaseBounds {
    async fn execute(&self, query: &str, params: &[serde_json::Value]) -> Result<QueryResult>;
    async fn query(&self, query: &str, params: &[serde_json::Value]) -> Result<Vec<Row>>;
    async fn migrate(&self, migrations: &[Migration]) -> Result<()>;
}

#[cfg(not(target_arch = "wasm32"))]
pub trait DatabaseBounds: Send + Sync {}

#[cfg(target_arch = "wasm32")]
pub trait DatabaseBounds: Sync {}
