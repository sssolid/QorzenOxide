// src/ui/router.rs
use crate::ui::{
    components::plugin_renderer::{PluginPageWrapper, PluginSettingsWrapper},
    layout::Layout,
    pages::{
        Dashboard as DashboardPage, Login as LoginPage, NotFound as NotFoundPage, PluginView,
        Plugins as PluginsPage, Profile as ProfilePage, Settings as SettingPage,
    },
    state::use_app_state,
};
use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

#[derive(Clone,Routable,Debug,PartialEq)]
#[rustfmt::skip]
pub enum Route{
    #[route("/login")]
    Login{},
    #[route("/")]
    #[redirect("/dashboard",||Route::Dashboard{})]
    Home{},
    #[route("/dashboard")]
    Dashboard{},
    #[route("/profile")]
    Profile{},
    #[route("/plugins")]
    Plugins{},
    #[route("/settings")]
    Settings{},
    #[route("/admin")]
    Admin{},
    #[route("/logs")]
    Logs{},
    #[route("/plugin/:plugin_id")]
    Plugin{plugin_id:String},
    #[route("/plugin/:plugin_id/:page")]
    PluginPage{plugin_id:String,page:String},
    #[route("/plugin/:plugin_id/settings")]
    PluginSettings{plugin_id:String},
    #[route("/plugins/:plugin_id/component/:component_id")]
    PluginComponent{plugin_id:String,component_id:String},
    #[route("/:..segments")]
    NotFound{segments:Vec<String>},
}

#[component]
pub fn Login() -> Element {
    rsx! {
        div{
            class:"min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8",
            LoginPage{}
        }
    }
}

#[component]
pub fn Home() -> Element {
    rsx! {
        AuthenticatedLayout{
            DashboardPage{}
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        AuthenticatedLayout{
            DashboardPage{}
        }
    }
}

#[component]
pub fn Profile() -> Element {
    rsx! {
        AuthenticatedLayout{
            ProfilePage{}
        }
    }
}

#[component]
pub fn Plugins() -> Element {
    rsx! {
        AuthenticatedLayout{
            PluginsPage{}
        }
    }
}

#[component]
pub fn Settings() -> Element {
    rsx! {
        AuthenticatedLayout{
            SettingPage{}
        }
    }
}

#[component]
pub fn Admin() -> Element {
    rsx! {
        AuthenticatedLayout{
            AdminPageWithPermissionCheck{}
        }
    }
}

#[component]
pub fn Logs() -> Element {
    rsx! {
        AuthenticatedLayout{
            LogsPage{}
        }
    }
}

#[component]
fn AdminPageWithPermissionCheck() -> Element {
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
                rsx! {crate::ui::pages::Admin{}}
            } else {
                rsx! {AccessDenied{}}
            }
        }
        None => {
            rsx! {AccessDenied{}}
        }
    }
}

#[component]
pub fn LogsPage() -> Element {
    rsx! {
        AuthenticatedLayout{
            crate::ui::pages::Logs{}
        }
    }
}

#[component]
fn AccessDenied() -> Element {
    rsx! {
        div{class:"text-center py-12",
            div{class:"text-6xl text-red-500 mb-4","🚫"}
            h1{class:"text-2xl font-bold text-gray-900 mb-2","Access Denied"}
            p{class:"text-gray-600 mb-6","You don't have permission to access this page."}
            Link{
                to:Route::Dashboard{},
                class:"inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                "Go to Dashboard"
            }
        }
    }
}

/// Plugin content page - shows the main plugin interface
#[component]
pub fn Plugin(plugin_id: String) -> Element {
    let key = format!("{}-content", plugin_id);
    rsx! {
        AuthenticatedLayout{
            key: key,
            PluginPageWrapper{
                plugin_id:plugin_id,
                page:None
            }
        }
    }
}

/// Plugin page with specific page parameter
#[component]
pub fn PluginPage(plugin_id: String, page: String) -> Element {
    let key = format!("{}-page", plugin_id);
    rsx! {
        AuthenticatedLayout{
            key: key,
            PluginPageWrapper{
                plugin_id:plugin_id,
                page:Some(page)
            }
        }
    }
}

/// Plugin component renderer
#[component]
pub fn PluginComponent(plugin_id: String, component_id: String) -> Element {
    rsx! {
        AuthenticatedLayout{
            crate::ui::components::plugin_renderer::PluginComponentRenderer{
                plugin_id,
                component_id,
                props:serde_json::json!({})
            }
        }
    }
}

/// Plugin settings page - shows configuration interface
#[component]
pub fn PluginSettings(plugin_id: String) -> Element {
    let key = format!("{}-settings", plugin_id);
    rsx! {
        AuthenticatedLayout{
            key: key,
            PluginSettingsWrapper{
                plugin_id:plugin_id
            }
        }
    }
}

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");

    rsx! {
        div{
            class:"min-h-screen flex items-center justify-center bg-gray-50",
            NotFoundPage{path:path}
        }
    }
}

#[component]
pub fn AuthenticatedLayout(children: Element) -> Element {
    let app_state = use_app_state();
    let navigator = use_navigator();

    if let Some(_user) = &app_state.current_user {
        rsx! {
            Layout{
                {children}
            }
        }
    } else {
        navigator.push(Route::Login {});
        rsx! {
            div{class:"min-h-screen flex items-center justify-center bg-gray-50",
                div{class:"animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600"}
                p{class:"mt-4 text-gray-600","Redirecting to login..."}
            }
        }
    }
}

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
            let direct_permission = user.permissions.iter().any(|perm| {
                (perm.resource == resource || perm.resource == "*")
                    && (perm.action == action || perm.action == "*")
            });

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
        rsx! {{children}}
    } else {
        match fallback {
            Some(fallback_element) => rsx! {{fallback_element}},
            None => rsx! {
                div{class:"text-center py-8",
                    div{class:"text-4xl text-gray-400 mb-2","🔒"}
                    p{class:"text-gray-600","Insufficient permissions"}
                }
            },
        }
    }
}

pub mod nav {
    use super::*;

    pub fn is_active_route(current: &Route, target: &Route) -> bool {
        std::mem::discriminant(current) == std::mem::discriminant(target)
    }

    pub fn route_title(route: &Route) -> &'static str {
        match route {
            Route::Login { .. } => "Login",
            Route::Home { .. } => "Home",
            Route::Dashboard { .. } => "Dashboard",
            Route::Profile { .. } => "Profile",
            Route::Plugins { .. } => "Plugins",
            Route::Settings { .. } => "Settings",
            Route::Admin { .. } => "Admin",
            Route::Logs { .. } => "Logs",
            Route::Plugin { .. } => "Plugin",
            Route::PluginPage { .. } => "Plugin Page",
            Route::PluginComponent { .. } => "Plugin Component",
            Route::PluginSettings { .. } => "Plugin Settings",
            Route::NotFound { .. } => "Not Found",
        }
    }

    pub fn route_icon(route: &Route) -> &'static str {
        match route {
            Route::Login { .. } => "🔐",
            Route::Home { .. } => "🏠",
            Route::Dashboard { .. } => "📊",
            Route::Profile { .. } => "👤",
            Route::Plugins { .. } => "🧩",
            Route::Settings { .. } => "⚙️",
            Route::Admin { .. } => "👑",
            Route::Logs { .. } => "📋",
            Route::Plugin { .. } => "🔌",
            Route::PluginPage { .. } => "📄",
            Route::PluginComponent { .. } => "🧩",
            Route::PluginSettings { .. } => "⚙️",
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
