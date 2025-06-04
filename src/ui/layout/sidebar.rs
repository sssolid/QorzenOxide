// src/ui/layout/sidebar.rs - Navigation sidebar with actual plugin integration

use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::ui::{
    router::{nav, Route},
    state::auth::use_has_permission,
};

/// Sidebar component props
#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    /// Whether the sidebar is collapsed on desktop
    pub collapsed: bool,
    /// Whether the mobile menu is open
    pub mobile_open: bool,
    /// Callback for closing mobile menu
    pub on_close: Callback<Event<MouseData>>,
}

/// Navigation item definition
#[derive(Debug, Clone, PartialEq)]
pub struct NavItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub route: Option<Route>,
    pub children: Vec<NavItem>,
    pub required_permission: Option<(String, String)>, // (resource, action)
    pub badge: Option<String>,
    pub external_url: Option<String>,
}

/// Main sidebar component
#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    // let app_state = use_app_state();
    let current_route = use_route::<Route>();
    let has_permission = use_has_permission();

    // Navigation items configuration
    let nav_items = get_navigation_items();

    // Filter navigation items based on user permissions
    let filtered_nav_items = nav_items
        .into_iter()
        .filter(|item| {
            if let Some((resource, action)) = &item.required_permission {
                has_permission(resource, action)
            } else {
                true
            }
        })
        .collect::<Vec<_>>();

    rsx! {
        // Desktop sidebar
        div {
            class: format!(
                "hidden lg:flex lg:flex-col lg:fixed lg:inset-y-0 lg:z-40 lg:transition-all lg:duration-200 lg:ease-in-out {}",
                if props.collapsed { "lg:w-16" } else { "lg:w-64" }
            ),
            div {
                class: "flex flex-col flex-grow bg-white border-r border-gray-200 pt-16 pb-4 overflow-y-auto",
                nav {
                    class: "flex-1 px-2 space-y-1",
                    for item in &filtered_nav_items {
                        NavigationItem {
                            key: "{item.id}",
                            item: item.clone(),
                            collapsed: props.collapsed,
                            current_route: current_route.clone()
                        }
                    }
                }

                // Plugin section
                if !props.collapsed {
                    div {
                        class: "px-2 mt-6",
                        div {
                            class: "text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2",
                            "Plugins"
                        }
                        PluginNavigation {}
                    }
                }
            }
        }

        // Mobile sidebar
        if props.mobile_open {
            div {
                class: "lg:hidden fixed inset-0 z-50 flex",

                // Sidebar
                div {
                    class: "relative flex flex-col flex-1 w-64 bg-white",

                    // Close button
                    div {
                        class: "absolute top-0 right-0 -mr-12 pt-2",
                        button {
                            r#type: "button",
                            class: "ml-1 flex items-center justify-center h-10 w-10 rounded-full focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white",
                            onclick: move |e| props.on_close.call(e),
                            span {
                                class: "sr-only",
                                "Close sidebar"
                            }
                            // X icon
                            svg {
                                class: "h-6 w-6 text-white",
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M6 18L18 6M6 6l12 12"
                                }
                            }
                        }
                    }

                    // Mobile navigation
                    div {
                        class: "flex-1 h-0 pt-5 pb-4 overflow-y-auto",
                        nav {
                            class: "px-2 space-y-1",
                            for item in &filtered_nav_items {
                                NavigationItem {
                                    key: "{item.id}",
                                    item: item.clone(),
                                    collapsed: false,
                                    current_route: current_route.clone(),
                                    on_click: Some(props.on_close)
                                }
                            }
                        }

                        // Mobile plugin section
                        div {
                            class: "px-2 mt-6",
                            div {
                                class: "text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2",
                                "Plugins"
                            }
                            PluginNavigation {
                                on_click: Some(props.on_close)
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Individual navigation item component
#[component]
fn NavigationItem(
    item: NavItem,
    collapsed: bool,
    current_route: Route,
    #[props(default = None)] on_click: Option<Callback<Event<MouseData>>>,
) -> Element {
    let is_active = item
        .route
        .as_ref()
        .map(|route| nav::is_active_route(&current_route, route))
        .unwrap_or(false);

    // If item has children, render as expandable group
    if !item.children.is_empty() {
        let mut expanded = use_signal(|| false);

        rsx! {
            div {
                // Group header
                button {
                    r#type: "button",
                    class: format!(
                        "group w-full flex items-center px-2 py-2 text-sm font-medium rounded-md text-gray-600 hover:bg-gray-50 hover:text-gray-900 {}",
                        if collapsed { "justify-center" } else { "justify-between" }
                    ),
                    onclick: move |_| {
                        if !collapsed {
                            expanded.set(!expanded());
                        }
                    },

                    div {
                        class: "flex items-center",
                        span {
                            class: "text-lg mr-3",
                            "{item.icon}"
                        }
                        if !collapsed {
                            span { "{item.label}" }
                        }
                    }

                    if !collapsed && !item.children.is_empty() {
                        // Expand/collapse icon
                        svg {
                            class: format!(
                                "h-4 w-4 transition-transform {}",
                                if expanded() { "transform rotate-90" } else { "" }
                            ),
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 111.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                }

                // Children (when not collapsed and expanded)
                if !collapsed && expanded() {
                    div {
                        class: "ml-6 space-y-1",
                        for child in &item.children {
                            NavigationItem {
                                key: "{child.id}",
                                item: child.clone(),
                                collapsed: false,
                                current_route: current_route.clone(),
                                on_click: on_click
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Regular navigation item
        rsx! {
            if let Some(route) = &item.route {
                Link {
                    to: route.clone(),
                    class: format!(
                        "group flex items-center px-2 py-2 text-sm font-medium rounded-md {} {}",
                        if is_active {
                            "bg-blue-50 border-r-4 border-blue-600 text-blue-700"
                        } else {
                            "text-gray-600 hover:bg-gray-50 hover:text-gray-900"
                        },
                        if collapsed { "justify-center" } else { "" }
                    ),
                    onclick: move |e| {
                        if let Some(callback) = &on_click {
                            callback.call(e);
                        }
                    },
                    span {
                        class: format!(
                            "text-lg {}",
                            if collapsed { "" } else { "mr-3" }
                        ),
                        "{item.icon}"
                    }
                    if !collapsed {
                        span {
                            class: "flex-1",
                            "{item.label}"
                        }
                        if let Some(badge) = &item.badge {
                            span {
                                class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800",
                                "{badge}"
                            }
                        }
                    }
                }
            } else if let Some(url) = &item.external_url {
                a {
                    href: "{url}",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    class: format!(
                        "group flex items-center px-2 py-2 text-sm font-medium rounded-md text-gray-600 hover:bg-gray-50 hover:text-gray-900 {}",
                        if collapsed { "justify-center" } else { "" }
                    ),
                    span {
                        class: format!(
                            "text-lg {}",
                            if collapsed { "" } else { "mr-3" }
                        ),
                        "{item.icon}"
                    }
                    if !collapsed {
                        span {
                            class: "flex-1",
                            "{item.label}"
                        }
                        // External link icon
                        svg {
                            class: "h-4 w-4 text-gray-400",
                            xmlns: "http://www.w3.org/2000/svg",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Plugin navigation section - now uses actual plugin data
#[component]
fn PluginNavigation(
    #[props(default = None)] on_click: Option<Callback<Event<MouseData>>>,
) -> Element {
    // Get actual plugin information from registry
    let plugin_menu_items = use_resource(|| async move {
        get_plugin_menu_items().await
    });

    match &*plugin_menu_items.read_unchecked() {
        Some(Ok(items)) => {
            if items.is_empty() {
                rsx! {
                    div {
                        class: "text-sm text-gray-500 italic px-2 py-2",
                        "No plugins with menu items"
                    }
                }
            } else {
                rsx! {
                    div {
                        class: "space-y-1",
                        for item in items {
                            PluginMenuItem {
                                key: "{item.id}",
                                item: item.clone(),
                                on_click: on_click
                            }
                        }
                    }
                }
            }
        }
        Some(Err(error)) => rsx! {
            div {
                class: "text-sm text-red-500 px-2 py-2",
                "Error loading plugins: {error}"
            }
        },
        None => rsx! {
            div {
                class: "text-sm text-gray-500 px-2 py-2",
                "Loading plugins..."
            }
        }
    }
}

/// Individual plugin menu item component
#[component]
fn PluginMenuItem(
    item: crate::plugin::MenuItem,
    #[props(default = None)] on_click: Option<Callback<Event<MouseData>>>,
) -> Element {
    // Extract shared values before rsx
    let icon = item.icon.clone().unwrap_or_else(|| "🧩".to_string());

    if let Some(route_str) = &item.route {
        if route_str.starts_with("/plugins/") {
            let plugin_id = route_str.strip_prefix("/plugins/").unwrap_or("unknown").to_string();
            rsx! {
                Link {
                    to: Route::Plugin { plugin_id },
                    class: "group flex items-center px-2 py-2 text-sm font-medium rounded-md text-gray-600 hover:bg-gray-50 hover:text-gray-900",
                    onclick: move |e| {
                        if let Some(callback) = &on_click {
                            callback.call(e);
                        }
                    },
                    span {
                        class: "text-lg mr-3",
                        "{icon}"
                    }
                    "{item.label}"
                }
            }
        } else {
            rsx! {
                a {
                    href: route_str.clone(),
                    class: "group flex items-center px-2 py-2 text-sm font-medium rounded-md text-gray-600 hover:bg-gray-50 hover:text-gray-900",
                    span {
                        class: "text-lg mr-3",
                        "{icon}"
                    }
                    "{item.label}"
                }
            }
        }
    } else if let Some(action) = &item.action {
        let action = action.clone();
        rsx! {
            button {
                r#type: "button",
                class: "group w-full flex items-center px-2 py-2 text-sm font-medium rounded-md text-gray-600 hover:bg-gray-50 hover:text-gray-900",
                onclick: move |_| {
                    tracing::info!("Plugin action triggered: {}", action);
                },
                span {
                    class: "text-lg mr-3",
                    "{icon}"
                }
                "{item.label}"
            }
        }
    } else {
        rsx! { Fragment {} }
    }
}

/// Get menu items from loaded plugins
async fn get_plugin_menu_items() -> Result<Vec<crate::plugin::MenuItem>, String> {
    // Get all registered plugins
    let plugin_infos = crate::plugin::PluginFactoryRegistry::get_all_plugin_info().await;

    let mut all_menu_items = Vec::new();

    // For each plugin, try to create it and get its menu items
    for plugin_info in plugin_infos {
        match crate::plugin::PluginFactoryRegistry::create_plugin(&plugin_info.id).await {
            Some(plugin) => {
                let menu_items = plugin.menu_items();
                all_menu_items.extend(menu_items);
            }
            None => {
                tracing::warn!("Could not create plugin instance for: {}", plugin_info.id);
            }
        }
    }

    // Sort by order
    all_menu_items.sort_by(|a, b| a.order.cmp(&b.order));

    Ok(all_menu_items)
}

/// Get the navigation items configuration
fn get_navigation_items() -> Vec<NavItem> {
    vec![
        NavItem {
            id: "dashboard".to_string(),
            label: "Dashboard".to_string(),
            icon: "📊".to_string(),
            route: Some(Route::Dashboard {}),
            children: vec![],
            required_permission: None,
            badge: None,
            external_url: None,
        },
        NavItem {
            id: "profile".to_string(),
            label: "Profile".to_string(),
            icon: "👤".to_string(),
            route: Some(Route::Profile {}),
            children: vec![],
            required_permission: None,
            badge: None,
            external_url: None,
        },
        NavItem {
            id: "plugins".to_string(),
            label: "Plugins".to_string(),
            icon: "🧩".to_string(),
            route: Some(Route::Plugins {}),
            children: vec![],
            required_permission: None, // Some(("plugins".to_string(), "read".to_string())),
            badge: None,
            external_url: None,
        },
        NavItem {
            id: "settings".to_string(),
            label: "Settings".to_string(),
            icon: "⚙️".to_string(),
            route: Some(Route::Settings {}),
            children: vec![],
            required_permission: Some(("settings".to_string(), "read".to_string())),
            badge: None,
            external_url: None,
        },
        NavItem {
            id: "admin".to_string(),
            label: "Administration".to_string(),
            icon: "👑".to_string(),
            route: Some(Route::Admin {}),
            children: vec![],
            required_permission: Some(("admin".to_string(), "read".to_string())),
            badge: Some("Admin".to_string()),
            external_url: None,
        },
        NavItem {
            id: "help".to_string(),
            label: "Help & Support".to_string(),
            icon: "❓".to_string(),
            route: None,
            children: vec![
                NavItem {
                    id: "documentation".to_string(),
                    label: "Documentation".to_string(),
                    icon: "📚".to_string(),
                    route: None,
                    children: vec![],
                    required_permission: None,
                    badge: None,
                    external_url: Some("https://docs.qorzen.com".to_string()),
                },
                NavItem {
                    id: "support".to_string(),
                    label: "Contact Support".to_string(),
                    icon: "💬".to_string(),
                    route: None,
                    children: vec![],
                    required_permission: None,
                    badge: None,
                    external_url: Some("mailto:support@qorzen.com".to_string()),
                },
            ],
            required_permission: None,
            badge: None,
            external_url: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_items() {
        let items = get_navigation_items();
        assert!(!items.is_empty());

        // Check that dashboard item exists
        let dashboard = items.iter().find(|item| item.id == "dashboard");
        assert!(dashboard.is_some());
        assert_eq!(dashboard.unwrap().label, "Dashboard");
    }

    #[test]
    fn test_nav_item_with_children() {
        let items = get_navigation_items();
        let help_item = items.iter().find(|item| item.id == "help");
        assert!(help_item.is_some());
        assert!(!help_item.unwrap().children.is_empty());
    }

    #[test]
    fn test_sidebar_component_creation() {
        let on_close = Callback::new(|_| {});

        let _sidebar = rsx! {
            Sidebar {
                collapsed: false,
                mobile_open: false,
                on_close: on_close
            }
        };
    }
}