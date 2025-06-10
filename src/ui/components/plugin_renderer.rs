// src/ui/components/plugin_renderer.rs
use crate::plugin::PluginFactoryRegistry;
use crate::ui::services::plugin_service::use_plugin_service;
use dioxus::prelude::*;

/// Renders a specific component from a plugin
#[component]
pub fn PluginComponentRenderer(
    plugin_id: String,
    component_id: String,
    #[props(default=serde_json::Value::Object(serde_json::Map::new()))] props: serde_json::Value,
) -> Element {
    let mut current_plugin_id = use_signal(|| plugin_id.clone());
    let mut current_component_id = use_signal(|| component_id.clone());

    // Update signals when props change - using dependencies
    use_effect(use_reactive((&plugin_id, &component_id), move |(plugin_id, component_id)| {
        current_plugin_id.set(plugin_id.clone());
        current_component_id.set(component_id.clone());
    }));

    let component_content = use_resource({
        let plugin_id = current_plugin_id();
        let component_id = current_component_id();
        let props = props.clone();
        move || {
            let plugin_id = plugin_id.clone();
            let component_id = component_id.clone();
            let props = props.clone();
            async move { render_plugin_component(plugin_id, component_id, props).await }
        }
    });

    match &*component_content.read_unchecked() {
        Some(Ok(content)) => rsx! {
            div{
                class:"plugin-component",
                dangerous_inner_html:"{content}"
            }
        },
        Some(Err(e)) => rsx! {
            div{
                class:"text-red-500 p-4 border border-red-200 rounded-md bg-red-50",
                div{class:"flex items-center",
                    span{class:"text-xl mr-2","⚠️"}
                    strong{"Plugin Component Error"}
                }
                p{class:"mt-2 text-sm","{e}"}
            }
        },
        None => rsx! {
            div{class:"flex items-center justify-center p-8",
                div{class:"animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"}
                span{class:"ml-3 text-gray-600","Loading plugin component..."}
            }
        },
    }
}

async fn render_plugin_component(
    plugin_id: String,
    component_id: String,
    props: serde_json::Value,
) -> Result<String, String> {
    match PluginFactoryRegistry::create_plugin(&plugin_id).await {
        Some(plugin) => match plugin.render_component(&component_id, props) {
            Ok(_vnode) => Ok(format!(
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
            )),
            Err(e) => Err(format!("Failed to render component: {}", e)),
        },
        None => Err(format!("Plugin '{}' not found", plugin_id)),
    }
}

/// Wrapper component for plugin pages that handles loading and routing
#[component]
pub fn PluginPageWrapper(plugin_id: String, page: Option<String>) -> Element {
    let plugin_service = use_plugin_service();
    let mut current_plugin_id = use_signal(|| plugin_id.clone());
    let mut current_page = use_signal(|| page.clone());

    // Update signals when props change and restart resources
    let mut plugin_info = use_resource({
        let plugin_id = plugin_id.clone();
        move || {
            let plugin_id = plugin_id.clone();
            async move { PluginFactoryRegistry::get_plugin_info(&plugin_id).await }
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

    // Restart resources when plugin ID changes
    use_effect({
        let plugin_id = plugin_id.clone();
        let page = page.clone();
        move || {
            if current_plugin_id() != plugin_id || current_page() != page {
                current_plugin_id.set(plugin_id.clone());
                current_page.set(page.clone());
                plugin_info.restart();
                plugin_status.restart();
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
                    div{class:"text-center py-12",
                        div{class:"text-6xl text-yellow-500 mb-4","⚠️"}
                        h2{class:"text-2xl font-bold text-gray-900 mb-2","Plugin Not Loaded"}
                        p{class:"text-gray-600","The plugin '{plugin_id}' is not currently loaded."}
                        button{
                            class:"mt-4 bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700",
                            onclick:move|_|{
                                let plugin_service=plugin_service.clone();
                                let plugin_id=plugin_id.clone();
                                spawn(async move{
                                    let service=plugin_service.read().await;
                                    if let Err(e)=service.install_plugin(&plugin_id).await{
                                        tracing::error!("Failed to load plugin: {}",e);
                                    }
                                    plugin_status.restart();
                                });
                            },
                            "Load Plugin"
                        }
                    }
                };
            }

            let component_id = get_plugin_component_id(&plugin_id, page.as_deref());

            rsx! {
                div {
                    class: "plugin-page p-6",
                    key: "{plugin_id}-{component_id}-{page.as_deref().unwrap_or(\"default\")}",
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
            div{class:"text-center py-12",
                div{class:"text-6xl text-gray-400 mb-4","🧩"}
                h2{class:"text-2xl font-bold text-gray-900 mb-2","Plugin Not Found"}
                p{class:"text-gray-600","The plugin '{plugin_id}' could not be found."}
                p{class:"text-sm text-gray-500 mt-4","Make sure the plugin is installed and loaded."}
            }
        },
        None => rsx! {
            div{class:"text-center py-12",
                div{class:"animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"}
                p{class:"mt-4 text-gray-600","Loading plugin..."}
            }
        },
    }
}

/// Wrapper for plugin settings pages that shows configuration interface
#[component]
pub fn PluginSettingsWrapper(plugin_id: String) -> Element {
    let plugin_service = use_plugin_service();
    let mut current_plugin_id = use_signal(|| plugin_id.clone());

    let mut plugin_info = use_resource({
        let plugin_id = plugin_id.clone();
        move || {
            let plugin_id = plugin_id.clone();
            async move { PluginFactoryRegistry::get_plugin_info(&plugin_id).await }
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

    // Restart resources when plugin ID changes
    use_effect({
        let plugin_id = plugin_id.clone();
        move || {
            if current_plugin_id() != plugin_id {
                current_plugin_id.set(plugin_id.clone());
                plugin_info.restart();
                plugin_status.restart();
            }
        }
    });

    match &*plugin_info.read_unchecked() {
        Some(Some(info)) => {
            rsx! {
                div{class:"plugin-settings-page p-6",
                    div{class:"bg-white shadow rounded-lg p-6",
                        div{class:"flex items-center mb-6",
                            span{class:"text-4xl mr-4","{get_plugin_icon(&info.id)}"}
                            div{
                                h2{class:"text-2xl font-semibold text-gray-900","{info.name} Settings"}
                                p{class:"text-gray-600","Configure {info.name} plugin settings"}
                                p{class:"text-sm text-gray-500 mt-1","v{info.version} by {info.author}"}
                            }
                        }

                        div{class:"grid grid-cols-1 md:grid-cols-2 gap-6",
                            div{class:"p-4 bg-gray-50 rounded-lg",
                                h3{class:"font-medium text-gray-900 mb-2","Plugin Information"}
                                dl{class:"space-y-2 text-sm",
                                    div{class:"flex justify-between",
                                        dt{class:"text-gray-500","ID:"}
                                        dd{class:"text-gray-900","{info.id}"}
                                    }
                                    div{class:"flex justify-between",
                                        dt{class:"text-gray-500","Version:"}
                                        dd{class:"text-gray-900","{info.version}"}
                                    }
                                    div{class:"flex justify-between",
                                        dt{class:"text-gray-500","Author:"}
                                        dd{class:"text-gray-900","{info.author}"}
                                    }
                                }
                            }
                            div{class:"p-4 bg-blue-50 rounded-lg",
                                h3{class:"font-medium text-gray-900 mb-2","Settings"}
                                p{class:"text-sm text-blue-800","🔧 Plugin-specific settings interface would be rendered here."}
                                p{class:"text-xs text-blue-600 mt-2","This would include configuration options, preferences, and plugin-specific controls."}
                            }
                        }

                        div{class:"mt-6 flex justify-end space-x-3",
                            button{
                                r#type:"button",
                                class:"px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                "Reset to Defaults"
                            }
                            button{
                                r#type:"button",
                                class:"px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                "Save Settings"
                            }
                        }
                    }
                }
            }
        }
        Some(None) => rsx! {
            div{class:"text-center py-12",
                div{class:"text-6xl text-gray-400 mb-4","🧩"}
                h2{class:"text-2xl font-bold text-gray-900 mb-2","Plugin Not Found"}
                p{class:"text-gray-600","The plugin '{plugin_id}' could not be found."}
            }
        },
        None => rsx! {
            div{class:"text-center py-12",
                div{class:"animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"}
                p{class:"mt-4 text-gray-600","Loading plugin settings..."}
            }
        },
    }
}

fn get_plugin_component_id(plugin_id: &str, page: Option<&str>) -> &'static str {
    match plugin_id {
        "system_monitor" => "system_metrics",
        "notifications" => "notification_center",
        "product_catalog" => match page {
            Some("products") => "product_list",
            Some("categories") => "product_categories",
            _ => "product_list",
        },
        _ => "main",
    }
}

fn get_plugin_icon(plugin_id: &str) -> String {
    match plugin_id {
        "system_monitor" => "🖥️".to_string(),
        "notifications" => "🔔".to_string(),
        "product_catalog" => "📦".to_string(),
        _ => "🧩".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plugin_component_id() {
        assert_eq!(
            get_plugin_component_id("system_monitor", None),
            "system_metrics"
        );
        assert_eq!(
            get_plugin_component_id("notifications", None),
            "notification_center"
        );
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
        )
        .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Plugin 'test_plugin' not found"));
    }
}
