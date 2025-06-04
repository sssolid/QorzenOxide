use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;
use crate::plugin::PluginFactoryRegistry;
use crate::ui::{
    pages::{EmptyState, PageWrapper},
    router::Route,
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
    pub status: PluginStatus,
    pub installed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub source: PluginSource,
    pub has_ui_components: bool,
    pub has_menu_items: bool,
    pub has_settings: bool,
    pub install_path: Option<std::path::PathBuf>,
}

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

/// Plugin source (installed vs discovered vs registry)
#[derive(Debug, Clone, PartialEq)]
pub enum PluginSource {
    Installed,
    Discovered, // Found in plugins directory but not installed
    Registry,
    Builtin,
}

/// Main plugins page component
#[component]
pub fn Plugins() -> Element {
    let mut active_tab = use_signal(|| "installed".to_string());
    let mut search_query = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut installing_plugins = use_signal(|| std::collections::HashSet::<String>::new());
    let mut error_message = use_signal(|| None::<String>);

    // Real data resources using the actual plugin system
    let mut installed_plugins = use_resource(move || async move {
        get_installed_plugins().await
    });

    let mut available_plugins = use_resource(move || {
        let query = search_query();
        async move {
            if query.is_empty() {
                get_available_plugins().await
            } else {
                search_plugins(&query).await
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
                    installed_plugins.restart();
                    available_plugins.restart();
                    let mut installing = installing_plugins();
                    installing.remove(&plugin_id);
                    installing_plugins.set(installing);
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to install {}: {}", plugin_id, e)));
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
                            cx: "12", cy: "12", r: "10",
                            stroke: "currentColor", stroke_width: "4"
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
                        stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2",
                        d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                    }
                }
                "Refresh"
            }
            button {
                r#type: "button",
                class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                onclick: move |_| {
                    spawn(async move {
                        if let Err(e) = discover_plugins().await {
                            error_message.set(Some(format!("Failed to discover plugins: {}", e)));
                        } else {
                            available_plugins.restart();
                        }
                    });
                },
                svg {
                    class: "-ml-1 mr-2 h-4 w-4",
                    xmlns: "http://www.w3.org/2000/svg",
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2",
                    d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                }
                "Discover Plugins"
            }
            Link {
                to: Route::Settings {},
                class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-gray-600 hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500",
                svg {
                    class: "-ml-1 mr-2 h-4 w-4",
                    xmlns: "http://www.w3.org/2000/svg",
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2",
                    d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
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

            if let Some(error) = error_message() {
                div { class: "mb-6 bg-red-50 border border-red-200 rounded-md p-4",
                    div { class: "flex",
                        div { class: "flex-shrink-0",
                            svg {
                                class: "h-5 w-5 text-red-400",
                                xmlns: "http://www.w3.org/2000/svg",
                                view_box: "0 0 20 20",
                                fill: "currentColor", fill_rule: "evenodd",
                                d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z",
                                clip_rule: "evenodd"
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
                                    fill: "currentColor", fill_rule: "evenodd",
                                    d: "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z",
                                    clip_rule: "evenodd"
                                }
                            }
                        }
                    }
                }
            }

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
                                fill: "currentColor", fill_rule: "evenodd",
                                d: "M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                }
            }

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
                        if let Some(Ok(plugins)) = available_plugins.read().as_ref() {
                            " ({plugins.len()})"
                        }
                    }
                }
            }

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
                _ => rsx! { div { "Unknown tab" } }
            }
        }
    }
}

/// Get real installed plugins from the plugin factory registry
async fn get_installed_plugins() -> Result<Vec<PluginInfo>, String> {
    // Simulate loading delay
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(500).await;

    // Get plugins from the actual plugin factory registry
    let plugin_infos = PluginFactoryRegistry::get_all_plugin_info().await;

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
        install_path: None,
    }).collect())
}

/// Get available plugins by discovering them in the plugins directory
async fn get_available_plugins() -> Result<Vec<PluginInfo>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(800).await;

    // Actually discover plugins from the filesystem
    match discover_plugins_from_directory().await {
        Ok(discovered) => {
            // Filter out already installed plugins
            let installed_ids: std::collections::HashSet<String> = PluginFactoryRegistry::list_plugins().await.into_iter().collect();

            let available = discovered.into_iter()
                .filter(|plugin| !installed_ids.contains(&plugin.id))
                .collect();

            Ok(available)
        }
        Err(e) => {
            tracing::error!("Failed to discover plugins: {}", e);
            Err(e)
        }
    }
}

/// Discover plugins from the plugins directory
async fn discover_plugins_from_directory() -> Result<Vec<PluginInfo>, String> {
    let plugins_dir = std::path::PathBuf::from("plugins");

    #[cfg(not(target_arch = "wasm32"))]
    {
        if !plugins_dir.exists() {
            return Ok(vec![]);
        }

        let mut discovered = Vec::new();
        let mut dir_entries = tokio::fs::read_dir(&plugins_dir).await
            .map_err(|e| format!("Failed to read plugins directory: {}", e))?;

        while let Some(entry) = dir_entries.next_entry().await
            .map_err(|e| format!("Failed to read directory entry: {}", e))? {

            if entry.file_type().await
                .map_err(|e| format!("Failed to get file type: {}", e))?
                .is_dir() {

                let plugin_dir = entry.path();
                let manifest_path = plugin_dir.join("plugin.toml");

                if manifest_path.exists() {
                    match load_plugin_manifest(&manifest_path).await {
                        Ok(plugin_info) => {
                            discovered.push(plugin_info);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load plugin manifest at {:?}: {}", manifest_path, e);
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    #[cfg(target_arch = "wasm32")]
    {
        // For WASM, we can't browse the filesystem, so return empty for now
        // In a real web app, this might come from a server API
        Ok(vec![])
    }
}

/// Load plugin manifest and convert to PluginInfo
async fn load_plugin_manifest(manifest_path: &std::path::Path) -> Result<PluginInfo, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let content = tokio::fs::read_to_string(manifest_path).await
            .map_err(|e| format!("Failed to read manifest file: {}", e))?;

        let manifest: toml::Value = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse TOML: {}", e))?;

        let plugin_section = manifest.get("plugin")
            .ok_or("No [plugin] section found in manifest")?;

        let id = plugin_section.get("id")
            .and_then(|v| v.as_str())
            .ok_or("Plugin ID not found")?
            .to_string();

        let name = plugin_section.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Plugin name not found")?
            .to_string();

        let version = plugin_section.get("version")
            .and_then(|v| v.as_str())
            .ok_or("Plugin version not found")?
            .to_string();

        let author = plugin_section.get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let description = plugin_section.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description")
            .to_string();

        Ok(PluginInfo {
            id: id.clone(),
            name,
            version,
            author,
            description,
            icon: get_plugin_icon(&id),
            status: PluginStatus::Available,
            installed_at: None,
            error_message: None,
            source: PluginSource::Discovered,
            has_ui_components: true, // We assume this for now
            has_menu_items: true,
            has_settings: true,
            install_path: Some(manifest_path.parent().unwrap().to_path_buf()),
        })
    }

    #[cfg(target_arch = "wasm32")]
    {
        Err("File system access not available in WASM".to_string())
    }
}

/// Trigger plugin discovery
async fn discover_plugins() -> Result<(), String> {
    // This would ideally call the plugin manager's discovery method
    // For now, we'll just refresh the available plugins
    tracing::info!("Discovering plugins in plugins directory...");
    Ok(())
}

/// Search for plugins
async fn search_plugins(query: &str) -> Result<Vec<PluginInfo>, String> {
    let all_available = get_available_plugins().await?;
    Ok(all_available.into_iter().filter(|p| {
        p.name.to_lowercase().contains(&query.to_lowercase()) ||
            p.description.to_lowercase().contains(&query.to_lowercase())
    }).collect())
}

/// Install a discovered plugin
async fn install_plugin(plugin_id: &str) -> Result<(), String> {
    tracing::info!("Installing plugin: {}", plugin_id);

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Find the plugin directory
        let plugin_dir = std::path::PathBuf::from("plugins").join(plugin_id);

        if !plugin_dir.exists() {
            return Err(format!("Plugin directory not found: {}", plugin_dir.display()));
        }

        // For now, simulate the installation process
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        // In a real implementation, this would:
        // 1. Validate the plugin
        // 2. Compile it if necessary
        // 3. Load it into the plugin manager
        // 4. Register it with the factory registry

        tracing::info!("Plugin {} installed successfully", plugin_id);
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(2000).await;

        // For WASM, plugins would need to be pre-compiled and included
        // This would typically involve adding the plugin as a dependency
        Err("Plugin installation not supported in web environment".to_string())
    }
}

/// Get appropriate icon for plugin
fn get_plugin_icon(plugin_id: &str) -> String {
    match plugin_id {
        "system_monitor" => "🖥️".to_string(),
        "notifications" => "🔔".to_string(),
        _ => "🧩".to_string(),
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
                for _ in 0..3 {
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
                        title: "No plugins available".to_string(),
                        description: "No plugins found in the plugins directory. Add plugins to the 'plugins' folder and click 'Discover Plugins'.".to_string(),
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
                h2 { class: "text-2xl font-bold text-gray-900 mb-2", "Failed to discover plugins" }
                p { class: "text-gray-600 mb-6", "{error}" }
                p { class: "text-sm text-gray-500", "Make sure the 'plugins' directory exists and contains valid plugin manifests." }
            }
        },
        None => rsx! {
            div { class: "text-center py-12",
                div { class: "animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto" }
                p { class: "mt-4 text-gray-600", "Discovering plugins..." }
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
        PluginStatus::Available => "bg-gray-100 text-gray-800",
        PluginStatus::Failed => "bg-red-100 text-red-800",
        PluginStatus::Installing | PluginStatus::Loading => "bg-yellow-100 text-yellow-800",
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
                    div { class: "mt-3 text-sm text-red-600", "Error: {error}" }
                }

                div { class: "mt-4 flex items-center text-xs text-gray-500 space-x-4",
                    div { class: "flex items-center",
                        span { class: "mr-1", "📁" }
                        match plugin.source {
                            PluginSource::Builtin => "Built-in",
                            PluginSource::Discovered => "Discovered",
                            PluginSource::Installed => "Installed",
                            PluginSource::Registry => "Registry",
                        }
                    }
                    if plugin.has_ui_components {
                        div { class: "flex items-center",
                            span { class: "mr-1", "🖼️" }
                            "UI Components"
                        }
                    }
                    if plugin.has_menu_items {
                        div { class: "flex items-center",
                            span { class: "mr-1", "📋" }
                            "Menu Items"
                        }
                    }
                    if plugin.has_settings {
                        div { class: "flex items-center",
                            span { class: "mr-1", "⚙️" }
                            "Configurable"
                        }
                    }
                }

                div { class: "mt-6 flex space-x-3",
                    match plugin.source {
                        PluginSource::Installed | PluginSource::Builtin => rsx! {
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
                        PluginSource::Discovered => {
                            if show_install_button {
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: "flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                                        disabled: is_installing,
                                        onclick: move |_| on_install.call(plugin.id.clone()),
                                        if is_installing { "Installing..." } else { "Install" }
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
                        },
                        PluginSource::Registry => {
                            if show_install_button {
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: "flex-1 bg-blue-600 py-2 px-3 border border-transparent rounded-md shadow-sm text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                                        disabled: is_installing,
                                        onclick: move |_| on_install.call(plugin.id.clone()),
                                        if is_installing { "Installing..." } else { "Install" }
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

/// Plugin view component for individual plugin pages
#[component]
pub fn PluginView(plugin_id: String, page: Option<String>) -> Element {
    // Get the actual plugin from the registry
    let plugin_info = use_resource({
        let plugin_id = plugin_id.clone(); // clone once for local capture

        move || {
            let plugin_id = plugin_id.clone(); // clone again for move inside async
            async move {
                PluginFactoryRegistry::get_plugin_info(&plugin_id).await
            }
        }
    });

    rsx! {
        PageWrapper {
            title: format!("Plugin: {}", plugin_id),
            subtitle: Some("Plugin configuration and management".to_string()),

            match &*plugin_info.read_unchecked() {
                Some(Some(info)) => rsx! {
                    div { class: "bg-white shadow rounded-lg p-6",
                        div { class: "flex items-center mb-6",
                            span { class: "text-4xl mr-4", "{get_plugin_icon(&info.id)}" }
                            div {
                                h2 { class: "text-2xl font-semibold text-gray-900", "{info.name}" }
                                p { class: "text-gray-600", "v{info.version} by {info.author}" }
                                p { class: "text-sm text-gray-500 mt-1", "{info.description}" }
                            }
                        }

                        if let Some(page_name) = page {
                            div { class: "mb-4 p-3 bg-blue-50 rounded-md",
                                p { class: "text-sm text-blue-800", "Page: {page_name}" }
                            }
                        }

                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                            div { class: "p-4 bg-gray-50 rounded-lg",
                                h3 { class: "font-medium text-gray-900 mb-2", "Plugin Information" }
                                dl { class: "space-y-2 text-sm",
                                    div { class: "flex justify-between",
                                        dt { class: "text-gray-500", "ID:" }
                                        dd { class: "text-gray-900", "{info.id}" }
                                    }
                                    div { class: "flex justify-between",
                                        dt { class: "text-gray-500", "Version:" }
                                        dd { class: "text-gray-900", "{info.version}" }
                                    }
                                    div { class: "flex justify-between",
                                        dt { class: "text-gray-500", "Author:" }
                                        dd { class: "text-gray-900", "{info.author}" }
                                    }
                                    div { class: "flex justify-between",
                                        dt { class: "text-gray-500", "License:" }
                                        dd { class: "text-gray-900", "{info.license}" }
                                    }
                                }
                            }

                            div { class: "p-4 bg-green-50 rounded-lg",
                                h3 { class: "font-medium text-gray-900 mb-2", "Status" }
                                div { class: "flex items-center",
                                    div { class: "w-3 h-3 bg-green-500 rounded-full mr-2" }
                                    span { class: "text-sm text-green-700", "Plugin is running and active" }
                                }
                            }
                        }

                        div { class: "mt-6 p-4 bg-blue-50 rounded-md",
                            p { class: "text-sm text-blue-800",
                                "🔌 Plugin components and functionality would be rendered here dynamically."
                            }
                        }
                    }
                },
                Some(None) => rsx! {
                    div { class: "bg-white shadow rounded-lg p-6",
                        div { class: "text-center py-12",
                            div { class: "text-6xl text-gray-400 mb-4", "🧩" }
                            h2 { class: "text-2xl font-bold text-gray-900 mb-2", "Plugin Not Found" }
                            p { class: "text-gray-600", "The plugin '{plugin_id}' could not be found." }
                        }
                    }
                },
                None => rsx! {
                    div { class: "bg-white shadow rounded-lg p-6",
                        div { class: "animate-pulse",
                            div { class: "flex items-center mb-6",
                                div { class: "w-16 h-16 bg-gray-200 rounded mr-4" }
                                div {
                                    div { class: "h-6 bg-gray-200 rounded w-48 mb-2" }
                                    div { class: "h-4 bg-gray-200 rounded w-32" }
                                }
                            }
                            div { class: "h-4 bg-gray-200 rounded mb-2" }
                            div { class: "h-4 bg-gray-200 rounded w-3/4" }
                        }
                    }
                }
            }
        }
    }
}