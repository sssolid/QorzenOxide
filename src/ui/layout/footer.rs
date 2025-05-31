// src/ui/layout/footer.rs - Application footer with links and information

use crate::utils::Time;
use chrono::Datelike;
use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::ui::router::Route;

/// Footer component
#[component]
pub fn Footer() -> Element {
    let current_year = Time::now().year();
    let build = option_env!("BUILD_HASH").unwrap_or("dev");

    rsx! {
        footer {
            class: "bg-white border-t border-gray-200 mt-auto",
            div {
                class: "mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-6",

                // Main footer content
                div {
                    class: "md:flex md:items-center md:justify-between",

                    // Left side - Links
                    div {
                        class: "flex flex-wrap justify-center md:justify-start space-x-6 md:order-2",

                        // Internal links
                        Link {
                            to: Route::Dashboard {},
                            class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                            "Dashboard"
                        }

                        // External links
                        a {
                            href: "https://docs.qorzen.com",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                            "Documentation"
                        }

                        a {
                            href: "https://github.com/qorzen/qorzen-oxide",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                            "GitHub"
                        }

                        a {
                            href: "mailto:support@qorzen.com",
                            class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                            "Support"
                        }

                        a {
                            href: "/privacy",
                            class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                            "Privacy"
                        }

                        a {
                            href: "/terms",
                            class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                            "Terms"
                        }
                    }

                    // Right side - Copyright and version
                    div {
                        class: "mt-4 md:mt-0 md:order-1",
                        div {
                            class: "flex flex-col items-center md:items-start space-y-1",
                            p {
                                class: "text-sm text-gray-500",
                                "© {current_year} Qorzen. All rights reserved."
                            }
                            p {
                                class: "text-xs text-gray-400",
                                "Version {crate::VERSION}"
                            }
                        }
                    }
                }

                // Additional footer information
                div {
                    class: "mt-6 pt-6 border-t border-gray-200",
                    div {
                        class: "flex flex-col sm:flex-row sm:items-center sm:justify-between text-xs text-gray-400 space-y-2 sm:space-y-0",

                        // System status
                        div {
                            class: "flex items-center space-x-4",
                            SystemStatus {}
                            div {
                                class: "flex items-center",
                                span {
                                    class: "inline-block w-2 h-2 bg-green-400 rounded-full mr-1"
                                }
                                "All systems operational"
                            }
                        }

                        // Build info
                        div {
                            class: "flex items-center space-x-4",
                            span { "Built with ❤️ and 🦀" }
                            span {
                                "Build: {build}",
                            }
                        }
                    }
                }
            }
        }
    }
}

/// System status indicator component
#[component]
fn SystemStatus() -> Element {
    // In a real app, this would check actual system health
    let status = use_signal(|| "healthy");
    let last_check = use_signal(Time::now);

    // Simulate periodic health checks
    use_effect({
        let mut status = status;
        let mut last_check = last_check;

        move || {
            spawn(async move {
                loop {
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(30000).await;

                    // Mock health check
                    let health_good = rand::random::<f32>() > 0.1; // 90% chance of good health
                    status.set(if health_good { "healthy" } else { "degraded" });
                    last_check.set(Time::now());
                }
            });
        }
    });

    let (status_color, status_text) = match status() {
        "healthy" => ("bg-green-400", "Healthy"),
        "degraded" => ("bg-yellow-400", "Degraded"),
        "down" => ("bg-red-400", "Down"),
        _ => ("bg-gray-400", "Unknown"),
    };

    fn fmt_time(ts: chrono::DateTime<chrono::Utc>) -> String {
        ts.format("%H:%M:%S UTC").to_string()
    }

    rsx! {
        div {
            class: "flex items-center space-x-1",
            title: "{fmt_time(last_check())}",
            span {
                class: format!("inline-block w-2 h-2 rounded-full {}", status_color)
            }
            span { "System: {status_text}" }
        }
    }
}

/// Expandable footer variant with more information
#[component]
pub fn ExpandedFooter() -> Element {
    let current_year = Time::now().year();

    rsx! {
        footer {
            class: "bg-gray-50 border-t border-gray-200 mt-auto",
            div {
                class: "mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-12",

                // Main footer grid
                div {
                    class: "grid grid-cols-1 md:grid-cols-4 gap-8",

                    // Company info
                    div {
                        class: "col-span-1 md:col-span-2",
                        div {
                            class: "flex items-center mb-4",
                            div {
                                class: "h-8 w-8 bg-blue-600 rounded-lg flex items-center justify-center mr-3",
                                span {
                                    class: "text-white font-bold text-sm",
                                    "Q"
                                }
                            }
                            span {
                                class: "text-xl font-bold text-gray-900",
                                "Qorzen"
                            }
                        }
                        p {
                            class: "text-sm text-gray-600 mb-4 max-w-md",
                            "A modular, cross-platform application framework built with Rust and Dioxus.
                             Extensible through plugins and designed for modern development workflows."
                        }
                        div {
                            class: "flex space-x-4",
                            // Social links would go here
                            a {
                                href: "https://github.com/qorzen",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                class: "text-gray-400 hover:text-gray-600 transition-colors",
                                "GitHub"
                            }
                            a {
                                href: "https://twitter.com/qorzen",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                class: "text-gray-400 hover:text-gray-600 transition-colors",
                                "Twitter"
                            }
                        }
                    }

                    // Product links
                    div {
                        h3 {
                            class: "text-sm font-semibold text-gray-900 tracking-wider uppercase mb-4",
                            "Product"
                        }
                        ul {
                            class: "space-y-2",
                            li {
                                Link {
                                    to: Route::Dashboard {},
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "Dashboard"
                                }
                            }
                            li {
                                Link {
                                    to: Route::Plugins {},
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "Plugins"
                                }
                            }
                            li {
                                Link {
                                    to: Route::Settings {},
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "Settings"
                                }
                            }
                            li {
                                a {
                                    href: "/api/docs",
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "API Documentation"
                                }
                            }
                        }
                    }

                    // Support links
                    div {
                        h3 {
                            class: "text-sm font-semibold text-gray-900 tracking-wider uppercase mb-4",
                            "Support"
                        }
                        ul {
                            class: "space-y-2",
                            li {
                                a {
                                    href: "https://docs.qorzen.com",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "Documentation"
                                }
                            }
                            li {
                                a {
                                    href: "https://docs.qorzen.com/guides",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "Guides"
                                }
                            }
                            li {
                                a {
                                    href: "mailto:support@qorzen.com",
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "Contact Support"
                                }
                            }
                            li {
                                a {
                                    href: "https://status.qorzen.com",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    class: "text-sm text-gray-600 hover:text-gray-900 transition-colors",
                                    "System Status"
                                }
                            }
                        }
                    }
                }

                // Bottom section
                div {
                    class: "mt-12 pt-8 border-t border-gray-200",
                    div {
                        class: "md:flex md:items-center md:justify-between",
                        div {
                            class: "flex space-x-6 md:order-2",
                            a {
                                href: "/privacy",
                                class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                                "Privacy Policy"
                            }
                            a {
                                href: "/terms",
                                class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                                "Terms of Service"
                            }
                            a {
                                href: "/cookies",
                                class: "text-sm text-gray-500 hover:text-gray-600 transition-colors",
                                "Cookie Policy"
                            }
                        }
                        p {
                            class: "mt-4 text-sm text-gray-500 md:mt-0 md:order-1",
                            "© {current_year} Qorzen. All rights reserved. Version {crate::VERSION}"
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
    fn test_footer_component_creation() {
        let _footer = rsx! { Footer {} };
    }

    #[test]
    fn test_expanded_footer_component_creation() {
        let _footer = rsx! { ExpandedFooter {} };
    }

    #[test]
    fn test_system_status_component_creation() {
        let _status = rsx! { SystemStatus {} };
    }
}
