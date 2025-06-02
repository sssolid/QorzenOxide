// src/ui/pages/mod.rs - Page components module

use dioxus::prelude::*;

// Module declarations
mod admin;
mod dashboard;
mod login;
mod not_found;
mod plugins;
mod profile;
mod settings;

// Re-exports
pub use admin::Admin;
pub use dashboard::Dashboard;
pub use login::Login;
pub use not_found::NotFound;
pub use plugins::{PluginView, Plugins};
pub use profile::Profile;
pub use settings::Settings;

/// Common page wrapper component
#[component]
pub fn PageWrapper(
    #[props(default = "".to_string())] title: String,
    #[props(default = None)] subtitle: Option<String>,
    #[props(default = None)] actions: Option<Element>,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!("space-y-6 {}", class),

            // Page header
            if !title.is_empty() {
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

            // Page content
            {children}
        }
    }
}

/// Loading skeleton component for pages
#[component]
pub fn PageSkeleton() -> Element {
    rsx! {
        div {
            class: "space-y-6 animate-pulse",

            // Title skeleton
            div {
                class: "h-8 bg-gray-200 rounded w-1/3"
            }

            // Content skeletons
            div {
                class: "space-y-4",
                div {
                    class: "h-4 bg-gray-200 rounded w-3/4"
                }
                div {
                    class: "h-4 bg-gray-200 rounded w-1/2"
                }
                div {
                    class: "h-4 bg-gray-200 rounded w-5/6"
                }
            }

            // Card skeletons
            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                for _ in 0..6 {
                    div {
                        class: "bg-white p-6 rounded-lg shadow",
                        div {
                            class: "space-y-3",
                            div {
                                class: "h-4 bg-gray-200 rounded w-1/2"
                            }
                            div {
                                class: "h-3 bg-gray-200 rounded w-full"
                            }
                            div {
                                class: "h-3 bg-gray-200 rounded w-3/4"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Error state component for pages
#[component]
pub fn PageError(
    #[props(default = "An error occurred".to_string())] message: String,
    #[props(default = None)] retry_action: Option<Callback<()>>,
) -> Element {
    rsx! {
        div {
            class: "text-center py-12",
            div {
                class: "text-6xl text-red-500 mb-4",
                "⚠️"
            }
            h2 {
                class: "text-2xl font-bold text-gray-900 mb-2",
                "Oops! Something went wrong"
            }
            p {
                class: "text-gray-600 mb-6",
                "{message}"
            }
            if let Some(retry) = retry_action {
                button {
                    r#type: "button",
                    class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                    onclick: move |_| retry.call(()),
                    "Try Again"
                }
            }
        }
    }
}

/// Empty state component for pages
#[component]
pub fn EmptyState(
    #[props(default = "📭".to_string())] icon: String,
    #[props(default = "No data available".to_string())] title: String,
    #[props(default = "There's nothing to show here yet.".to_string())] description: String,
    #[props(default = None)] action: Option<Element>,
) -> Element {
    rsx! {
        div {
            class: "text-center py-12",
            div {
                class: "text-6xl mb-4",
                "{icon}"
            }
            h3 {
                class: "text-lg font-medium text-gray-900 mb-2",
                "{title}"
            }
            p {
                class: "text-gray-500 mb-6",
                "{description}"
            }
            if let Some(action) = action {
                {action}
            }
        }
    }
}

/// Stat card component for dashboards
#[component]
pub fn StatCard(
    title: String,
    value: String,
    #[props(default = None)] change: Option<String>,
    #[props(default = None)] trend: Option<StatTrend>,
    #[props(default = None)] icon: Option<String>,
) -> Element {
    let trend_color = match trend {
        Some(StatTrend::Up) => "text-green-600",
        Some(StatTrend::Down) => "text-red-600",
        Some(StatTrend::Neutral) => "text-gray-600",
        None => "text-gray-600",
    };

    rsx! {
        div {
            class: "bg-white overflow-hidden shadow rounded-lg",
            div {
                class: "p-5",
                div {
                    class: "flex items-center",
                    div {
                        class: "flex-shrink-0",
                        if let Some(icon) = icon {
                            span {
                                class: "text-2xl",
                                "{icon}"
                            }
                        }
                    }
                    div {
                        class: "ml-5 w-0 flex-1",
                        dl {
                            dt {
                                class: "text-sm font-medium text-gray-500 truncate",
                                "{title}"
                            }
                            dd {
                                class: "flex items-baseline",
                                div {
                                    class: "text-2xl font-semibold text-gray-900",
                                    "{value}"
                                }
                                if let Some(change_text) = change {
                                    div {
                                        class: format!("ml-2 flex items-baseline text-sm font-semibold {}", trend_color),
                                        match trend {
                                            Some(StatTrend::Up) => rsx! { "↗ {change_text}" },
                                            Some(StatTrend::Down) => rsx! { "↘ {change_text}" },
                                            _ => rsx! { "{change_text}" },
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

/// Trend direction for stat cards
#[derive(Debug, Clone, PartialEq)]
pub enum StatTrend {
    Up,
    Down,
    Neutral,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_wrapper_creation() {
        let _wrapper = rsx! {
            PageWrapper {
                title: "Test Page".to_string(),
                div { "Content" }
            }
        };
    }

    #[test]
    fn test_stat_card_creation() {
        let _card = rsx! {
            StatCard {
                title: "Total Users".to_string(),
                value: "1,234".to_string(),
                change: Some("+12%".to_string()),
                trend: Some(StatTrend::Up),
                icon: Some("👥".to_string())
            }
        };
    }

    #[test]
    fn test_empty_state_creation() {
        let _empty = rsx! {
            EmptyState {
                icon: "📝".to_string(),
                title: "No items".to_string(),
                description: "Create your first item".to_string()
            }
        };
    }
}
