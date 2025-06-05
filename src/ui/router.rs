// src/ui/router.rs - Routing configuration with authentication guards

use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::ui::{
    layout::Layout,
    pages::{
        Dashboard as DashboardPage, Login as LoginPage, NotFound as NotFoundPage,
        Plugins as PluginsPage, Profile as ProfilePage, Settings as SettingPage,
    },
    state::use_app_state,
};

/// Application routes with authentication and authorization
#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    // Public routes
    #[route("/login")]
    Login {},

    // Protected routes (require authentication)
    #[route("/")]
    #[redirect("/dashboard", || Route::Dashboard {})]
    Home {},

    #[route("/dashboard")]
    Dashboard {},

    #[route("/profile")]
    Profile {},

    #[route("/plugins")]
    Plugins {},

    #[route("/settings")]
    Settings {},

    #[route("/admin")]
    Admin {},

    // Plugin routes (dynamically loaded)
    #[route("/plugin/:plugin_id")]
    Plugin { plugin_id: String },

    #[route("/plugin/:plugin_id/:page")]
    PluginPage { plugin_id: String, page: String },

    #[route("/plugins/:plugin_id/component/:component_id")]
    PluginComponent { plugin_id: String, component_id: String },

    // Catch-all for 404
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

/// Route component implementations
#[component]
pub fn Login() -> Element {
    rsx! {
        div {
            class: "min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8",
            LoginPage {}
        }
    }
}

#[component]
pub fn Home() -> Element {
    rsx! {
        AuthenticatedLayout {
            DashboardPage {}
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        AuthenticatedLayout {
            DashboardPage {}
        }
    }
}

#[component]
pub fn Profile() -> Element {
    rsx! {
        AuthenticatedLayout {
            ProfilePage {}
        }
    }
}

#[component]
pub fn Plugins() -> Element {
    rsx! {
        AuthenticatedLayout {
            PluginsPage {}
        }
    }
}

#[component]
pub fn Settings() -> Element {
    rsx! {
        AuthenticatedLayout {
            SettingPage {}
        }
    }
}

#[component]
pub fn Admin() -> Element {
    rsx! {
        AuthenticatedLayout {
            AdminPageWithPermissionCheck {}
        }
    }
}

/// Admin page with permission checking
#[component]
fn AdminPageWithPermissionCheck() -> Element {
    // Check if user has admin permissions
    let app_state = use_app_state();

    match &app_state.current_user {
        Some(user) => {
            let has_admin_permission = user.roles.iter().any(|role| {
                role.id == "admin"
                    || role
                        .permissions
                        .iter()
                        .any(|perm| perm.resource == "admin" && perm.action == "*")
            });

            if has_admin_permission {
                rsx! {
                    crate::ui::pages::Admin {}
                }
            } else {
                rsx! {
                    AccessDenied {}
                }
            }
        }
        None => {
            rsx! {
                AccessDenied {}
            }
        }
    }
}

/// Access denied component
#[component]
fn AccessDenied() -> Element {
    rsx! {
        div {
            class: "text-center py-12",
            div {
                class: "text-6xl text-red-500 mb-4",
                "🚫"
            }
            h1 {
                class: "text-2xl font-bold text-gray-900 mb-2",
                "Access Denied"
            }
            p {
                class: "text-gray-600 mb-6",
                "You don't have permission to access this page."
            }
            Link {
                to: Route::Dashboard {},
                class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                "Go to Dashboard"
            }
        }
    }
}

#[component]
pub fn Plugin(plugin_id: String) -> Element {
    rsx! {
        AuthenticatedLayout {
            crate::ui::pages::PluginView {
                plugin_id: plugin_id
            }
        }
    }
}

#[component]
pub fn PluginPage(plugin_id: String, page: String) -> Element {
    rsx! {
        AuthenticatedLayout {
            crate::ui::components::plugin_renderer::PluginPageWrapper {
                plugin_id,
                page: Some(page)
            }
        }
    }
}

#[component]
pub fn PluginComponent(plugin_id: String, component_id: String) -> Element {
    rsx! {
        AuthenticatedLayout {
            crate::ui::components::plugin_renderer::PluginComponentRenderer {
                plugin_id,
                component_id,
                props: serde_json::json!({})
            }
        }
    }
}

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");

    rsx! {
        div {
            class: "min-h-screen flex items-center justify-center bg-gray-50",
            NotFoundPage {
                path: path
            }
        }
    }
}

/// Authenticated layout wrapper that checks authentication before rendering
#[component]
pub fn AuthenticatedLayout(children: Element) -> Element {
    let app_state = use_app_state();
    let navigator = use_navigator();

    // Check if user is authenticated
    if let Some(_user) = &app_state.current_user {
        rsx! {
            Layout {
                {children}
            }
        }
    } else {
        // Redirect to login immediately (not in an effect)
        navigator.push(Route::Login {});

        rsx! {
            div {
                class: "min-h-screen flex items-center justify-center bg-gray-50",
                div {
                    class: "animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600"
                }
                p {
                    class: "mt-4 text-gray-600",
                    "Redirecting to login..."
                }
            }
        }
    }
}

/// Permission guard component
#[component]
pub fn PermissionGuard(
    resource: String,
    action: String,
    fallback: Option<Element>,
    children: Element,
) -> Element {
    let app_state = use_app_state();

    let has_permission = match &app_state.current_user {
        Some(user) => {
            // Check direct permissions
            let direct_permission = user.permissions.iter().any(|perm| {
                (perm.resource == resource || perm.resource == "*")
                    && (perm.action == action || perm.action == "*")
            });

            // Check role permissions
            let role_permission = user.roles.iter().any(|role| {
                role.permissions.iter().any(|perm| {
                    (perm.resource == resource || perm.resource == "*")
                        && (perm.action == action || perm.action == "*")
                })
            });

            direct_permission || role_permission
        }
        None => false,
    };

    if has_permission {
        rsx! { {children} }
    } else {
        match fallback {
            Some(fallback_element) => rsx! { {fallback_element} },
            None => rsx! {
                div {
                    class: "text-center py-8",
                    div {
                        class: "text-4xl text-gray-400 mb-2",
                        "🔒"
                    }
                    p {
                        class: "text-gray-600",
                        "Insufficient permissions"
                    }
                }
            },
        }
    }
}

/// Navigation utilities
pub mod nav {
    use super::*;

    /// Check if current route matches the given route
    pub fn is_active_route(current: &Route, target: &Route) -> bool {
        std::mem::discriminant(current) == std::mem::discriminant(target)
    }

    /// Get route title for display
    pub fn route_title(route: &Route) -> &'static str {
        match route {
            Route::Login { .. } => "Login",
            Route::Home { .. } => "Home",
            Route::Dashboard { .. } => "Dashboard",
            Route::Profile { .. } => "Profile",
            Route::Plugins { .. } => "Plugins",
            Route::Settings { .. } => "Settings",
            Route::Admin { .. } => "Admin",
            Route::Plugin { .. } => "Plugin",
            Route::PluginPage { .. } => "Plugin Page",
            Route::PluginComponent { .. } => "Plugin Component",
            Route::NotFound { .. } => "Not Found",
        }
    }

    /// Get route icon (for navigation menus)
    pub fn route_icon(route: &Route) -> &'static str {
        match route {
            Route::Login { .. } => "🔐",
            Route::Home { .. } => "🏠",
            Route::Dashboard { .. } => "📊",
            Route::Profile { .. } => "👤",
            Route::Plugins { .. } => "🧩",
            Route::Settings { .. } => "⚙️",
            Route::Admin { .. } => "👑",
            Route::Plugin { .. } => "🔌",
            Route::PluginPage { .. } => "📄",
            Route::PluginComponent { .. } => "🧩",
            Route::NotFound { .. } => "❓",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_equality() {
        let route1 = Route::Dashboard {};
        let route2 = Route::Dashboard {};
        assert_eq!(route1, route2);
    }

    #[test]
    fn test_route_title() {
        assert_eq!(nav::route_title(&Route::Dashboard {}), "Dashboard");
        assert_eq!(nav::route_title(&Route::Profile {}), "Profile");
    }

    #[test]
    fn test_route_icon() {
        assert_eq!(nav::route_icon(&Route::Dashboard {}), "📊");
        assert_eq!(nav::route_icon(&Route::Settings {}), "⚙️");
    }
}
