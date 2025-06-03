# How to See and Use Plugins in Your App

## 1. Router Setup

First, make sure your router includes the plugin routes in your `ui/router.rs`:

```rust
// src/ui/router.rs
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Routable, Debug, PartialEq, Serialize, Deserialize)]
pub enum Route {
    #[route("/")]
    Home {},
    
    #[route("/plugins")]
    Plugins {},
    
    #[route("/plugins/:plugin_id")]
    Plugin { plugin_id: String },
    
    #[route("/plugins/:plugin_id/:page")]
    PluginPage { plugin_id: String, page: String },
    
    #[route("/settings")]
    Settings {},
    
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}
```

## 2. Main App Component

Update your main app component to include the plugins routes:

```rust
// src/ui/mod.rs or src/main.rs
use dioxus::prelude::*;
use crate::ui::{
    router::Route,
    pages::{Home, Plugins, PluginView, Settings, PageNotFound},
};

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn RouteComponent() -> Element {
    rsx! {
        div { class: "min-h-screen bg-gray-50",
            // Navigation bar
            Navigation {}
            
            // Main content area
            main { class: "container mx-auto px-4 py-8",
                Outlet::<Route> {}
            }
        }
    }
}

// Route outlet that renders the appropriate page
#[component]
fn Outlet() -> Element {
    let route = use_route::<Route>();
    
    match route {
        Route::Home {} => rsx! { Home {} },
        Route::Plugins {} => rsx! { Plugins {} },
        Route::Plugin { plugin_id } => rsx! { 
            PluginView { 
                plugin_id: plugin_id.clone(),
                page: None
            } 
        },
        Route::PluginPage { plugin_id, page } => rsx! { 
            PluginView { 
                plugin_id: plugin_id.clone(),
                page: Some(page.clone())
            } 
        },
        Route::Settings {} => rsx! { Settings {} },
        Route::PageNotFound { route } => rsx! { 
            PageNotFound { 
                route: route.join("/") 
            } 
        },
    }
}
```

## 3. Navigation Component

Create a navigation component that includes a link to the plugins page:

```rust
// src/ui/components/navigation.rs
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::ui::router::Route;

#[component]
pub fn Navigation() -> Element {
    rsx! {
        nav { class: "bg-white shadow-sm border-b border-gray-200",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                div { class: "flex justify-between h-16",
                    // Logo/Brand
                    div { class: "flex items-center",
                        Link { 
                            to: Route::Home {},
                            class: "text-xl font-bold text-gray-900",
                            "Qorzen Oxide"
                        }
                    }
                    
                    // Navigation links
                    div { class: "flex items-center space-x-8",
                        Link {
                            to: Route::Home {},
                            class: "text-gray-600 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium",
                            "Home"
                        }
                        Link {
                            to: Route::Plugins {},
                            class: "text-gray-600 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium flex items-center",
                            span { class: "mr-1", "🧩" }
                            "Plugins"
                        }
                        Link {
                            to: Route::Settings {},
                            class: "text-gray-600 hover:text-gray-900 px-3 py-2 rounded-md text-sm font-medium",
                            "Settings"
                        }
                    }
                }
            }
        }
    }
}
```

## 4. Main Application Setup

In your `main.rs` (for desktop) or main web entry point:

```rust
// src/main.rs (Desktop)
use dioxus::prelude::*;
use qorzen_oxide::{
    app::ApplicationCore,
    ui::App,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("qorzen_oxide=debug,info")
        .init();

    // Create and initialize the application core
    let mut app_core = ApplicationCore::new();
    app_core.initialize().await?;
    
    println!("🚀 Qorzen Oxide starting...");
    println!("📝 Check the plugins at: http://localhost:8080/plugins");
    
    // Start the Dioxus app
    let config = dioxus::desktop::Config::new()
        .with_window(
            dioxus::desktop::WindowBuilder::new()
                .with_title("Qorzen Oxide")
                .with_resizable(true)
                .with_inner_size(dioxus::desktop::LogicalSize::new(1200, 800))
        );
    
    // Provide the app core as context and launch
    dioxus::launch_with_props(App, (), config);
    
    // Graceful shutdown
    app_core.shutdown().await?;
    Ok(())
}
```

## 5. WASM Entry Point

For web/WASM builds, create a similar setup:

```rust
// src/lib.rs (for WASM)
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use qorzen_oxide::{
    app::ApplicationCore,
    ui::App,
};

#[wasm_bindgen(start)]
pub async fn main() {
    // Setup panic hook and logging for WASM
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    
    // Initialize the application core
    let mut app_core = ApplicationCore::new();
    if let Err(e) = app_core.initialize().await {
        web_sys::console::error_1(&format!("Failed to initialize app: {}", e).into());
        return;
    }
    
    web_sys::console::log_1(&"🚀 Qorzen Oxide initialized - check /plugins for plugin management".into());
    
    // Launch the Dioxus app
    dioxus::launch(App);
}
```

## 6. How to Access Plugins

Once your app is running:

### **Desktop Application:**
1. **Start the app**: `cargo run --bin qorzen_desktop --features="desktop,example_plugin"`
2. **Navigate to plugins**: Click the "🧩 Plugins" link in the navigation bar
3. **URL**: `http://localhost:8080/plugins` (if running with web server)

### **Web Application:**
1. **Build for web**: `dx build --release --features="web,example_plugin"`
2. **Serve the app**: `dx serve --features="web,example_plugin"`
3. **Open browser**: Go to `http://localhost:8080`
4. **Navigate**: Click "🧩 Plugins" in the navigation

## 7. What You'll See

In the Plugins page, you should see:

### **Installed Tab:**
- **System Monitor** 🖥️ - CPU, Memory, Disk monitoring
- **Notifications** 🔔 - Alert and notification system
- **Product Catalog** 📦 - Product management (if `example_plugin` feature enabled)

### **Plugin Cards Show:**
- Plugin name, version, and author
- Status (Running/Failed/etc.)
- Description
- Available UI components, menu items, settings
- Configure button to access plugin settings

### **Individual Plugin Pages:**
Click "Configure" on any plugin to see:
- Plugin details and status
- Configuration options
- Plugin-specific UI components
- Management controls

## 8. Plugin Components in Your App

To render plugin UI components in other parts of your app:

```rust
#[component]
pub fn Dashboard() -> Element {
    // This would show the system metrics widget
    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
            // Render the system monitor widget
            PluginComponent {
                plugin_id: "system_monitor",
                component_id: "system_metrics",
                props: serde_json::json!({
                    "refresh_interval": 5000
                })
            }
            
            // Render the notification center
            PluginComponent {
                plugin_id: "notifications", 
                component_id: "notification_center",
                props: serde_json::json!({
                    "max_visible": 5
                })
            }
        }
    }
}

#[component]
fn PluginComponent(
    plugin_id: &'static str,
    component_id: &'static str, 
    props: serde_json::Value
) -> Element {
    // Implementation to render plugin component
    // This would use the plugin manager to render the actual component
    rsx! {
        div { "Plugin component: {plugin_id}::{component_id}" }
    }
}
```

## 9. Verification Steps

To verify everything is working:

1. **Check logs** during startup for:
   ```
   INFO Registered plugin factory: system_monitor
   INFO Registered plugin factory: notifications
   INFO Successfully loaded plugin: system_monitor
   ```

2. **Navigate to `/plugins`** in your app

3. **Look for the plugins** listed in the "Installed" tab

4. **Click "Configure"** on any plugin to see its details

5. **Check browser console** for any errors

The plugins should now be fully visible and accessible in your running application!