// src/lib.rs

//! Qorzen Core - A modular plugin-based system with async core managers

#![cfg_attr(debug_assertions, allow(unsafe_code))]
#![deny(unsafe_code)]
#![cfg_attr(test, allow(unsafe_code))]
#![warn(clippy::all)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::result_large_err)]
#![allow(clippy::type_complexity)]
#![allow(clippy::large_enum_variant)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    web_sys::console::log_1(&"🚀 WASM ENTRY POINT CALLED".into());
    tracing::info!("🚀 TRACING INFO LOG");

    wasm_bindgen_futures::spawn_local(async {
        plugin::PluginFactoryRegistry::initialize();

        // Register plugins based on feature flags
        if let Err(e) = plugin::builtin::register_builtin_plugins().await {
            tracing::error!("Failed to register builtin plugins: {}", e);
            web_sys::console::error_1(&format!("Plugin registration failed: {}", e).into());
        } else {
            tracing::info!("Successfully registered builtin plugins for WASM");
        }
    });

    dioxus::launch(ui::App);
}

// Core modules (always available)
pub mod app;
pub mod auth;
pub mod config;
pub mod error;
pub mod event;
pub mod manager;
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
