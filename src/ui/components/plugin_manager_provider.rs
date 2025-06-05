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
                    // Just mark as ready since plugins are already registered
                    plugin_manager_ready.set(true);
                });
            }
        });

        use_context_provider(|| plugin_manager_ready());
    }

    rsx! { {children} }
}

/// Hook to check if plugin manager is ready
pub fn use_plugin_manager_ready() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let ready = use_context::<Signal<bool>>();
        ready()
    }

    #[cfg(target_arch = "wasm32")]
    {
        true // Always ready in WASM since plugins are compiled in
    }
}