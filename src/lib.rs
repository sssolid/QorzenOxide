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
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    // Emergency error handling
    console_error_panic_hook::set_once();

    // Force immediate console output
    web_sys::console::log_1(&"🚀 WASM APPLICATION STARTING".into());

    // Set up tracing with error handling
    if let Err(e) = tracing_wasm::try_set_as_global_default() {
        web_sys::console::error_1(&format!("Failed to set up tracing: {:?}", e).into());
    }

    web_sys::console::log_1(&"🔧 Tracing setup complete".into());

    // Launch Dioxus with error handling
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
pub use error::{Error, ErrorKind, Result, ResultExt};
pub use manager::{Manager, ManagerState, ManagerStatus};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
