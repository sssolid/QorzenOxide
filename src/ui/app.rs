// src/ui/app.rs - Main application component with routing

use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::ui::components::plugin_manager_provider::PluginManagerProvider;

use crate::ui::{
    layout::Layout,
    pages::{Dashboard, Login, NotFound, Profile},
    router::Route,
    state::AppStateProvider,
};

/// Main application component that sets up routing and global state
#[component]
pub fn App() -> Element {
    rsx! {
        AppStateProvider {
            PluginManagerProvider {
                Router::<Route> {}
            }
        }
    }
}

/// Root layout component that wraps all routes
#[component]
fn RootLayout() -> Element {
    rsx! {
        AppStateProvider {
            div {
                class: "min-h-screen bg-gray-50",
                Outlet::<Route> {}
            }
        }
    }
}

/// Authentication guard component
#[component]
fn AuthGuard(children: Element) -> Element {
    let app_state = use_context::<crate::ui::state::AppStateContext>();

    // In a real app, this would check authentication status
    let is_authenticated = app_state.current_user.is_some();

    if is_authenticated {
        rsx! { {children} }
    } else {
        rsx! {
            div {
                class: "min-h-screen flex items-center justify-center bg-gray-50",
                Login {}
            }
        }
    }
}

/// Protected route wrapper
#[component]
fn ProtectedRoute(children: Element) -> Element {
    rsx! {
        AuthGuard {
            Layout {
                {children}
            }
        }
    }
}

/// Route definitions with proper authentication guards
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum AppRoute {
    #[route("/")]
    #[redirect("/dashboard", || AppRoute::DashboardPage {})]
    HomePage {},

    #[route("/login")]
    LoginPage {},

    #[route("/dashboard")]
    DashboardPage {},

    #[route("/profile")]
    ProfilePage {},

    // Catch-all route for 404s
    #[route("/:..route")]
    NotFoundPage { route: Vec<String> },
}

/// Route components that implement the routes
#[component]
fn HomePage() -> Element {
    rsx! {
        ProtectedRoute {
            Dashboard {}
        }
    }
}

#[component]
fn LoginPage() -> Element {
    rsx! {
        div {
            class: "min-h-screen flex items-center justify-center bg-gray-50",
            Login {}
        }
    }
}

#[component]
fn DashboardPage() -> Element {
    rsx! {
        ProtectedRoute {
            Dashboard {}
        }
    }
}

#[component]
fn ProfilePage() -> Element {
    rsx! {
        ProtectedRoute {
            Profile {}
        }
    }
}

#[component]
fn NotFoundPage(route: Vec<String>) -> Element {
    rsx! {
        div {
            class: "min-h-screen flex items-center justify-center bg-gray-50",
            NotFound {
                path: route.join("/")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    #[test]
    fn app_component_renders() {
        // Basic test to ensure the component structure is valid
        let mut vdom = VirtualDom::new(App);
        let _ = vdom.rebuild_in_place();
    }
}
