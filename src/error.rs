// src/error.rs

//! Comprehensive error handling system for Qorzen Core
//!
//! Provides structured error types with context, severity levels, and
//! integration with the logging and event systems.

use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Add to config.rs - Key fixes for config errors
impl Error {
    /// Create a config operation error
    pub fn config_operation(
        key: impl Into<String>,
        _operation: ConfigOperation,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorKind::Configuration {
                key: Some(key.into()),
                validation_errors: Vec::new(),
            },
            message,
        ).severity(ErrorSeverity::High)
    }
}

/// Result type alias for Qorzen operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error severity levels for categorizing and handling errors appropriately
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Low priority errors that don't affect operation
    Low,
    /// Medium priority errors that may affect some functionality
    Medium,
    /// High priority errors that significantly impact operation
    High,
    /// Critical errors that require immediate attention
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Specific error categories for fine-grained error handling
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    /// Configuration-related errors
    Configuration {
        /// The configuration key that caused the error
        key: Option<String>,
        /// Additional validation context
        validation_errors: Vec<String>,
    },
    /// Manager lifecycle errors
    Manager {
        /// Name of the manager that failed
        manager_name: String,
        /// Type of operation that failed
        operation: ManagerOperation,
    },
    /// Event system errors
    Event {
        /// Event type that caused the error
        event_type: Option<String>,
        /// Subscriber or publisher ID
        subscriber_id: Option<Uuid>,
        /// Type of operation that failed
        operation: EventOperation,
    },
    /// Task execution errors
    Task {
        /// Task ID
        task_id: Option<Uuid>,
        /// Task name
        task_name: Option<String>,
        /// Whether the task was cancelled
        cancelled: bool,
    },
    /// File system operation errors
    File {
        /// File path that caused the error
        path: Option<String>,
        /// Type of file operation
        operation: FileOperation,
    },
    /// Concurrency and threading errors
    Concurrency {
        /// Thread or operation identifier
        thread_id: Option<String>,
        /// Type of concurrency operation
        operation: ConcurrencyOperation,
    },
    /// Plugin-related errors
    Plugin {
        /// Plugin identifier
        plugin_id: Option<String>,
        /// Plugin name
        plugin_name: Option<String>,
    },
    /// Network and API errors
    Network {
        /// HTTP status code if applicable
        status_code: Option<u16>,
        /// API endpoint
        endpoint: Option<String>,
    },
    /// Database operation errors
    Database {
        /// SQL query or operation
        query: Option<String>,
        /// Database connection identifier
        connection_id: Option<String>,
    },
    /// Security and authentication errors
    Security {
        /// User identifier
        user_id: Option<String>,
        /// Required permission
        permission: Option<String>,
    },
    /// Validation errors
    Validation {
        /// Field that failed validation
        field: Option<String>,
        /// Validation rules that failed
        rules: Vec<String>,
    },
    /// Generic application errors
    Application,
    /// I/O related errors
    Io,
    /// Serialization/Deserialization errors
    Serialization(String),
    /// Timeout errors
    Timeout,
    /// Resource exhaustion errors
    ResourceExhausted,
}

/// Manager operation types for error context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagerOperation {
    /// Manager initialization
    Initialize,
    /// Manager shutdown
    Shutdown,
    /// Manager configuration update
    Configure,
    /// Manager pause operation
    Pause,
    /// Manager resume operation
    Resume,
    /// Manager registration
    Register,
    /// Manager unregistration
    Unregister,
    /// Generic manager operation
    Operation(String),
}

/// Configuration operation types for error context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigOperation {
    /// Configuration get operation
    Get,
    /// Configuration set operation
    Set,
    /// Configuration reload operation
    Reload,
    /// Configuration validation operation
    Validate,
}

/// Event operation types for error context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventOperation {
    /// Event publish operation
    Publish,
    /// Event subscribe operation
    Subscribe,
    /// Event unsubscribe operation
    Unsubscribe,
    /// Event processing operation
    Process,
}

/// File operation types for error context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperation {
    /// File read operation
    Read,
    /// File write operation
    Write,
    /// File delete operation
    Delete,
    /// File copy operation
    Copy,
    /// File move operation
    Move,
    /// Directory creation
    CreateDirectory,
    /// File metadata access
    Metadata,
    /// File locking
    Lock,
    /// File watching
    Watch,
    /// File compression
    Compress,
    /// File decompression
    Decompress,
}

/// Concurrency operation types for error context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcurrencyOperation {
    /// Thread pool operation
    ThreadPool,
    /// Task spawning
    Spawn,
    /// Synchronization operation
    Sync,
    /// Channel operation
    Channel,
    /// Lock acquisition
    Lock,
}

/// Main error type for Qorzen Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    /// Unique error identifier
    pub id: Uuid,
    /// Error kind/category
    pub kind: ErrorKind,
    /// Human-readable error message
    pub message: String,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Source component that generated the error
    pub source: String,
    /// Plugin ID if error originated from a plugin
    pub plugin_id: Option<String>,
    /// Correlation ID for tracking related operations
    pub correlation_id: Option<Uuid>,
    /// When the error occurred
    pub timestamp: DateTime<Utc>,
    /// Additional context and metadata
    pub metadata: crate::types::Metadata,
    /// Stack trace if available
    pub backtrace: Option<String>,
    /// Chain of underlying errors
    pub causes: Vec<String>,
}

impl Error {
    /// Create a new error with the specified kind and message
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            message: message.into(),
            severity: ErrorSeverity::Medium,
            source: "unknown".to_string(),
            plugin_id: None,
            correlation_id: None,
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
            backtrace: Some(std::backtrace::Backtrace::capture().to_string()),
            causes: Vec::new(),
        }
    }

    /// Set the error severity
    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the error source component
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Set the plugin ID
    pub fn plugin_id(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin_id = Some(plugin_id.into());
        self
    }

    /// Set the correlation ID
    pub fn correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Add metadata to the error
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add multiple metadata entries
    pub fn with_metadata(mut self, metadata: crate::types::Metadata) -> Self {
        self.metadata.extend(metadata);
        self
    }

    /// Add a cause to the error chain
    pub fn caused_by(mut self, cause: impl fmt::Display) -> Self {
        self.causes.push(cause.to_string());
        self
    }

    /// Check if this error should be handled automatically based on severity
    pub fn should_handle(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Low | ErrorSeverity::Medium)
    }

    /// Check if this error requires immediate attention
    pub fn is_critical(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Critical)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({}): {}",
            self.severity, self.source, self.id, self.message
        )
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to an error
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Add source information to an error
    fn with_source(self, source: impl Into<String>) -> Result<T>;

    /// Add plugin context to an error
    fn with_plugin(self, plugin_id: impl Into<String>) -> Result<T>;

    /// Add correlation ID to an error
    fn with_correlation(self, correlation_id: Uuid) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            Error::new(ErrorKind::Application, f())
                .caused_by(e)
        })
    }

    fn with_source(self, source: impl Into<String>) -> Result<T> {
        self.map_err(|e| {
            Error::new(ErrorKind::Application, e.to_string())
                .source(source)
                .caused_by(e)
        })
    }

    fn with_plugin(self, plugin_id: impl Into<String>) -> Result<T> {
        self.map_err(|e| {
            Error::new(ErrorKind::Plugin {
                plugin_id: Some(plugin_id.into()),
                plugin_name: None,
            }, e.to_string())
                .caused_by(e)
        })
    }

    fn with_correlation(self, correlation_id: Uuid) -> Result<T> {
        self.map_err(|e| {
            Error::new(ErrorKind::Application, e.to_string())
                .correlation_id(correlation_id)
                .caused_by(e)
        })
    }
}

/// Convenient error constructors for common error types
impl Error {
    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Configuration {
                key: None,
                validation_errors: Vec::new(),
            },
            message,
        ).severity(ErrorSeverity::High)
    }

    /// Create a manager error
    pub fn manager(
        manager_name: impl Into<String>,
        operation: ManagerOperation,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorKind::Manager {
                manager_name: manager_name.into(),
                operation,
            },
            message,
        ).severity(ErrorSeverity::High)
    }

    /// Create a task error
    pub fn task(task_id: Option<Uuid>, message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Task {
                task_id,
                task_name: None,
                cancelled: false,
            },
            message,
        )
    }

    /// Create a file operation error
    pub fn file(path: impl Into<String>, operation: FileOperation, message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::File {
                path: Some(path.into()),
                operation,
            },
            message,
        )
    }

    /// Create a validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Validation {
                field: Some(field.into()),
                rules: Vec::new(),
            },
            message,
        ).severity(ErrorSeverity::Medium)
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message)
            .severity(ErrorSeverity::Medium)
    }

    /// Create a critical system error
    pub fn critical(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Application, message)
            .severity(ErrorSeverity::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = Error::config("Invalid configuration value")
            .source("config_manager")
            .metadata("key", serde_json::Value::String("database.host".to_string()));

        assert_eq!(error.severity, ErrorSeverity::High);
        assert_eq!(error.source, "config_manager");
        assert!(matches!(error.kind, ErrorKind::Configuration { .. }));
        assert!(error.metadata.contains_key("key"));
    }

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Low < ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium < ErrorSeverity::High);
        assert!(ErrorSeverity::High < ErrorSeverity::Critical);
    }

    #[test]
    fn test_result_ext() {
        let result: std::result::Result<(), std::io::Error> =
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));

        let error = result.with_source("test_component").unwrap_err();
        assert_eq!(error.source, "test_component");
        assert!(!error.causes.is_empty());
    }
}