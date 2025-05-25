use std::collections::HashMap;
use uuid::Uuid;
use serde_json::Value;

/// Unique identifier for managers, tasks, events, etc.
pub type Id = Uuid;

/// Generic metadata container
pub type Metadata = HashMap<String, Value>;

/// Correlation ID for tracking related operations
pub type CorrelationId = Uuid;
