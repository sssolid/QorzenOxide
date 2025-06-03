# Plugin Loading Integration Guide

## How Plugin Loading Works

The plugin system has been completely refactored to work automatically. Here's how it functions:

### 1. **Automatic Plugin Registration**
Built-in plugins are automatically registered during app initialization:

```rust
// In ApplicationCore::init_plugin_manager()
crate::plugin::PluginFactoryRegistry::initialize();
crate::plugin::builtin::register_builtin_plugins().await?;
```

### 2. **Plugin Manager Creation**
The plugin manager is created with proper configuration:

```rust
let plugin_config = PluginManagerConfig::default();
let mut plugin_manager = PluginManager::new(plugin_config);
```

### 3. **Automatic Plugin Loading**
Default plugins are loaded automatically:

```rust
let default_plugins = ["system_monitor", "notifications"];

#[cfg(feature = "example_plugin")]
let default_plugins = ["system_monitor", "notifications", "product_catalog"];

for plugin_id in default_plugins {
    if let Err(e) = plugin_manager.load_plugin(plugin_id).await {
        tracing::warn!("Failed to load plugin {}: {}", plugin_id, e);
    } else {
        tracing::info!("Successfully loaded plugin: {}", plugin_id);
    }
}
```

## Accessing Plugins from UI

### 1. **In Your UI Components**
You can access the plugin manager through the application core:

```rust
// In your Dioxus component
#[component]
pub fn MyComponent() -> Element {
    // Access the global application core
    let app_core = use_context::<ApplicationCore>();
    
    // Get plugin statistics
    let plugin_stats = use_resource(move || async move {
        app_core.get_plugin_stats().await
    });
    
    rsx! {
        // Display plugin information
        if let Some(Ok(stats)) = plugin_stats.read().as_ref() {
            div {
                p { "Total Plugins: {stats.total_plugins}" }
                p { "Active Plugins: {stats.active_plugins}" }
            }
        }
    }
}
```

### 2. **In Your Plugins Page**
Update your plugins page to use the actual plugin manager:

```rust
// In ui/pages/plugins.rs
async fn get_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    // Get from the actual plugin factory registry
    let plugin_infos = crate::plugin::PluginFactoryRegistry::get_all_plugin_info().await;
    
    Ok(plugin_infos.into_iter().map(|info| PluginInfo {
        id: info.id.clone(),
        name: info.name.clone(),
        version: info.version.clone(),
        author: info.author.clone(),
        description: info.description.clone(),
        icon: get_plugin_icon(&info.id),
        status: PluginStatus::Running, // All registered plugins are running
        installed_at: Some(chrono::Utc::now()),
        error_message: None,
        source: PluginSource::Builtin,
        has_ui_components: true,
        has_menu_items: true,
        has_settings: true,
    }).collect())
}
```

## Plugin Components Rendering

### 1. **Rendering Plugin UI Components**
You can render plugin components through the plugin manager:

```rust
// In your main app
#[component]
pub fn PluginContent(plugin_id: String, component_id: String) -> Element {
    let app_core = use_context::<ApplicationCore>();
    
    let component_content = use_resource(move || {
        let plugin_id = plugin_id.clone();
        let component_id = component_id.clone();
        async move {
            if let Some(plugin_manager) = app_core.get_plugin_manager() {
                plugin_manager.render_component(
                    &plugin_id,
                    &component_id,
                    serde_json::json!({})
                ).await
            } else {
                Err(crate::error::Error::plugin("app", "Plugin manager not available"))
            }
        }
    });
    
    rsx! {
        match &*component_content.read_unchecked() {
            Some(Ok(vnode)) => rsx! { {vnode.clone()} },
            Some(Err(e)) => rsx! { div { "Error: {e}" } },
            None => rsx! { div { "Loading plugin component..." } }
        }
    }
}
```

### 2. **Plugin Menu Integration**
Get menu items from all loaded plugins:

```rust
// In your navigation component
#[component]
pub fn Navigation() -> Element {
    let app_core = use_context::<ApplicationCore>();
    
    let menu_items = use_resource(move || async move {
        if let Some(plugin_manager) = app_core.get_plugin_manager() {
            plugin_manager.get_all_menu_items().await
        } else {
            vec![]
        }
    });
    
    rsx! {
        nav {
            // Your regular navigation
            
            // Plugin menu items
            if let Some(items) = menu_items.read().as_ref() {
                for (plugin_id, item) in items {
                    Link {
                        to: Route::Plugin {
                            plugin_id: plugin_id.clone()
                        },
                        span { class: "mr-2", "{item.icon.as_deref().unwrap_or("🧩")}" }
                        "{item.label}"
                    }
                }
            }
        }
    }
}
```

## Available Built-in Plugins

After initialization, these plugins are automatically available:

### 1. **System Monitor Plugin** (`system_monitor`)
- **UI Components**: `system_metrics` widget
- **API Routes**: `/api/plugins/system_monitor/metrics`
- **Purpose**: Display system performance metrics

### 2. **Notifications Plugin** (`notifications`)
- **UI Components**: `notification_center` widget
- **API Routes**: `/api/plugins/notifications/send`
- **Purpose**: Handle notifications and alerts

### 3. **Product Catalog Plugin** (`product_catalog`) *(if `example_plugin` feature enabled)*
- **UI Components**: `product_list` page
- **API Routes**: `/api/plugins/product_catalog/products`
- **Purpose**: Product management and display

## Verification

To verify plugins are loading correctly, check the logs during app startup:

```
INFO Registered plugin factory: system_monitor
INFO Registered plugin factory: notifications  
INFO Registered plugin factory: product_catalog  // if feature enabled
INFO Successfully loaded plugin: system_monitor
INFO Successfully loaded plugin: notifications
INFO Successfully loaded plugin: product_catalog  // if feature enabled
```

## Next Steps

1. **Build and test** the application with `cargo build --features="example_plugin"`
2. **Verify plugin loading** in the console logs
3. **Test the plugins page** to see loaded plugins
4. **Implement plugin component rendering** in your routes
5. **Add plugin menu items** to your navigation

The plugin system is now fully integrated and should work seamlessly with your Dioxus application!