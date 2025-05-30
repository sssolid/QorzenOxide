// src/lib.rs

//! Qorzen Core - A modular plugin-based system with async core managers

#![cfg_attr(debug_assertions, allow(unsafe_code))]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(unsafe_code))]
#![warn(clippy::all)]
#![allow(clippy::module_name_repetitions)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    // SETUP PANIC HOOK FIRST
    console_error_panic_hook::set_once();

    // SETUP WASM LOGGING
    tracing_wasm::set_as_global_default();

    // NOW YOUR LOGS WILL WORK
    web_sys::console::error_1(&"🚀 WASM ENTRY POINT CALLED".into());
    web_sys::console::log_1(&"🚀 WASM ENTRY POINT CALLED".into());
    web_sys::console::warn_1(&"🚀 WASM ENTRY POINT CALLED".into());
    web_sys::console::info_1(&"🚀 WASM ENTRY POINT CALLED".into());

    // TRACING LOGS
    tracing::error!("🚀 TRACING ERROR LOG");
    tracing::warn!("🚀 TRACING WARN LOG");
    tracing::info!("🚀 TRACING INFO LOG");
    tracing::debug!("🚀 TRACING DEBUG LOG");

    // Just launch a simple div - DON'T use your complex App yet
    // Launch the Dioxus application for web
    dioxus::launch(ui::App);
}

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
pub mod utils_general;

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

pub const VERSION: &str = env!("CARGO_PKG_VERSION");