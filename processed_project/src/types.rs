use serde_json::Value;use std::collections::HashMap;use uuid::Uuid;pub type Id=Uuid;pub type Metadata=HashMap<String,Value>;pub type CorrelationId=Uuid;
