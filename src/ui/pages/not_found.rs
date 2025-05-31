// src/ui/pages/not_found.rs - 404 Not Found page

use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::ui::router::Route;

/// 404 Not Found page component
#[component]
pub fn NotFound(#[props(default = "".to_string())] path: String) -> Element {
    rsx! {
        div {
            class: "min-h-screen bg-white px-4 py-16 sm:px-6 sm:py-24 md:grid md:place-items-center lg:px-8",
            div {
                class: "max-w-max mx-auto",
                main {
                    class: "sm:flex",
                    p {
                        class: "text-4xl font-extrabold text-blue-600 sm:text-5xl",
                        "404"
                    }
                    div {
                        class: "sm:ml-6",
                        div {
                            class: "sm:border-l sm:border-gray-200 sm:pl-6",
                            h1 {
                                class: "text-4xl font-extrabold text-gray-900 tracking-tight sm:text-5xl",
                                "Page not found"
                            }
                            p {
                                class: "mt-1 text-base text-gray-500",
                                "Sorry, we couldn't find the page you're looking for."
                            }
                            if !path.is_empty() {
                                p {
                                    class: "mt-2 text-sm text-gray-400 font-mono bg-gray-100 px-2 py-1 rounded",
                                    "Path: /{path}"
                                }
                            }
                        }
                        div {
                            class: "mt-10 flex space-x-3 sm:border-l sm:border-transparent sm:pl-6",
                            Link {
                                to: Route::Dashboard {},
                                class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                svg {
                                    class: "-ml-1 mr-2 h-4 w-4",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    fill: "none",
                                    view_box: "0 0 24 24",
                                    stroke: "currentColor",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
                                    }
                                }
                                "Go back home"
                            }
                            button {
                                r#type: "button",
                                class: "inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                onclick: move |_| {
                                    // Go back in history
                                    #[cfg(target_arch = "wasm32")]
                                    web_sys::window().unwrap().history().unwrap().back().unwrap();
                                },
                                svg {
                                    class: "-ml-1 mr-2 h-4 w-4",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    fill: "none",
                                    view_box: "0 0 24 24",
                                    stroke: "currentColor",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M10 19l-7-7m0 0l7-7m-7 7h18"
                                    }
                                }
                                "Go back"
                            }
                        }
                    }
                }

                // Helpful suggestions
                div {
                    class: "mt-16",
                    div {
                        class: "bg-gray-50 rounded-lg px-6 py-8",
                        h2 {
                            class: "text-lg font-medium text-gray-900 mb-4",
                            "What you can do:"
                        }
                        ul {
                            class: "space-y-3 text-sm text-gray-600",
                            li {
                                class: "flex items-start",
                                svg {
                                    class: "flex-shrink-0 h-5 w-5 text-green-500 mt-0.5 mr-3",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    view_box: "0 0 20 20",
                                    fill: "currentColor",
                                    path {
                                        fill_rule: "evenodd",
                                        d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z",
                                        clip_rule: "evenodd"
                                    }
                                }
                                "Check the URL for any typos"
                            }
                            li {
                                class: "flex items-start",
                                svg {
                                    class: "flex-shrink-0 h-5 w-5 text-green-500 mt-0.5 mr-3",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    view_box: "0 0 20 20",
                                    fill: "currentColor",
                                    path {
                                        fill_rule: "evenodd",
                                        d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z",
                                        clip_rule: "evenodd"
                                    }
                                }
                                "Use the navigation menu to find what you're looking for"
                            }
                            li {
                                class: "flex items-start",
                                svg {
                                    class: "flex-shrink-0 h-5 w-5 text-green-500 mt-0.5 mr-3",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    view_box: "0 0 20 20",
                                    fill: "currentColor",
                                    path {
                                        fill_rule: "evenodd",
                                        d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z",
                                        clip_rule: "evenodd"
                                    }
                                }
                                "Contact support if you believe this is an error"
                            }
                        }
                    }
                }

                // Popular pages
                div {
                    class: "mt-8",
                    h3 {
                        class: "text-sm font-medium text-gray-900 mb-4",
                        "Popular pages:"
                    }
                    div {
                        class: "grid grid-cols-1 sm:grid-cols-2 gap-4",

                        // Dashboard link
                        Link {
                            to: Route::Dashboard {},
                            class: "group relative rounded-lg p-6 bg-white border border-gray-300 shadow-sm hover:shadow-md transition-shadow",
                            div {
                                class: "flex items-center",
                                div {
                                    class: "flex-shrink-0",
                                    span {
                                        class: "text-2xl",
                                        "📊"
                                    }
                                }
                                div {
                                    class: "ml-4 min-w-0 flex-1",
                                    p {
                                        class: "text-base font-medium text-gray-900 group-hover:text-blue-600",
                                        "Dashboard"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "View your overview and stats"
                                    }
                                }
                                div {
                                    class: "flex-shrink-0 ml-4",
                                    svg {
                                        class: "h-5 w-5 text-gray-400 group-hover:text-blue-500",
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
                        }

                        // Profile link
                        Link {
                            to: Route::Profile {},
                            class: "group relative rounded-lg p-6 bg-white border border-gray-300 shadow-sm hover:shadow-md transition-shadow",
                            div {
                                class: "flex items-center",
                                div {
                                    class: "flex-shrink-0",
                                    span {
                                        class: "text-2xl",
                                        "👤"
                                    }
                                }
                                div {
                                    class: "ml-4 min-w-0 flex-1",
                                    p {
                                        class: "text-base font-medium text-gray-900 group-hover:text-blue-600",
                                        "Profile"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Manage your account settings"
                                    }
                                }
                                div {
                                    class: "flex-shrink-0 ml-4",
                                    svg {
                                        class: "h-5 w-5 text-gray-400 group-hover:text-blue-500",
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
                        }

                        // Plugins link
                        Link {
                            to: Route::Plugins {},
                            class: "group relative rounded-lg p-6 bg-white border border-gray-300 shadow-sm hover:shadow-md transition-shadow",
                            div {
                                class: "flex items-center",
                                div {
                                    class: "flex-shrink-0",
                                    span {
                                        class: "text-2xl",
                                        "🧩"
                                    }
                                }
                                div {
                                    class: "ml-4 min-w-0 flex-1",
                                    p {
                                        class: "text-base font-medium text-gray-900 group-hover:text-blue-600",
                                        "Plugins"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Browse available plugins"
                                    }
                                }
                                div {
                                    class: "flex-shrink-0 ml-4",
                                    svg {
                                        class: "h-5 w-5 text-gray-400 group-hover:text-blue-500",
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
                        }

                        // Settings link
                        Link {
                            to: Route::Settings {},
                            class: "group relative rounded-lg p-6 bg-white border border-gray-300 shadow-sm hover:shadow-md transition-shadow",
                            div {
                                class: "flex items-center",
                                div {
                                    class: "flex-shrink-0",
                                    span {
                                        class: "text-2xl",
                                        "⚙️"
                                    }
                                }
                                div {
                                    class: "ml-4 min-w-0 flex-1",
                                    p {
                                        class: "text-base font-medium text-gray-900 group-hover:text-blue-600",
                                        "Settings"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Configure application preferences"
                                    }
                                }
                                div {
                                    class: "flex-shrink-0 ml-4",
                                    svg {
                                        class: "h-5 w-5 text-gray-400 group-hover:text-blue-500",
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
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    #[test]
    fn test_not_found_component_creation() {
        let _not_found = rsx! {
            NotFound {
                path: "test/path".to_string()
            }
        };
    }

    #[test]
    fn test_not_found_without_path() {
        let _not_found = rsx! { NotFound {} };
    }
}
