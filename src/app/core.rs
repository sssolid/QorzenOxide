// src/app/core.rs

use crate::app::ApplicationCore;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::RwLock;
#[cfg(not(target_arch = "wasm32"))]
use once_cell::sync::OnceCell;

#[cfg(not(target_arch = "wasm32"))]
static APP_CORE: OnceCell<Arc<RwLock<ApplicationCore>>> = OnceCell::new();

#[cfg(not(target_arch = "wasm32"))]
pub fn set_application_core(core: ApplicationCore) {
    let _ = APP_CORE.set(Arc::new(RwLock::new(core)));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_application_core() -> Option<Arc<RwLock<ApplicationCore>>> {
    APP_CORE.get().cloned()
}

#[cfg(target_arch = "wasm32")]
pub fn get_application_core() -> Option<()> {
    // Return nothing in WASM builds
    None
}
