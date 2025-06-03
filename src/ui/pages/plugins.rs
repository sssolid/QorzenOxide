use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::ui::{
    pages::{EmptyState, PageWrapper},
    router::Route,
    state::use_app_state,
};

/// Plugin information from the plugin manager
#[derive(Debug, Clone, PartialEq)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub icon: String,
    pub rating: f32,
    pub downloads: String,
    pub category: String,
    pub status: PluginStatus,
    pub installed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub source: PluginSource,
}

/// Plugin status enum
#[derive(Debug, Clone, PartialEq)]
pub enum PluginStatus {
    Available,
    Installing,
    Installed,
    Loading,
    Running,
    Failed,
    Uninstalling,
}

impl std::fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Available => write!(f, "Available"),
            Self::Installing => write!(f, "Installing"),
            Self::Installed => write!(f, "Installed"),
            Self::Loading => write!(f, "Loading"),
            Self::Running => write!(f, "Running"),
            Self::Failed => write!(f, "Failed"),
            Self::Uninstalling => write!(f, "Uninstalling"),
        }
    }
}

/// Plugin source (installed vs registry)
#[derive(Debug, Clone, PartialEq)]
pub enum PluginSource {
    Installed,
    Registry,
}

/// Plugin update information
#[derive(Debug, Clone, PartialEq)]
pub struct PluginUpdate {
    pub plugin_id: String,
    pub name: String,
    pub icon: String,
    pub current_version: String,
    pub new_version: String,
    pub changelog: Vec<String>,
}

/// Main plugins page component
#[component]
pub fn Plugins() -> Element {
    let mut active_tab = use_signal(|| "installed".to_string());
    let mut search_query = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut installing_plugins = use_signal(|| std::collections::HashSet::<String>::new());
    let mut error_message = use_signal(|| None::<String>);

    // Get real plugin data
    let mut installed_plugins = use_resource(move || async move {
        get_installed_plugins().await
    });

    let mut available_plugins = use_resource(move || {
        let query = search_query();
        async move {
            if query.is_empty() {
                get_featured_plugins().await
            } else {
                search_registry_plugins(&query).await
            }
        }
    });

    let handle_refresh = move |_| {
        loading.set(true);
        spawn(async move {
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(1000).await;

            // Trigger refresh of resources
            installed_plugins.restart();
            available_plugins.restart();
            loading.set(false);
        });
    };

    let handle_install = move |plugin_id: String| {
        let mut installing = installing_plugins();
        installing.insert(plugin_id.clone());
        installing_plugins.set(installing);

        spawn(async move {
            match install_plugin(&plugin_id).await {
                Ok(_) => {
                    // Refresh the installed plugins list
                    installed_plugins.restart();

                    // Remove from installing set
                    let mut installing = installing_plugins();
                    installing.remove(&plugin_id);
                    installing_plugins.set(installing);
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to install {}: {}", plugin_id, e)));

                    // Remove from installing set
                    let mut installing = installing_plugins();
                    installing.remove(&plugin_id);
                    installing_plugins.set(installing);
                }
            }
        });
    };

    let page_actions = rsx! {
        div { class: "flex space-x-3",
            button {
                r#type: "button",
                class: "inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                onclick: handle_refresh,
                disabled: loading(),
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
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                    }
                }
                "Refresh"
            }
            Link {
                to: Route::Settings {},
                class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                svg {
                    class: "-ml-1 mr-2 h-4 w-4",
                    xmlns: "http://www.w3.org/2000/svg",
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke: "currentColor",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    stroke_width: "2",
                    d: "M12 6v6m0 0v6m0-6h6m-6 0H6"
                }
                "Plugin Settings"
            }
        }
    };

    rsx! {
        PageWrapper {
            title: "Plugins".to_string(),
            subtitle: Some("Extend your application with plugins".to_string()),
            actions: Some(page_actions),

            // Error message
            if let Some(error) = error_message() {
                div { class: "mb-6 bg-red-50 border border-red-200 rounded-md p-4",
                    div { class: "flex",
                        div { class: "flex-shrink-0",
                            svg {
                                class: "h-5 w-5 text-red-400",
                                xmlns: "http://www.w3.org/2000/svg",
                                view_box: "0 0 20 20",
                                fill: "currentColor",
                                path {
                                    fill_rule: "evenodd",
                                    d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z",
                                    clip_rule: "evenodd"
                                }
                            }
                        }
                        div { class: "ml-3",
                            h3 { class: "text-sm font-medium text-red-800", "Error" }
                            div { class: "mt-2 text-sm text-red-700", "{error}" }
                        }
                        div { class: "ml-auto pl-3",
                            button {
                                r#type: "button",
                                class: "inline-flex rounded-md bg-red-50 p-1.5 text-red-500 hover:bg-red-100 focus:outline-none focus:ring-2 focus:ring-red-600 focus:ring-offset-2 focus:ring-offset-red-50",
                                onclick: move |_| error_message.set(None),
                                span { class: "sr-only", "Dismiss" }
                                svg {
                                    class: "h-5 w-5",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    view_box: "0 0 20 20",
                                    fill: "currentColor",
                                    path {
                                        fill_rule: "evenodd",
                                        d: "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z",
                                        clip_rule: "evenodd"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Search bar (only show for available tab)
            if active_tab() == "available" {
                div { class: "mb-6",
                    div { class: "relative",
                        input {
                            r#type: "text",
                            placeholder: "Search plugins...",
                            class: "block w-full pr-12 border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 sm:text-sm",
                            value: "{search_query}",
                            oninput: move |e| {
                                search_query.set(e.value());
                                available_plugins.restart();
                            }
                        }
                        div { class: "absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none",
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
            }

            // Tabs
            div { class: "border-b border-gray-200 mb-6",
                nav { class: "-mb-px flex space-x-8",
                    button {
                        r#type: "button",
                        class: format!("py-2 px-1 border-b-2 font-medium text-sm {}",
                            if active_tab() == "installed" {
                                "border-blue-500 text-blue-600"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                            }
                        ),
                        onclick: move |_| active_tab.set("installed".to_string()),
                        "Installed"
                        if let Some(Ok(plugins)) = installed_plugins.read().as_ref() {
                            " ({plugins.len()})"
                        }
                    }
                    button {
                        r#type: "button",
                        class: format!("py-2 px-1 border-b-2 font-medium text-sm {}",
                            if active_tab() == "available" {
                                "border-blue-500 text-blue-600"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                            }
                        ),
                        onclick: move |_| active_tab.set("available".to_string()),
                        "Available"
                    }
                    button {
                        r#type: "button",
                        class: format!("py-2 px-1 border-b-2 font-medium text-sm {}",
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
                        plugins_resource: installed_plugins,
                        installing_plugins: installing_plugins(),
                    }
                },
                "available" => rsx! {
                    AvailablePluginsTab {
                        plugins_resource: available_plugins,
                        installing_plugins: installing_plugins(),
                        on_install: handle_install,
                    }
                },
                "updates" => rsx! {
                    UpdatesTab {}
                },
                _ => rsx! {
                    div { "Unknown tab" }
                }
            }
        }
    }
}

/// Installed plugins tab component
#[component]
fn InstalledPluginsTab(
    plugins_resource: Resource<Result<Vec<PluginInfo>, String>>,
    installing_plugins: std::collections::HashSet<String>,
) -> Element {
    match &*plugins_resource.read_unchecked() {
        Some(Ok(plugins)) => {
            if plugins.is_empty() {
                rsx! {
                    EmptyState {
                        icon: "🧩".to_string(),
                        title: "No plugins installed".to_string(),
                        description: "Install plugins to extend your application functionality".to_string(),
                    }
                }
            } else {
                rsx! {
                    div { class: "grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for plugin in plugins {
                            PluginCard {
                                key: "{plugin.id}",
                                plugin: plugin.clone(),
                                is_installing: installing_plugins.contains(&plugin.id),
                                show_install_button: false,
                                on_install: move |_: String| {},
                            }
                        }
                    }
                }
            }
        }
        Some(Err(error)) => rsx! {
            div { class: "text-center py-12",
                div { class: "text-6xl text-red-500 mb-4", "⚠️" }
                h2 { class: "text-2xl font-bold text-gray-900 mb-2", "Failed to load plugins" }
                p { class: "text-gray-600 mb-6", "{error}" }
            }
        },
        None => rsx! {
            div { class: "grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                for _ in 0..6 {
                    div { class: "animate-pulse",
                        div { class: "bg-white overflow-hidden shadow rounded-lg",
                            div { class: "p-6",
                                div { class: "flex items-center justify-between mb-4",
                                    div { class: "flex items-center space-x-3",
                                        div { class: "w-12 h-12 bg-gray-200 rounded-lg" }
                                        div {
                                            div { class: "h-4 bg-gray-200 rounded w-24 mb-2" }
                                            div { class: "h-3 bg-gray-200 rounded w-16" }
                                        }
                                    }
                                }
                                div { class: "h-3 bg-gray-200 rounded mb-2" }
                                div { class: "h-3 bg-gray-200 rounded w-3/4" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Available plugins tab component
#[component]
fn AvailablePluginsTab(
    plugins_resource: Resource<Result<Vec<PluginInfo>, String>>,
    installing_plugins: std::collections::HashSet<String>,
    on_install: EventHandler<String>,
) -> Element {
    match &*plugins_resource.read_unchecked() {
        Some(Ok(plugins)) => {
            if plugins.is_empty() {
                rsx! {
                    EmptyState {
                        icon: "🔍".to_string(),
                        title: "No plugins found".to_string(),
                        description: "Try adjusting your search terms".to_string(),
                    }
                }
            } else {
                rsx! {
                    div { class: "grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for plugin in plugins {
                            PluginCard {
                                key: "{plugin.id}",
                                plugin: plugin.clone(),
                                is_installing: installing_plugins.contains(&plugin.id),
                                show_install_button: true,
                                on_install: on_install,
                            }
                        }
                    }
                }
            }
        }
        Some(Err(error)) => rsx! {
            div { class: "text-center py-12",
                div { class: "text-6xl text-red-500 mb-4", "⚠️" }
                h2 { class: "text-2xl font-bold text-gray-900 mb-2", "Failed to load plugins" }
                p { class: "text-gray-600 mb-6", "{error}" }
            }
        },
        None => rsx! {
            div { class: "text-center py-12",
                div { class: "animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto" }
                p { class: "mt-4 text-gray-600", "Loading plugins..." }
            }
        }
    }
}

/// Plugin card component
#[component]
fn PluginCard(
    plugin: PluginInfo,
    is_installing: bool,
    show_install_button: bool,
    on_install: EventHandler<String>,
) -> Element {
    let status_color = match plugin.status {
        PluginStatus::Running => "bg-green-100 text-green-800",
        PluginStatus::Installed => "bg-blue-100 text-blue-800",
        PluginStatus::Failed => "bg-red-100 text-red-800",
        PluginStatus::Installing | PluginStatus::Loading => "bg-yellow-100 text-yellow-800",
        PluginStatus::Available => "bg-gray-100 text-gray-800",
        PluginStatus::Uninstalling => "bg-orange-100 text-orange-800",
    };

    rsx! {
        div { class: "bg-white overflow-hidden shadow rounded-lg hover:shadow-md transition-shadow",
            div { class: "p-6",
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center space-x-3",
                        div { class: "flex-shrink-0",
                            span { class: "text-3xl", "{plugin.icon}" }
                        }
                        div { class: "ml-4",
                            h3 { class: "text-lg font-medium text-gray-900", "{plugin.name}" }
                            p { class: "text-sm text-gray-500", "v{plugin.version} by {plugin.author}" }
                        }
                    }
                    div { class: "flex-shrink-0",
                        span {
                            class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {status_color}",
                            "{plugin.status}"
                        }
                    }
                }

                div { class: "mt-4",
                    p { class: "text-sm text-gray-600", "{plugin.description}" }
                }

                if let Some(error) = &plugin.error_message {
                    div { class: "mt-3 text-sm text-red-600",
                        "Error: {error}"
                    }
                }

                div { class: "mt-4 flex items-center text-xs text-gray-500 space-x-4",
                    div { class: "flex items-center",
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
                    div { class: "flex items-center",
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
                    div { class: "flex items-center",
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

                div { class: "mt-6 flex space-x-3",
                    match plugin.source {
                        PluginSource::Installed => rsx! {
                            Link {
                                to: Route::Plugin { plugin_id: plugin.id.clone() },
                                class: "flex-1 bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 text-center",
                                "Configure"
                            }
                            if plugin.status == PluginStatus::Failed {
                                button {
                                    r#type: "button",
                                    class: "flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Restart"
                                }
                            } else if plugin.status == PluginStatus::Running {
                                button {
                                    r#type: "button",
                                    class: "flex-1 bg-yellow-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-yellow-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-yellow-500",
                                    "Stop"
                                }
                            }
                        },
                        PluginSource::Registry => {
                            if show_install_button {
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: "flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                                        disabled: is_installing,
                                        onclick: move |_| on_install.call(plugin.id.clone()),
                                        if is_installing {
                                            "Installing..."
                                        } else {
                                            "Install"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                        "Details"
                                    }
                                }
                            } else {
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: "flex-1 bg-white py-2 px-3 border border-gray-300 rounded-md shadow-sm text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                        "View Details"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Updates tab component
#[component]
fn UpdatesTab() -> Element {
    let updates = get_plugin_updates();

    if updates.is_empty() {
        rsx! {
            EmptyState {
                icon: "✅".to_string(),
                title: "All plugins up to date".to_string(),
                description: "Your plugins are running the latest versions".to_string(),
            }
        }
    } else {
        rsx! {
            div { class: "space-y-4",
                for update in updates {
                    UpdateCard {
                        key: "{update.plugin_id}",
                        update: update.clone(),
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

    let handle_update = move |_| {
        updating.set(true);
        spawn(async move {
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(2000).await;
            updating.set(false);
        });
    };

    rsx! {
        div { class: "bg-white overflow-hidden shadow rounded-lg border-l-4 border-yellow-400",
            div { class: "p-6",
                div { class: "flex items-center justify-between",
                    div { class: "flex items-center space-x-3",
                        div { class: "flex-shrink-0",
                            span { class: "text-2xl", "{update.icon}" }
                        }
                        div { class: "ml-4",
                            h3 { class: "text-lg font-medium text-gray-900", "{update.name}" }
                            p { class: "text-sm text-gray-500",
                                "Update available: v{update.current_version} → v{update.new_version}"
                            }
                        }
                    }
                    div { class: "flex space-x-3",
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
                            if updating() {
                                "Updating..."
                            } else {
                                "Update"
                            }
                        }
                    }
                }

                if !update.changelog.is_empty() {
                    div { class: "mt-4",
                        h4 { class: "text-sm font-medium text-gray-900 mb-2", "What's New:" }
                        ul { class: "text-sm text-gray-600 space-y-1",
                            for item in &update.changelog {
                                li { class: "flex items-start",
                                    span { class: "text-green-500 mr-2", "•" }
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
pub fn PluginView(plugin_id: String, page: Option<String>) -> Element {
    rsx! {
        PageWrapper {
            title: format!("Plugin: {}", plugin_id),
            subtitle: Some("Plugin configuration and management".to_string()),
            div { class: "bg-white shadow rounded-lg p-6",
                h2 { class: "text-xl font-semibold text-gray-900 mb-4", "Plugin: {plugin_id}" }
                if let Some(page_name) = page {
                    p { class: "text-gray-600 mb-4", "Page: {page_name}" }
                }
                p { class: "text-gray-600",
                    "This would show the plugin's interface and configuration options."
                }
                div { class: "mt-6 p-4 bg-blue-50 rounded-md",
                    p { class: "text-sm text-blue-800",
                        "🔌 Plugin content would be rendered here dynamically based on the plugin's configuration."
                    }
                }
            }
        }
    }
}

// Mock data functions (replace with real API calls)

/// Get installed plugins from the plugin manager
async fn get_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    // This would call the actual plugin manager
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Simulate API call
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(500).await;
    }

    Ok(vec![
        PluginInfo {
            id: "inventory_management".to_string(),
            name: "Inventory Management".to_string(),
            version: "2.1.0".to_string(),
            author: "QorzenTech".to_string(),
            description: "Complete inventory tracking and management system with barcode support"
                .to_string(),
            icon: "📦".to_string(),
            rating: 4.8,
            downloads: "12.5k".to_string(),
            category: "Business".to_string(),
            status: PluginStatus::Running,
            installed_at: Some(chrono::Utc::now()),
            error_message: None,
            source: PluginSource::Installed,
        },
        PluginInfo {
            id: "analytics_dashboard".to_string(),
            name: "Analytics Dashboard".to_string(),
            version: "1.5.2".to_string(),
            author: "DataViz Inc".to_string(),
            description: "Advanced analytics and reporting with beautiful visualizations".to_string(),
            icon: "📊".to_string(),
            rating: 4.6,
            downloads: "8.2k".to_string(),
            category: "Analytics".to_string(),
            status: PluginStatus::Failed,
            installed_at: Some(chrono::Utc::now()),
            error_message: Some("Initialization timeout".to_string()),
            source: PluginSource::Installed,
        },
    ])
}

/// Get featured plugins from registry
async fn get_featured_plugins() -> Result<Vec<PluginInfo>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(800).await;
    }

    Ok(vec![
        PluginInfo {
            id: "crm_system".to_string(),
            name: "Customer Relations".to_string(),
            version: "1.2.0".to_string(),
            author: "CRM Solutions".to_string(),
            description: "Comprehensive customer relationship management system".to_string(),
            icon: "👥".to_string(),
            rating: 4.4,
            downloads: "6.8k".to_string(),
            category: "Business".to_string(),
            status: PluginStatus::Available,
            installed_at: None,
            error_message: None,
            source: PluginSource::Registry,
        },
        PluginInfo {
            id: "payment_processing".to_string(),
            name: "Payment Processing".to_string(),
            version: "2.3.1".to_string(),
            author: "PayTech".to_string(),
            description: "Secure payment processing with multiple gateway support".to_string(),
            icon: "💳".to_string(),
            rating: 4.7,
            downloads: "9.4k".to_string(),
            category: "Finance".to_string(),
            status: PluginStatus::Available,
            installed_at: None,
            error_message: None,
            source: PluginSource::Registry,
        },
    ])
}

/// Search plugins in registry
async fn search_registry_plugins(query: &str) -> Result<Vec<PluginInfo>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(600).await;
    }

    let all_plugins = get_featured_plugins().await?;

    Ok(all_plugins
        .into_iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query.to_lowercase())
                || p.description.to_lowercase().contains(&query.to_lowercase())
                || p.category.to_lowercase().contains(&query.to_lowercase())
        })
        .collect())
}

/// Install a plugin
async fn install_plugin(plugin_id: &str) -> Result<(), String> {
    tracing::info!("Installing plugin: {}", plugin_id);

    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(2000).await;
    }

    // Simulate occasional failures
    if plugin_id == "payment_processing" && rand::random::<f32>() < 0.3 {
        return Err("Network timeout during installation".to_string());
    }

    Ok(())
}

/// Get plugin updates
fn get_plugin_updates() -> Vec<PluginUpdate> {
    vec![
        PluginUpdate {
            plugin_id: "inventory_management".to_string(),
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
            plugin_id: "analytics_dashboard".to_string(),
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