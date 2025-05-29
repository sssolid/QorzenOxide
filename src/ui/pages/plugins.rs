// src/ui/pages/plugins.rs - Plugin management and marketplace

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::ui::{
    pages::{PageWrapper, EmptyState},
    router::Route,
    state::use_app_state,
};

/// Main plugins page component
#[component]
pub fn Plugins() -> Element {
    let mut active_tab = use_signal(|| "installed".to_string());
    let mut search_query = use_signal(|| String::new());
    let mut loading = use_signal(|| false);

    // Mock plugin data
    let installed_plugins = get_installed_plugins();
    let available_plugins = get_available_plugins();

    rsx! {
        PageWrapper {
            title: "Plugins".to_string(),
            subtitle: Some("Extend your application with plugins".to_string()),
            actions: Some(rsx! {
                div {
                    class: "flex space-x-3",
                    button {
                        r#type: "button",
                        class: "inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                        onclick: move |_| {
                            loading.set(true);
                            spawn(async move {
                                #[cfg(not(target_arch = "wasm32"))]
                                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                                #[cfg(target_arch = "wasm32")]
                                gloo_timers::future::TimeoutFuture::new(1000).await;
                                loading.set(false);
                            });
                        },
                        if loading() {
                            svg {
                                class: "animate-spin -ml-1 mr-2 h-4 w-4",
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                circle {
                                    class: "opacity-25",
                                    cx: "12",
                                    cy: "12",
                                    r: "10",
                                    stroke: "currentColor",
                                    stroke_width: "4"
                                }
                                path {
                                    class: "opacity-75",
                                    fill: "currentColor",
                                    d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                                }
                            }
                        } else {
                            svg {
                                class: "-ml-1 mr-2 h-4 w-4",
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                }
                            }
                        }
                        "Refresh"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                        svg {
                            class: "-ml-1 mr-2 h-4 w-4",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M12 6v6m0 0v6m0-6h6m-6 0H6"
                            }
                        }
                        "Install Plugin"
                    }
                }
            }),

            // Search and filters
            div {
                class: "mb-6",
                div {
                    class: "relative",
                    input {
                        r#type: "text",
                        placeholder: "Search plugins...",
                        class: "block w-full pr-12 border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 sm:text-sm",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value())
                    }
                    div {
                        class: "absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none",
                        svg {
                            class: "h-5 w-5 text-gray-400",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                }
            }

            // Tabs
            div {
                class: "border-b border-gray-200 mb-6",
                nav {
                    class: "-mb-px flex space-x-8",
                    button {
                        r#type: "button",
                        class: format!(
                            "py-2 px-1 border-b-2 font-medium text-sm {}",
                            if active_tab() == "installed" {
                                "border-blue-500 text-blue-600"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                            }
                        ),
                        onclick: move |_| active_tab.set("installed".to_string()),
                        "Installed ({installed_plugins.len()})"
                    }
                    button {
                        r#type: "button",
                        class: format!(
                            "py-2 px-1 border-b-2 font-medium text-sm {}",
                            if active_tab() == "available" {
                                "border-blue-500 text-blue-600"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                            }
                        ),
                        onclick: move |_| active_tab.set("available".to_string()),
                        "Available ({available_plugins.len()})"
                    }
                    button {
                        r#type: "button",
                        class: format!(
                            "py-2 px-1 border-b-2 font-medium text-sm {}",
                            if active_tab() == "updates" {
                                "border-blue-500 text-blue-600"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                            }
                        ),
                        onclick: move |_| active_tab.set("updates".to_string()),
                        "Updates (2)"
                    }
                }
            }

            // Tab content
            match active_tab().as_str() {
                "installed" => rsx! {
                    InstalledPluginsTab {
                        plugins: installed_plugins,
                        search_query: search_query()
                    }
                },
                "available" => rsx! {
                    AvailablePluginsTab {
                        plugins: available_plugins,
                        search_query: search_query()
                    }
                },
                "updates" => rsx! {
                    UpdatesTab {}
                },
                _ => rsx! { div { "Unknown tab" } }
            }
        }
    }
}

/// Installed plugins tab
#[component]
fn InstalledPluginsTab(plugins: Vec<PluginInfo>, search_query: String) -> Element {
    let filtered_plugins: Vec<PluginInfo> = plugins.into_iter()
        .filter(|p| {
            if search_query.is_empty() {
                true
            } else {
                p.name.to_lowercase().contains(&search_query.to_lowercase()) ||
                    p.description.to_lowercase().contains(&search_query.to_lowercase())
            }
        })
        .collect();

    rsx! {
        if filtered_plugins.is_empty() {
            EmptyState {
                icon: "🧩".to_string(),
                title: if search_query.is_empty() {
                    "No plugins installed".to_string()
                } else {
                    "No plugins found".to_string()
                },
                description: if search_query.is_empty() {
                    "Install plugins to extend your application functionality".to_string()
                } else {
                    "Try adjusting your search terms".to_string()
                },
            }
        } else {
            div {
                class: "grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                for plugin in filtered_plugins {
                    PluginCard {
                        key: "{plugin.id}",
                        plugin: plugin,
                        is_installed: true
                    }
                }
            }
        }
    }
}

/// Available plugins tab
#[component]
fn AvailablePluginsTab(plugins: Vec<PluginInfo>, search_query: String) -> Element {
    let filtered_plugins: Vec<PluginInfo> = plugins.into_iter()
        .filter(|p| {
            if search_query.is_empty() {
                true
            } else {
                p.name.to_lowercase().contains(&search_query.to_lowercase()) ||
                    p.description.to_lowercase().contains(&search_query.to_lowercase())
            }
        })
        .collect();

    rsx! {
        if filtered_plugins.is_empty() {
            EmptyState {
                icon: "🔍".to_string(),
                title: "No plugins found".to_string(),
                description: "Try adjusting your search terms".to_string(),
            }
        } else {
            div {
                class: "grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                for plugin in filtered_plugins {
                    PluginCard {
                        key: "{plugin.id}",
                        plugin: plugin,
                        is_installed: false
                    }
                }
            }
        }
    }
}

/// Updates tab
#[component]
fn UpdatesTab() -> Element {
    let updates = get_plugin_updates();

    rsx! {
        if updates.is_empty() {
            EmptyState {
                icon: "✅".to_string(),
                title: "All plugins up to date".to_string(),
                description: "Your plugins are running the latest versions".to_string(),
            }
        } else {
            div {
                class: "space-y-4",
                for update in updates {
                    UpdateCard {
                        key: "{update.plugin_id}",
                        update: update
                    }
                }
            }
        }
    }
}

/// Individual plugin card component
#[component]
fn PluginCard(plugin: PluginInfo, is_installed: bool) -> Element {
    let mut installing = use_signal(|| false);
    let mut uninstalling = use_signal(|| false);

    let handle_install = {
        move |_| {
            installing.set(true);
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(2000).await;
                installing.set(false);
            });
        }
    };

    let handle_uninstall = {
        move |_| {
            uninstalling.set(true);
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(1500).await;
                uninstalling.set(false);
            });
        }
    };

    rsx! {
        div {
            class: "bg-white overflow-hidden shadow rounded-lg hover:shadow-md transition-shadow",
            div {
                class: "p-6",
                div {
                    class: "flex items-center justify-between",
                    div {
                        class: "flex items-center",
                        div {
                            class: "flex-shrink-0",
                            span {
                                class: "text-3xl",
                                "{plugin.icon}"
                            }
                        }
                        div {
                            class: "ml-4",
                            h3 {
                                class: "text-lg font-medium text-gray-900",
                                "{plugin.name}"
                            }
                            p {
                                class: "text-sm text-gray-500",
                                "v{plugin.version} by {plugin.author}"
                            }
                        }
                    }
                    div {
                        class: "flex-shrink-0",
                        if is_installed {
                            span {
                                class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800",
                                "Installed"
                            }
                        } else {
                            span {
                                class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800",
                                "Available"
                            }
                        }
                    }
                }

                div {
                    class: "mt-4",
                    p {
                        class: "text-sm text-gray-600",
                        "{plugin.description}"
                    }
                }

                // Plugin stats
                div {
                    class: "mt-4 flex items-center text-xs text-gray-500 space-x-4",
                    div {
                        class: "flex items-center",
                        svg {
                            class: "h-4 w-4 mr-1",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                d: "M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"
                            }
                        }
                        "{plugin.rating}/5"
                    }
                    div {
                        class: "flex items-center",
                        svg {
                            class: "h-4 w-4 mr-1",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm3.293-7.707a1 1 0 011.414 0L9 10.586V3a1 1 0 112 0v7.586l1.293-1.293a1 1 0 111.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z",
                                clip_rule: "evenodd"
                            }
                        }
                        "{plugin.downloads}"
                    }
                    div {
                        class: "flex items-center",
                        svg {
                            class: "h-4 w-4 mr-1",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M12.316 3.051a1 1 0 01.633 1.265l-4 12a1 1 0 11-1.898-.632l4-12a1 1 0 011.265-.633zM5.707 6.293a1 1 0 010 1.414L3.414 10l2.293 2.293a1 1 0 11-1.414 1.414l-3-3a1 1 0 010-1.414l3-3a1 1 0 011.414 0zm8.586 0a1 1 0 011.414 0l3 3a1 1 0 010 1.414l-3 3a1 1 0 11-1.414-1.414L16.586 10l-2.293-2.293a1 1 0 010-1.414z",
                                clip_rule: "evenodd"
                            }
                        }
                        "{plugin.category}"
                    }
                }

                // Action buttons
                div {
                    class: "mt-6 flex space-x-3",
                    if is_installed {
                        Link {
                            to: Route::Plugin { plugin_id: plugin.id.clone() },
                            class: "flex-1 bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 text-center",
                            "Configure"
                        }
                        button {
                            r#type: "button",
                            class: "flex-1 bg-red-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500 disabled:opacity-50",
                            disabled: uninstalling(),
                            onclick: handle_uninstall,
                            if uninstalling() { "Uninstalling..." } else { "Uninstall" }
                        }
                    } else {
                        button {
                            r#type: "button",
                            class: "flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                            disabled: installing(),
                            onclick: handle_install,
                            if installing() { "Installing..." } else { "Install" }
                        }
                        button {
                            r#type: "button",
                            class: "bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                            "Details"
                        }
                    }
                }
            }
        }
    }
}

/// Update card component
#[component]
fn UpdateCard(update: PluginUpdate) -> Element {
    let mut updating = use_signal(|| false);

    let handle_update = {
        move |_| {
            updating.set(true);
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(2000).await;
                updating.set(false);
            });
        }
    };

    rsx! {
        div {
            class: "bg-white overflow-hidden shadow rounded-lg border-l-4 border-yellow-400",
            div {
                class: "p-6",
                div {
                    class: "flex items-center justify-between",
                    div {
                        class: "flex items-center",
                        div {
                            class: "flex-shrink-0",
                            span {
                                class: "text-2xl",
                                "{update.icon}"
                            }
                        }
                        div {
                            class: "ml-4",
                            h3 {
                                class: "text-lg font-medium text-gray-900",
                                "{update.name}"
                            }
                            p {
                                class: "text-sm text-gray-500",
                                "Update available: v{update.current_version} → v{update.new_version}"
                            }
                        }
                    }
                    div {
                        class: "flex space-x-3",
                        button {
                            r#type: "button",
                            class: "bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                            "View Changes"
                        }
                        button {
                            r#type: "button",
                            class: "bg-yellow-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500 disabled:opacity-50",
                            disabled: updating(),
                            onclick: handle_update,
                            if updating() { "Updating..." } else { "Update" }
                        }
                    }
                }

                if !update.changelog.is_empty() {
                    div {
                        class: "mt-4",
                        h4 {
                            class: "text-sm font-medium text-gray-900 mb-2",
                            "What's New:"
                        }
                        ul {
                            class: "text-sm text-gray-600 space-y-1",
                            for item in &update.changelog {
                                li {
                                    class: "flex items-start",
                                    span {
                                        class: "text-green-500 mr-2",
                                        "•"
                                    }
                                    "{item}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Plugin view component for individual plugin pages
#[component]
pub fn PluginView(plugin_id: String, #[props(default = None)] page: Option<String>) -> Element {
    rsx! {
        PageWrapper {
            title: format!("Plugin: {}", plugin_id),
            subtitle: Some("Plugin configuration and management".to_string()),

            div {
                class: "bg-white shadow rounded-lg p-6",
                h2 {
                    class: "text-xl font-semibold text-gray-900 mb-4",
                    "Plugin: {plugin_id}"
                }

                if let Some(page_name) = page {
                    p {
                        class: "text-gray-600 mb-4",
                        "Page: {page_name}"
                    }
                }

                p {
                    class: "text-gray-600",
                    "This would show the plugin's interface and configuration options."
                }

                div {
                    class: "mt-6 p-4 bg-blue-50 rounded-md",
                    p {
                        class: "text-sm text-blue-800",
                        "🔌 Plugin content would be rendered here dynamically based on the plugin's configuration."
                    }
                }
            }
        }
    }
}

// Data structures and mock data
#[derive(Debug, Clone, PartialEq)]
struct PluginInfo {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    icon: String,
    rating: f32,
    downloads: String,
    category: String,
}

#[derive(Debug, Clone, PartialEq)]
struct PluginUpdate {
    plugin_id: String,
    name: String,
    icon: String,
    current_version: String,
    new_version: String,
    changelog: Vec<String>,
}

fn get_installed_plugins() -> Vec<PluginInfo> {
    vec![
        PluginInfo {
            id: "inventory".to_string(),
            name: "Inventory Management".to_string(),
            version: "2.1.0".to_string(),
            author: "QorzenTech".to_string(),
            description: "Complete inventory tracking and management system with barcode support".to_string(),
            icon: "📦".to_string(),
            rating: 4.8,
            downloads: "12.5k".to_string(),
            category: "Business".to_string(),
        },
        PluginInfo {
            id: "analytics".to_string(),
            name: "Analytics Dashboard".to_string(),
            version: "1.5.2".to_string(),
            author: "DataViz Inc".to_string(),
            description: "Advanced analytics and reporting with beautiful visualizations".to_string(),
            icon: "📊".to_string(),
            rating: 4.6,
            downloads: "8.2k".to_string(),
            category: "Analytics".to_string(),
        },
        PluginInfo {
            id: "backup".to_string(),
            name: "Backup & Sync".to_string(),
            version: "3.0.1".to_string(),
            author: "SecureData".to_string(),
            description: "Automated backup and synchronization across multiple cloud providers".to_string(),
            icon: "☁️".to_string(),
            rating: 4.9,
            downloads: "15.1k".to_string(),
            category: "Utility".to_string(),
        },
    ]
}

fn get_available_plugins() -> Vec<PluginInfo> {
    vec![
        PluginInfo {
            id: "crm".to_string(),
            name: "Customer Relations".to_string(),
            version: "1.2.0".to_string(),
            author: "CRM Solutions".to_string(),
            description: "Comprehensive customer relationship management system".to_string(),
            icon: "👥".to_string(),
            rating: 4.4,
            downloads: "6.8k".to_string(),
            category: "Business".to_string(),
        },
        PluginInfo {
            id: "payments".to_string(),
            name: "Payment Processing".to_string(),
            version: "2.3.1".to_string(),
            author: "PayTech".to_string(),
            description: "Secure payment processing with multiple gateway support".to_string(),
            icon: "💳".to_string(),
            rating: 4.7,
            downloads: "9.4k".to_string(),
            category: "Finance".to_string(),
        },
        PluginInfo {
            id: "notifications".to_string(),
            name: "Smart Notifications".to_string(),
            version: "1.0.5".to_string(),
            author: "NotifyMe".to_string(),
            description: "Advanced notification system with email, SMS, and push support".to_string(),
            icon: "🔔".to_string(),
            rating: 4.2,
            downloads: "3.1k".to_string(),
            category: "Communication".to_string(),
        },
        PluginInfo {
            id: "scheduler".to_string(),
            name: "Task Scheduler".to_string(),
            version: "1.8.0".to_string(),
            author: "TimeKeeper".to_string(),
            description: "Powerful task scheduling and automation system".to_string(),
            icon: "⏰".to_string(),
            rating: 4.5,
            downloads: "5.7k".to_string(),
            category: "Productivity".to_string(),
        },
    ]
}

fn get_plugin_updates() -> Vec<PluginUpdate> {
    vec![
        PluginUpdate {
            plugin_id: "inventory".to_string(),
            name: "Inventory Management".to_string(),
            icon: "📦".to_string(),
            current_version: "2.1.0".to_string(),
            new_version: "2.2.0".to_string(),
            changelog: vec![
                "Added bulk import/export functionality".to_string(),
                "Improved barcode scanning accuracy".to_string(),
                "Fixed issue with low stock alerts".to_string(),
                "Enhanced reporting capabilities".to_string(),
            ],
        },
        PluginUpdate {
            plugin_id: "analytics".to_string(),
            name: "Analytics Dashboard".to_string(),
            icon: "📊".to_string(),
            current_version: "1.5.2".to_string(),
            new_version: "1.6.0".to_string(),
            changelog: vec![
                "New real-time dashboard widgets".to_string(),
                "Added data export to Excel".to_string(),
                "Performance improvements for large datasets".to_string(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugins_component_creation() {
        let _plugins = rsx! { Plugins {} };
    }

    #[test]
    fn test_plugin_view_creation() {
        let _plugin_view = rsx! {
            PluginView {
                plugin_id: "test".to_string(),
                page: Some("config".to_string())
            }
        };
    }

    #[test]
    fn test_mock_data() {
        let installed = get_installed_plugins();
        let available = get_available_plugins();
        let updates = get_plugin_updates();

        assert!(!installed.is_empty());
        assert!(!available.is_empty());
        assert!(!updates.is_empty());
    }
}