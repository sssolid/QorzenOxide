// src/app/mod.rs - Updated to use debug core
use std::sync::OnceLock;
use tokio::sync::RwLock;

static GLOBAL_APP: OnceLock<RwLock<Option<debug::MinimalApp>>> = OnceLock::new();

pub mod debug;
pub mod core;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::{ApplicationCore};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
use wasm::{ApplicationCore};

pub use debug::{DebugApplicationCore, MinimalApp, AppState, InitError};

/// Initialize the global application instance with debug capabilities
pub async fn initialize_global_app() -> Result<(), InitError> {
    let app_lock = GLOBAL_APP.get_or_init(|| RwLock::new(None));

    let mut debug_core = DebugApplicationCore::new();
    let app = debug_core.initialize_with_debug().await?;

    *app_lock.write().await = Some(app);
    Ok(())
}

/// Get the current application state for UI display
pub async fn get_app_state() -> Option<AppState> {
    let app_lock = GLOBAL_APP.get()?;
    let app_guard = app_lock.read().await;
    app_guard.as_ref().map(|app| app.state.clone())
}

/// Get the application status string
pub async fn get_app_status() -> String {
    match GLOBAL_APP.get() {
        Some(app_lock) => {
            let app_guard = app_lock.read().await;
            match app_guard.as_ref() {
                Some(app) => app.get_status(),
                None => "Application not initialized".to_string(),
            }
        }
        None => "Application not created".to_string(),
    }
}