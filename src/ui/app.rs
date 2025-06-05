// src/ui/app.rs - Main application component updated to work with ApplicationCore integration

use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;
use crate::app::core::get_application_core;
use crate::ui::components::plugin_manager_provider::PluginManagerProvider;

use crate::ui::{
    layout::Layout,
    pages::{Dashboard, Login, NotFound, Profile},
    router::Route,
    state::AppStateProvider,
};
use crate::ui::services::plugin_service::PluginServiceProvider;
use crate::VERSION;

/// Initialization state for the application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitializationState {
    Loading,
    Ready,
    Error,
}

/// Context for application initialization state
#[derive(Debug, Clone)]
pub struct AppInitContext {
    pub state: InitializationState,
    pub error_message: Option<String>,
}

/// Main application component that sets up routing and global state
#[component]
pub fn App() -> Element {
    // Check if we're running in a properly initialized environment
    let init_state = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, plugins are compiled in, so we're always ready
            AppInitContext {
                state: InitializationState::Ready,
                error_message: None,
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // For native, check if ApplicationCore is available
            if get_application_core().is_some() {
                AppInitContext {
                    state: InitializationState::Ready,
                    error_message: None,
                }
            } else {
                AppInitContext {
                    state: InitializationState::Loading,
                    error_message: None,
                }
            }
        }
    });

    // Monitor initialization state for native builds
    #[cfg(not(target_arch = "wasm32"))]
    use_effect({
        let mut init_state = init_state.clone();
        move || {
            spawn(async move {
                let mut retry_count = 0;
                let max_retries = 20; // Increased from 10
                let retry_delay = 250; // Reduced from 500ms

                loop {
                    if let Some(_app_core) = get_application_core() {
                        match crate::ui::services::plugin_service::get_plugin_service().try_read() {
                            Ok(service) => {
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(5),
                                    service.get_loaded_plugins()
                                ).await {
                                    Ok(Ok(_plugins)) => {
                                        tracing::info!("ApplicationCore and plugin service are ready");
                                        init_state.set(AppInitContext {
                                            state: InitializationState::Ready,
                                            error_message: None,
                                        });
                                        return;
                                    }
                                    Ok(Err(e)) => {
                                        tracing::debug!("Plugin service not fully ready yet: {}", e);
                                    }
                                    Err(_) => {
                                        tracing::debug!("Plugin service call timed out");
                                    }
                                }
                            }
                            Err(_) => {
                                tracing::debug!("Plugin service not accessible yet");
                            }
                        }
                    } else {
                        tracing::debug!("ApplicationCore not available yet (attempt {}/{})", retry_count + 1, max_retries);
                    }

                    retry_count += 1;
                    if retry_count >= max_retries {
                        tracing::error!("Timeout waiting for ApplicationCore initialization after {} attempts", max_retries);
                        init_state.set(AppInitContext {
                            state: InitializationState::Ready, // Changed from Error to Ready to allow partial functionality
                            error_message: Some("Initialization took longer than expected, continuing anyway".to_string()),
                        });
                        return;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
                }
            });
        }
    });

    // Provide initialization context to child components
    use_context_provider(|| init_state);

    match init_state().state {
        InitializationState::Loading => {
            rsx! {
                LoadingScreen {}
            }
        }
        InitializationState::Error => {
            rsx! {
                ErrorScreen {
                    message: init_state().error_message.unwrap_or_else(|| "Unknown error".to_string())
                }
            }
        }
        InitializationState::Ready => {
            rsx! {
                AppStateProvider {
                    PluginServiceProvider {
                        PluginManagerProvider {
                            Router::<Route> {}
                        }
                    }
                }
            }
        }
    }
}

/// Loading screen shown during initialization
#[component]
fn LoadingScreen() -> Element {
    rsx! {
        div {
            class: "min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center",
            div {
                class: "text-center",
                div {
                    class: "animate-spin rounded-full h-16 w-16 border-b-2 border-blue-600 mx-auto mb-4"
                }
                h1 {
                    class: "text-3xl font-bold text-gray-900 mb-2",
                    "Qorzen Oxide"
                }
                p {
                    class: "text-gray-600 mb-4",
                    "Initializing application..."
                }
                div {
                    class: "flex items-center justify-center space-x-2 text-sm text-gray-500",
                    span { "🔧" }
                    span { "Loading core systems and plugins" }
                }
                div {
                    class: "mt-4 text-xs text-gray-400",
                    span { "Version " }
                    span { "{crate::VERSION}" }
                }
            }
        }
    }
}

/// Error screen shown if initialization fails
#[component]
fn ErrorScreen(message: String) -> Element {
    rsx! {
        div {
            class: "min-h-screen bg-gradient-to-br from-red-50 to-pink-100 flex items-center justify-center",
            div {
                class: "text-center max-w-md mx-auto p-6",
                div {
                    class: "text-6xl text-red-500 mb-4",
                    "⚠️"
                }
                h1 {
                    class: "text-3xl font-bold text-gray-900 mb-2",
                    "Initialization Failed"
                }
                p {
                    class: "text-gray-600 mb-6",
                    "{message}"
                }
                div {
                    class: "text-sm text-gray-500 mb-6",
                    p { "The application failed to initialize properly." }
                    p { "Please check the logs for more details." }
                }
                button {
                    class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors",
                    onclick: move |_| {
                        // Reload the page
                        #[cfg(target_arch = "wasm32")]
                        {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().reload();
                            }
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // For native, we can't easily restart, so exit with error
                            std::process::exit(1);
                        }
                    },
                    "Restart Application"
                }
            }
        }
    }
}

/// Hook to access initialization state
pub fn use_init_state() -> Signal<AppInitContext> {
    use_context::<Signal<AppInitContext>>()
}

/// Hook to access ApplicationCore (native only)
#[cfg(not(target_arch = "wasm32"))]
pub fn use_application_core() -> Option<std::sync::Arc<tokio::sync::RwLock<crate::app::native::ApplicationCore>>> {
    get_application_core()
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

    #[test]
    fn test_initialization_states() {
        assert_eq!(InitializationState::Loading, InitializationState::Loading);
        assert_ne!(InitializationState::Loading, InitializationState::Ready);
    }

    #[test]
    fn test_app_init_context() {
        let context = AppInitContext {
            state: InitializationState::Ready,
            error_message: None,
        };

        assert_eq!(context.state, InitializationState::Ready);
        assert!(context.error_message.is_none());
    }
}