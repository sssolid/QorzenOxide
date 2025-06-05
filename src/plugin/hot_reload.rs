// src/plugin/hot_reload.rs
#[cfg(not(target_arch = "wasm32"))]
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use crate::error::Result;
use crate::plugin::PluginManager;

/// Hot reload manager for plugins
#[cfg(not(target_arch = "wasm32"))]
pub struct PluginHotReloader {
    plugin_manager: Arc<RwLock<PluginManager>>,
    plugins_dir: PathBuf,
    _watcher: Option<RecommendedWatcher>,
    reload_tx: mpsc::UnboundedSender<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PluginHotReloader {
    /// Create a new hot reloader
    pub fn new(plugin_manager: Arc<RwLock<PluginManager>>, plugins_dir: PathBuf) -> Self {
        let (reload_tx, _) = mpsc::unbounded_channel();

        Self {
            plugin_manager,
            plugins_dir,
            _watcher: None,
            reload_tx,
        }
    }

    /// Start watching for plugin changes
    pub async fn start_watching(&mut self) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let reload_tx = self.reload_tx.clone();

            // Create file watcher
            let mut watcher = RecommendedWatcher::new(
                move |res: notify::Result<Event>| {
                    if let Ok(event) = res {
                        if let Err(e) = tx.send(event) {
                            tracing::error!("Failed to send file event: {}", e);
                        }
                    }
                },
                Config::default(),
            ).map_err(|e| crate::error::Error::platform("native", "hot_reload", format!("Failed to create file watcher: {}", e)))?;

            // Watch the plugins directory
            watcher.watch(&self.plugins_dir, RecursiveMode::Recursive)
                .map_err(|e| crate::error::Error::platform("native", "hot_reload", format!("Failed to watch plugins directory: {}", e)))?;

            self._watcher = Some(watcher);

            // Start the event processing task
            let plugin_manager = Arc::clone(&self.plugin_manager);
            tokio::spawn(async move {
                let mut debounce_map = std::collections::HashMap::<String, tokio::time::Instant>::new();
                let debounce_duration = Duration::from_millis(500);

                while let Some(event) = rx.recv().await {
                    if let EventKind::Modify(_) = event.kind {
                        for path in event.paths {
                            if let Some(plugin_id) = Self::extract_plugin_id(&path) {
                                // Debounce rapid file changes
                                let now = tokio::time::Instant::now();
                                if let Some(last_reload) = debounce_map.get(&plugin_id) {
                                    if now.duration_since(*last_reload) < debounce_duration {
                                        continue;
                                    }
                                }
                                debounce_map.insert(plugin_id.clone(), now);

                                // Trigger reload after a short delay
                                let manager = Arc::clone(&plugin_manager);
                                let plugin_id_clone = plugin_id.clone();
                                let reload_tx_clone = reload_tx.clone();

                                tokio::spawn(async move {
                                    sleep(Duration::from_millis(100)).await;

                                    tracing::info!("Hot reloading plugin: {}", plugin_id_clone);

                                    let manager = manager.read().await;

                                    // Stop the plugin
                                    if let Err(e) = manager.stop_plugin(&plugin_id_clone).await {
                                        tracing::warn!("Failed to stop plugin {} during hot reload: {}", plugin_id_clone, e);
                                    }

                                    // Small delay to ensure clean shutdown
                                    sleep(Duration::from_millis(100)).await;

                                    // Reload the plugin
                                    match manager.load_plugin(&plugin_id_clone).await {
                                        Ok(_) => {
                                            tracing::info!("Successfully hot reloaded plugin: {}", plugin_id_clone);
                                            if let Err(e) = reload_tx_clone.send(plugin_id_clone.clone()) {
                                                tracing::error!("Failed to send reload notification: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to hot reload plugin {}: {}", plugin_id_clone, e);
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            });

            tracing::info!("Hot reloading enabled for plugins directory: {}", self.plugins_dir.display());
        }

        #[cfg(target_arch = "wasm32")]
        {
            tracing::info!("Hot reloading not available in WASM environment");
        }

        Ok(())
    }

    /// Extract plugin ID from file path
    fn extract_plugin_id(path: &std::path::Path) -> Option<String> {
        // Look for plugin.toml or source files in plugin directories
        let path_str = path.to_string_lossy();

        if path_str.contains("plugin.toml") || path_str.contains(".rs") {
            // Try to extract plugin ID from path like "plugins/my_plugin/..."
            let components: Vec<&str> = path.components()
                .filter_map(|c| match c {
                    std::path::Component::Normal(name) => name.to_str(),
                    _ => None,
                })
                .collect();

            if let Some(plugins_idx) = components.iter().position(|&c| c == "plugins") {
                if let Some(plugin_id) = components.get(plugins_idx + 1) {
                    return Some(plugin_id.to_string());
                }
            }
        }

        None
    }

    /// Get a receiver for reload notifications
    pub fn reload_receiver(&self) -> mpsc::UnboundedReceiver<String> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Store the sender for future use
        rx
    }
}