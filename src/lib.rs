// src/lib.rs

//! Qorzen Core - A modular plugin-based system with async core managers

#![deny(unsafe_code)]
#![cfg_attr(test, allow(unsafe_code))]
#![warn(clippy::all)]
#![allow(clippy::module_name_repetitions)]

// Core modules (always available)
pub mod app;
pub mod auth;
pub mod error;
pub mod event;
pub mod manager;
pub mod config;
pub mod platform;
pub mod plugin;
pub mod types;
pub mod ui;
pub mod utils;

// Native-only modules
#[cfg(not(target_arch = "wasm32"))]
pub mod concurrency;
#[cfg(not(target_arch = "wasm32"))]
pub mod file;
#[cfg(not(target_arch = "wasm32"))]
pub mod logging;
#[cfg(not(target_arch = "wasm32"))]
pub mod task;

// Re-export commonly used types
pub use app::ApplicationCore;
pub use error::{Error, ErrorKind, Result, ResultExt};
pub use manager::{Manager, ManagerState, ManagerStatus};
pub use ui::App;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");