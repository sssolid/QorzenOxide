use dioxus::prelude::*;

/// Provider component for plugin management
#[component]
pub fn PluginManagerProvider(children: Element) -> Element {
    // For now, just pass through since plugins are registered at startup
    // In the future, this could manage desktop-specific plugin operations

    #[cfg(not(target_arch = "wasm32"))]
    {
        let plugin_manager_ready = use_signal(|| false);

        use_effect({
            let mut plugin_manager_ready = plugin_manager_ready;
            move || {
                spawn(async move {
                    // Wait a bit for the backend to initialize
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                    // Mark as ready since plugins should be registered by now
                    plugin_manager_ready.set(true);

                    tracing::debug!("Plugin manager provider marked as ready");
                });
            }
        });

        use_context_provider(|| plugin_manager_ready);
    }

    #[cfg(target_arch = "wasm32")]
    {
        // For WASM, plugins are always ready since they're compiled in
        let plugin_manager_ready = use_signal(|| true);
        use_context_provider(|| plugin_manager_ready);
    }

    rsx! { {children} }
}

/// Hook to check if plugin manager is ready
pub fn use_plugin_manager_ready() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(ready_signal) = try_use_context::<Signal<bool>>() {
            ready_signal()
        } else {
            // Fallback: assume not ready if context not available
            false
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(ready_signal) = try_use_context::<Signal<bool>>() {
            ready_signal()
        } else {
            // For WASM, always assume ready as fallback
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_provider_creation() {
        // Basic test to ensure the component can be created
        let _provider = rsx! {
            PluginManagerProvider {
                div { "Test content" }
            }
        };
    }
}