// src/ui/layout/main_layout.rs - Main layout component that orchestrates the overall page structure

use dioxus::prelude::*;

use crate::ui::{
    layout::{Footer, Header, Sidebar},
    state::{ui::use_mobile_menu, ui::use_sidebar, use_app_state},
};

/// Main layout component that provides the overall page structure
#[component]
pub fn Layout(#[props] children: Element) -> Element {
    let app_state = use_app_state();
    let (sidebar_collapsed, toggle_sidebar, _) = use_sidebar();
    let (mobile_menu_open, toggle_mobile_menu, set_mobile_menu_open) = use_mobile_menu();

    // Close mobile menu when clicking outside
    let close_mobile_menu = use_callback(move |_: Event<MouseData>| {
        set_mobile_menu_open(false);
    });

    rsx! {
        div {
            class: "min-h-screen bg-gray-50 flex flex-col",

            // Header
            Header {
                on_menu_toggle: toggle_mobile_menu,
                on_sidebar_toggle: toggle_sidebar
            }

            // Main content area with sidebar
            div {
                class: "flex flex-1 overflow-hidden",

                // Sidebar
                Sidebar {
                    collapsed: sidebar_collapsed,
                    mobile_open: mobile_menu_open,
                    on_close: close_mobile_menu
                }

                // Main content
                main {
                    class: format!(
                        "flex-1 overflow-y-auto transition-all duration-200 ease-in-out {}",
                        if sidebar_collapsed {
                            "lg:ml-16"
                        } else {
                            "lg:ml-64"
                        }
                    ),

                    // Content container
                    div {
                        class: "container mx-auto px-4 sm:px-6 lg:px-8 py-6 max-w-7xl",

                        // Error message display
                        if let Some(error) = &app_state.error_message {
                            div {
                                class: "mb-6 bg-red-50 border border-red-200 rounded-md p-4",
                                div {
                                    class: "flex items-center",
                                    div {
                                        class: "flex-shrink-0",
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
                                    div {
                                        class: "ml-3",
                                        h3 {
                                            class: "text-sm font-medium text-red-800",
                                            "Error"
                                        }
                                        div {
                                            class: "mt-2 text-sm text-red-700",
                                            "{error}"
                                        }
                                    }
                                }
                            }
                        }

                        // Loading indicator
                        if app_state.is_loading {
                            div {
                                class: "mb-6 bg-blue-50 border border-blue-200 rounded-md p-4",
                                div {
                                    class: "flex items-center",
                                    div {
                                        class: "animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600 mr-3"
                                    }
                                    span {
                                        class: "text-blue-800 text-sm font-medium",
                                        "Loading..."
                                    }
                                }
                            }
                        }

                        // Page content
                        {children}
                    }
                }

                // Mobile menu overlay
                if mobile_menu_open {
                    div {
                        class: "fixed inset-0 z-40 lg:hidden",
                        onclick: close_mobile_menu,

                        // Backdrop
                        div {
                            class: "fixed inset-0 bg-gray-600 bg-opacity-75 transition-opacity"
                        }
                    }
                }
            }

            // Footer
            Footer {}
        }
    }
}

/// Responsive layout wrapper for different screen sizes
#[component]
pub fn ResponsiveLayout(
    #[props(default = "".to_string())] mobile_class: String,
    #[props(default = "".to_string())] tablet_class: String,
    #[props(default = "".to_string())] desktop_class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!(
                "{} sm:{} md:{} lg:{}",
                mobile_class,
                tablet_class,
                tablet_class,
                desktop_class
            ),
            {children}
        }
    }
}

/// Content wrapper with consistent padding and max-width
#[component]
pub fn ContentWrapper(
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!(
                "mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 {}",
                class
            ),
            {children}
        }
    }
}

/// Page header component for consistent page titles and actions
#[component]
pub fn PageHeader(
    title: String,
    #[props(default = None)] subtitle: Option<String>,
    #[props(default = None)] actions: Option<Element>,
) -> Element {
    rsx! {
        div {
            class: "mb-8",
            div {
                class: "md:flex md:items-center md:justify-between",
                div {
                    class: "flex-1 min-w-0",
                    h1 {
                        class: "text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate",
                        "{title}"
                    }
                    if let Some(subtitle) = subtitle {
                        p {
                            class: "mt-1 text-sm text-gray-500",
                            "{subtitle}"
                        }
                    }
                }
                if let Some(actions) = actions {
                    div {
                        class: "mt-4 flex md:mt-0 md:ml-4",
                        {actions}
                    }
                }
            }
        }
    }
}

/// Card component for consistent content containers
#[component]
pub fn Card(
    #[props(default = "".to_string())] class: String,
    #[props(default = None)] title: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!(
                "bg-white overflow-hidden shadow rounded-lg {}",
                class
            ),
            if let Some(title) = title {
                div {
                    class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900",
                        "{title}"
                    }
                }
            }
            div {
                class: "px-4 py-5 sm:p-6",
                {children}
            }
        }
    }
}

/// Grid layout component for responsive grids
#[component]
pub fn Grid(
    #[props(default = 1)] cols: u32,
    #[props(default = 2)] sm_cols: u32,
    #[props(default = 3)] md_cols: u32,
    #[props(default = 4)] lg_cols: u32,
    #[props(default = "gap-6".to_string())] gap: String,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!(
                "grid grid-cols-{} sm:grid-cols-{} md:grid-cols-{} lg:grid-cols-{} {} {}",
                cols, sm_cols, md_cols, lg_cols, gap, class
            ),
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    #[test]
    fn test_layout_component_creation() {
        // Test that the component can be created without panicking
        let _layout = rsx! {
            Layout {
                div { "Test content" }
            }
        };
    }

    #[test]
    fn test_card_component_creation() {
        let _card = rsx! {
            Card {
                title: "Test Card".to_string(),
                div { "Card content" }
            }
        };
    }

    #[test]
    fn test_page_header_component_creation() {
        let _header = rsx! {
            PageHeader {
                title: "Test Page".to_string(),
                subtitle: Some("Test subtitle".to_string())
            }
        };
    }
}
