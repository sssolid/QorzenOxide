// src/lib.rs

//! Qorzen Core - A modular plugin-based system with async core managers
//!
//! This library provides a comprehensive foundation for building plugin-based applications
//! with strong emphasis on type safety, async operations, and modular architecture.
//!
//! # Core Components
//!
//! - **Configuration Management**: Type-safe configuration with hot-reloading
//! - **Event System**: High-performance pub/sub event bus
//! - **Logging**: Structured logging with multiple outputs
//! - **Task Management**: Async task execution with progress tracking
//! - **File Management**: Safe concurrent file operations
//! - **Error Handling**: Comprehensive error management with context
//! - **Concurrency**: Thread pool and async coordination utilities
//!
//! # Example
//!
//! ```rust,no_run
//! use qorzen_core::{ApplicationCore, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut app = ApplicationCore::new().await?;
//!     app.initialize().await?;
//!
//!     // Application logic here
//!
//!     app.shutdown().await?;
//!     Ok(())
//! }
//! ```

// #![deny(missing_docs)]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(unsafe_code))]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    rust_2018_idioms,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_import_braces,
    unused_qualifications
)]
#![allow(clippy::module_name_repetitions)]

pub mod types;
pub mod config;
pub mod error;
pub mod event;
pub mod file;
pub mod logging;
pub mod manager;
pub mod task;
pub mod concurrency;
pub mod app;
pub mod utils;

// Re-export commonly used types
pub use app::ApplicationCore;
pub use error::{Error, ErrorKind, Result, ResultExt};
pub use manager::{Manager, ManagerState, ManagerStatus};

/// Current version of the Qorzen Core library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");