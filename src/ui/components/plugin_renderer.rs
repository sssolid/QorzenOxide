use dioxus::prelude::*;
use crate::plugin::PluginFactoryRegistry;
use crate::ui::state::use_app_state;

/// Renders a plugin component dynamically
#[component]
pub fn PluginComponentRenderer(
    plugin_id: String,
    component_id: String,
    #[props(default = serde_json::Value::Object(serde_json::Map::new()))]
    props: serde_json::Value,
) -> Element {
    let app_state = use_app_state();

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
    props: serde_json::Value
) -> Result<String, String> {
    match PluginFactoryRegistry::create_plugin(&plugin_id).await {
        Some(plugin) => {
            match plugin.render_component(&component_id, props) {
                Ok(vnode) => {
                    // Convert VNode to HTML string
                    // This is a simplified implementation - in practice, you'd need
                    // a proper VNode to HTML serializer
                    Ok(format!("<div>Plugin component: {} - {}</div>", plugin_id, component_id))
                }
                Err(e) => Err(format!("Failed to render component: {}", e))
            }
        }
        None => Err(format!("Plugin '{}' not found", plugin_id))
    }
}

/// Wrapper for plugin pages
#[component]
pub fn PluginPageWrapper(
    plugin_id: String,
    page: Option<String>,
) -> Element {
    let app_state = use_app_state();

    let plugin_info = use_resource({
        let plugin_id = plugin_id.clone();
        move || {
            let plugin_id = plugin_id.clone();
            async move {
                PluginFactoryRegistry::get_plugin_info(&plugin_id).await
            }
        }
    });

    match &*plugin_info.read_unchecked() {
        Some(Some(info)) => {
            let page_component_id = page.as_deref().unwrap_or("main");

            rsx! {
                div { class: "plugin-page",
                    // Plugin header
                    div { class: "mb-6 bg-white shadow rounded-lg p-6",
                        div { class: "flex items-center",
                            span { class: "text-4xl mr-4", "🧩" }
                            div {
                                h1 { class: "text-2xl font-bold text-gray-900", "{info.name}" }
                                p { class: "text-gray-600", "v{info.version} by {info.author}" }
                                p { class: "text-sm text-gray-500 mt-1", "{info.description}" }
                            }
                        }
                    }

                    // Plugin component content
                    PluginComponentRenderer {
                        plugin_id: plugin_id.clone(),
                        component_id: page_component_id.to_string(),
                        props: serde_json::json!({
                            "plugin_id": plugin_id,
                            "page": page
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
            }
        },
        None => rsx! {
            div { class: "text-center py-12",
                div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto" }
                p { class: "mt-4 text-gray-600", "Loading plugin..." }
            }
        }
    }
}