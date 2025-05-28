// src/error.rs - Enhanced error handling with platform and plugin support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    Configuration {
        key: Option<String>,
        validation_errors: Vec<String>,
    },
    Manager {
        manager_name: String,
        operation: ManagerOperation,
    },
    Event {
        event_type: Option<String>,
        subscriber_id: Option<Uuid>,
        operation: EventOperation,
    },
    Task {
        task_id: Option<Uuid>,
        task_name: Option<String>,
        cancelled: bool,
    },
    File {
        path: Option<String>,
        operation: FileOperation,
    },
    Concurrency {
        thread_id: Option<String>,
        operation: ConcurrencyOperation,
    },
    Plugin {
        plugin_id: Option<String>,
        plugin_name: Option<String>,
        dependency_missing: Option<String>,
    },
    Platform {
        platform: String,
        feature: String,
        fallback_available: bool,
    },
    Permission {
        required_permission: String,
        user_role: Option<String>,
    },
    Network {
        status_code: Option<u16>,
        endpoint: Option<String>,
    },
    Database {
        query: Option<String>,
        connection_id: Option<String>,
    },
    Security {
        user_id: Option<String>,
        permission: Option<String>,
    },
    Validation {
        field: Option<String>,
        rules: Vec<String>,
    },
    Authentication {
        provider: Option<String>,
        reason: String,
    },
    Authorization {
        resource: String,
        action: String,
        user_id: Option<String>,
    },
    Application,
    Io,
    Serialization,
    Timeout,
    ResourceExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagerOperation {
    Initialize,
    Shutdown,
    Configure,
    Pause,
    Resume,
    Register,
    Unregister,
    Operation(String),
    Reload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigOperation {
    Get,
    Set,
    Reload,
    Validate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventOperation {
    Publish,
    Subscribe,
    Unsubscribe,
    Process,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperation {
    Read,
    Write,
    Delete,
    Copy,
    Move,
    CreateDirectory,
    Metadata,
    Lock,
    Watch,
    Compress,
    Decompress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcurrencyOperation {
    ThreadPool,
    Spawn,
    Sync,
    Channel,
    Lock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub id: Uuid,
    pub kind: ErrorKind,
    pub message: String,
    pub severity: ErrorSeverity,
    pub source: String,
    pub plugin_id: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub metadata: crate::types::Metadata,
    pub backtrace: Option<String>,
    pub causes: Vec<String>,
}

impl Error {
    /// Creates a new error with the specified kind and message
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
            backtrace: Self::capture_backtrace(),
            causes: Vec::new(),
        }
    }

    /// Capture backtrace if available on the platform
    fn capture_backtrace() -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(std::backtrace::Backtrace::capture().to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    /// Sets the error severity
    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets the error source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Sets the plugin ID
    pub fn plugin_id(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin_id = Some(plugin_id.into());
        self
    }

    /// Sets the correlation ID
    pub fn correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Adds metadata to the error
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Adds metadata collection to the error
    pub fn with_metadata(mut self, metadata: crate::types::Metadata) -> Self {
        self.metadata.extend(metadata);
        self
    }

    /// Adds a cause to the error chain
    pub fn caused_by(mut self, cause: impl fmt::Display) -> Self {
        self.causes.push(cause.to_string());
        self
    }

    /// Checks if the error should be handled automatically
    pub fn should_handle(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Low | ErrorSeverity::Medium)
    }

    /// Checks if the error is critical
    pub fn is_critical(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Critical)
    }

    /// Creates a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Configuration {
                key: None,
                validation_errors: Vec::new(),
            },
            message,
        )
            .severity(ErrorSeverity::High)
    }

    /// Creates a manager operation error
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
        )
            .severity(ErrorSeverity::High)
    }

    /// Creates a platform-specific error
    pub fn platform(
        platform: impl Into<String>,
        feature: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorKind::Platform {
                platform: platform.into(),
                feature: feature.into(),
                fallback_available: false,
            },
            message,
        )
            .severity(ErrorSeverity::Medium)
    }

    /// Creates a permission error
    pub fn permission(required_permission: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Permission {
                required_permission: required_permission.into(),
                user_role: None,
            },
            message,
        )
            .severity(ErrorSeverity::High)
    }

    /// Creates a plugin error
    pub fn plugin(plugin_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Plugin {
                plugin_id: Some(plugin_id.into()),
                plugin_name: None,
                dependency_missing: None,
            },
            message,
        )
            .severity(ErrorSeverity::Medium)
    }

    /// Creates an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::new(
            ErrorKind::Authentication {
                provider: None,
                reason: msg.clone(),
            },
            msg,
        )
            .severity(ErrorSeverity::High)
    }

    /// Creates an authorization error
    pub fn authorization(
        resource: impl Into<String>,
        action: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorKind::Authorization {
                resource: resource.into(),
                action: action.into(),
                user_id: None,
            },
            message,
        )
            .severity(ErrorSeverity::High)
    }

    /// Creates a file operation error
    pub fn file(
        path: impl Into<String>,
        operation: FileOperation,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorKind::File {
                path: Some(path.into()),
                operation,
            },
            message,
        )
    }

    /// Creates a task error
    pub fn task(
        task_id: Option<uuid::Uuid>,
        task_name: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            ErrorKind::Task {
                task_id,
                task_name,
                cancelled: false,
            },
            message,
        )
    }

    /// Creates a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message)
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

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        let msg = err.to_string();

        let mut error = Error::new(ErrorKind::Io, msg);
        error.source = "std::io::Error".to_string();
        error.severity = ErrorSeverity::High;

        error
    }
}

/// Extension trait for Results to add context
pub trait ResultExt<T> {
    /// Adds context to an error
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Sets the error source
    fn with_source(self, source: impl Into<String>) -> Result<T>;

    /// Sets the plugin ID
    fn with_plugin(self, plugin_id: impl Into<String>) -> Result<T>;

    /// Sets the correlation ID
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
        self.map_err(|e| Error::new(ErrorKind::Application, f()).caused_by(e))
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
            Error::new(
                ErrorKind::Plugin {
                    plugin_id: Some(plugin_id.into()),
                    plugin_name: None,
                    dependency_missing: None,
                },
                e.to_string(),
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = Error::config("Invalid configuration value")
            .source("config_manager")
            .metadata(
                "key",
                serde_json::Value::String("database.host".to_string()),
            );

        assert_eq!(error.severity, ErrorSeverity::High);
        assert_eq!(error.source, "config_manager");
        assert!(matches!(error.kind, ErrorKind::Configuration { .. }));
        assert!(error.metadata.contains_key("key"));
    }

    #[test]
    fn test_platform_error() {
        let error = Error::platform("wasm", "filesystem", "File API not available");
        assert!(matches!(error.kind, ErrorKind::Platform { .. }));
        assert_eq!(error.severity, ErrorSeverity::Medium);
    }

    #[test]
    fn test_permission_error() {
        let error = Error::permission("admin.users.read", "Access denied");
        assert!(matches!(error.kind, ErrorKind::Permission { .. }));
        assert_eq!(error.severity, ErrorSeverity::High);
    }
}