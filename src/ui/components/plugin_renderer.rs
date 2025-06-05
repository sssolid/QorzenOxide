use dioxus::prelude::*;
use crate::plugin::PluginFactoryRegistry;
use crate::ui::services::plugin_service::use_plugin_service;

/// Renders a plugin component dynamically
#[component]
pub fn PluginComponentRenderer(
    plugin_id: String,
    component_id: String,
    #[props(default = serde_json::Value::Object(serde_json::Map::new()))]
    props: serde_json::Value,
) -> Element {
    let component_content = use_resource({
        let plugin_id = plugin_id.clone();
        let component_id = component_id.clone();
        let props = props.clone();
        move || {
            let plugin_id = plugin_id.clone();
            let component_id = component_id.clone();
            let props = props.clone();
            async move {
                render_plugin_component(plugin_id, component_id, props).await
            }
        }
    });

    match &*component_content.read_unchecked() {
        Some(Ok(content)) => rsx! {
            div {
                class: "plugin-component",
                dangerous_inner_html: "{content}"
            }
        },
        Some(Err(e)) => rsx! {
            div {
                class: "text-red-500 p-4 border border-red-200 rounded-md bg-red-50",
                div { class: "flex items-center",
                    span { class: "text-xl mr-2", "⚠️" }
                    strong { "Plugin Component Error" }
                }
                p { class: "mt-2 text-sm", "{e}" }
            }
        },
        None => rsx! {
            div {
                class: "flex items-center justify-center p-8",
                div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" }
                span { class: "ml-3 text-gray-600", "Loading plugin component..." }
            }
        }
    }
}

async fn render_plugin_component(
    plugin_id: String,
    component_id: String,
    props: serde_json::Value,
) -> Result<String, String> {
    match PluginFactoryRegistry::create_plugin(&plugin_id).await {
        Some(plugin) => {
            match plugin.render_component(&component_id, props) {
                Ok(_vnode) => {
                    // For now, return HTML that represents the component
                    // In a full implementation, we'd convert VNode to HTML
                    Ok(format!(
                        r#"<div class="plugin-component-{}" data-plugin-id="{}">
                            <h3>Plugin Component: {}</h3>
                            <p>Component ID: {}</p>
                            <div class="plugin-content">
                                <!-- Plugin {} component content would render here -->
                                <div class="bg-blue-50 p-4 rounded-lg">
                                    <p class="text-blue-800">🔌 {} component is active</p>
                                </div>
                            </div>
                        </div>"#,
                        component_id, plugin_id, plugin_id, component_id, plugin_id, component_id
                    ))
                }
                Err(e) => Err(format!("Failed to render component: {}", e)),
            }
        }
        None => Err(format!("Plugin '{}' not found", plugin_id)),
    }
}

/// Wrapper for plugin pages
#[component]
pub fn PluginPageWrapper(plugin_id: String, page: Option<String>) -> Element {
    let plugin_service = use_plugin_service();

    let plugin_info = use_resource({
        let plugin_id = plugin_id.clone();
        move || {
            let plugin_id = plugin_id.clone();
            async move {
                PluginFactoryRegistry::get_plugin_info(&plugin_id).await
            }
        }
    });

    let mut plugin_status = use_resource({
        let plugin_id = plugin_id.clone();
        let plugin_service = plugin_service.clone();
        move || {
            let plugin_id = plugin_id.clone();
            let plugin_service = plugin_service.clone();
            async move {
                let service = plugin_service.read().await;
                service.get_plugin_status(&plugin_id).await
            }
        }
    });

    match &*plugin_info.read_unchecked() {
        Some(Some(info)) => {
            let is_loaded = match plugin_status.read_unchecked().as_ref() {
                Some(Ok(status)) => status == "loaded",
                _ => false,
            };

            if !is_loaded {
                return rsx! {
                    div { class: "text-center py-12",
                        div { class: "text-6xl text-yellow-500 mb-4", "⚠️" }
                        h2 { class: "text-2xl font-bold text-gray-900 mb-2", "Plugin Not Loaded" }
                        p { class: "text-gray-600", "The plugin '{plugin_id}' is not currently loaded." }
                        button {
                            class: "mt-4 bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700",
                            onclick: move |_| {
                                let plugin_service = plugin_service.clone();
                                let plugin_id = plugin_id.clone();
                                spawn(async move {
                                    let service = plugin_service.read().await;
                                    if let Err(e) = service.install_plugin(&plugin_id).await {
                                        tracing::error!("Failed to load plugin: {}", e);
                                    }
                                    plugin_status.restart();
                                });
                            },
                            "Load Plugin"
                        }
                    }
                };
            }

            // Get available components from the plugin
            let component_id = get_plugin_component_id(&plugin_id, page.as_deref());

            rsx! {
                div {
                    class: "plugin-page p-6",
                    key: "{plugin_id}-{component_id}", // Force re-render when plugin changes
                    PluginComponentRenderer {
                        plugin_id: plugin_id.clone(),
                        component_id: component_id.to_string(),
                        props: serde_json::json!({
                            "plugin_id": plugin_id,
                            "page": page,
                            "full_page": true
                        })
                    }
                }
            }
        }
        Some(None) => rsx! {
            div { class: "text-center py-12",
                div { class: "text-6xl text-gray-400 mb-4", "🧩" }
                h2 { class: "text-2xl font-bold text-gray-900 mb-2", "Plugin Not Found" }
                p { class: "text-gray-600", "The plugin '{plugin_id}' could not be found." }
                p { class: "text-sm text-gray-500 mt-4", "Make sure the plugin is installed and loaded." }
            }
        },
        None => rsx! {
            div { class: "text-center py-12",
                div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto" }
                p { class: "mt-4 text-gray-600", "Loading plugin..." }
            }
        },
    }
}

/// Get the appropriate component ID for a plugin
fn get_plugin_component_id(plugin_id: &str, page: Option<&str>) -> &'static str {
    match plugin_id {
        "system_monitor" => "system_metrics",
        "notifications" => "notification_center",
        "product_catalog" => match page {
            Some("products") => "product_list",
            Some("categories") => "product_categories",
            _ => "product_list",
        },
        _ => "main", // Default component
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plugin_component_id() {
        assert_eq!(get_plugin_component_id("system_monitor", None), "system_metrics");
        assert_eq!(get_plugin_component_id("notifications", None), "notification_center");
        assert_eq!(
            get_plugin_component_id("product_catalog", Some("products")),
            "product_list"
        );
        assert_eq!(
            get_plugin_component_id("product_catalog", Some("categories")),
            "product_categories"
        );
        assert_eq!(get_plugin_component_id("unknown", None), "main");
    }

    #[tokio::test]
    async fn test_render_plugin_component() {
        let result = render_plugin_component(
            "test_plugin".to_string(),
            "test_component".to_string(),
            serde_json::json!({}),
        ).await;

        // Should return an error since the plugin doesn't exist
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Plugin 'test_plugin' not found"));
    }
}