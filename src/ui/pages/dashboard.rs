// src/ui/pages/dashboard.rs - Main dashboard page with overview and stats

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::ui::{
    pages::{PageWrapper, StatCard, StatTrend},
    router::Route,
    state::use_app_state,
};

/// Main dashboard component
#[component]
pub fn Dashboard() -> Element {
    let app_state = use_app_state();
    let mut loading = use_signal(|| false);

    // Clone user data to avoid borrowing issues
    let current_user = app_state.current_user.clone();

    // Mock data - in real app this would come from API
    let stats = get_dashboard_stats();
    let recent_activities = get_recent_activities();
    let quick_actions = get_quick_actions();

    let page_actions = rsx! {
        div {
            class: "flex space-x-3",
            RefreshButton { loading: loading }
            Link {
                to: Route::Settings {},
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
                        d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                    }
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                    }
                }
                "Settings"
            }
        }
    };

    rsx! {
        PageWrapper {
            title: "Dashboard".to_string(),
            subtitle: Some("Welcome back! Here's what's happening.".to_string()),
            actions: Some(page_actions),

            WelcomeMessage { user: current_user }
            StatisticsCards { stats: stats }
            MainContentGrid {
                recent_activities: recent_activities,
                quick_actions: quick_actions
            }
            SystemHealthCard {}
        }
    }
}

/// Refresh button component
#[component]
fn RefreshButton(loading: Signal<bool>) -> Element {
    let handle_refresh = move |_| {
        loading.set(true);
        // Simulate refresh
        spawn(async move {
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(1000).await;
            loading.set(false);
        });
    };

    let button_content = if loading() {
        rsx! {
            svg {
                class: "animate-spin -ml-1 mr-2 h-4 w-4",
                xmlns: "http://www.w3.org/2000/svg",
                fill: "none",
                view_box: "0 0 24 24",
                circle {
                    class: "opacity-25",
                    cx: "12",
                    cy: "12",
                    r: "10",
                    stroke: "currentColor",
                    stroke_width: "4",
                }
                path {
                    class: "opacity-75",
                    fill: "currentColor",
                    d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                }
            }
        }
    } else {
        rsx! {
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
                    d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                }
            }
        }
    };

    rsx! {
        button {
            r#type: "button",
            class: "inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
            onclick: handle_refresh,
            {button_content}
            "Refresh"
        }
    }
}

/// Welcome message component
#[component]
fn WelcomeMessage(user: Option<crate::ui::state::User>) -> Element {
    fn fmt_last_login_time(ts: chrono::DateTime<chrono::Utc>) -> String {
        ts.format("%B %d, %Y at %H:%M").to_string()
    }

    if let Some(user) = user {
        let last_login_message = if let Some(last_login) = user.last_login {
            rsx! {
                p { "Last login: {fmt_last_login_time(last_login)}" }
            }
        } else {
            rsx! {
                p { "This is your first login. Welcome to Qorzen!" }
            }
        };

        rsx! {
            div {
                class: "bg-blue-50 border border-blue-200 rounded-md p-4 mb-6",
                div {
                    class: "flex",
                    div {
                        class: "flex-shrink-0",
                        span {
                            class: "text-2xl",
                            "👋"
                        }
                    }
                    div {
                        class: "ml-3",
                        h3 {
                            class: "text-sm font-medium text-blue-800",
                            "Welcome back, {user.profile.display_name}!"
                        }
                        div {
                            class: "mt-2 text-sm text-blue-700",
                            {last_login_message}
                        }
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}

/// Statistics cards component
#[component]
fn StatisticsCards(stats: Vec<DashboardStat>) -> Element {
    rsx! {
        div {
            class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8",
            for stat in stats {
                StatCard {
                    key: "{stat.id}",
                    title: stat.title,
                    value: stat.value,
                    change: stat.change,
                    trend: stat.trend,
                    icon: stat.icon
                }
            }
        }
    }
}

/// Main content grid component
#[component]
fn MainContentGrid(recent_activities: Vec<Activity>, quick_actions: Vec<QuickAction>) -> Element {
    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-6",

            // Recent Activity (2/3 width)
            div {
                class: "lg:col-span-2",
                RecentActivityCard { activities: recent_activities }
            }

            // Quick Actions (1/3 width)
            div {
                class: "lg:col-span-1",
                QuickActionsCard { actions: quick_actions }
            }
        }
    }
}

/// Recent activity card component
#[component]
fn RecentActivityCard(activities: Vec<Activity>) -> Element {
    let activity_content = if activities.is_empty() {
        rsx! {
            div {
                class: "text-center py-6",
                span {
                    class: "text-4xl mb-2 block",
                    "📝"
                }
                p {
                    class: "text-gray-500",
                    "No recent activity"
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "flow-root",
                ul {
                    class: "-mb-8",
                    for (i, activity) in activities.iter().enumerate() {
                        ActivityListItem {
                            key: "{activity.id}",
                            activity: activity.clone(),
                            show_line: i < activities.len() - 1
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div {
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "Recent Activity"
                }
                p {
                    class: "mt-1 max-w-2xl text-sm text-gray-500",
                    "Latest system events and updates"
                }
            }
            div {
                class: "px-4 py-5 sm:p-6",
                {activity_content}
            }
        }
    }
}

/// Individual activity list item
#[component]
fn ActivityListItem(activity: Activity, show_line: bool) -> Element {
    let timeline_line = if show_line {
        rsx! {
            span {
                class: "absolute top-5 left-5 -ml-px h-full w-0.5 bg-gray-200"
            }
        }
    } else {
        rsx! {}
    };

    let activity_description = if let Some(description) = &activity.description {
        rsx! {
            p {
                class: "mt-1 text-sm text-gray-600",
                "{description}"
            }
        }
    } else {
        rsx! {}
    };

    fn fmt_activity_time(ts: chrono::DateTime<chrono::Utc>) -> String {
        ts.format("%H:%M").to_string()
    }

    rsx! {
        li {
            div {
                class: "relative pb-8",
                {timeline_line}
                div {
                    class: "relative flex items-start space-x-3",
                    div {
                        class: "relative",
                        span {
                            class: "h-10 w-10 rounded-full flex items-center justify-center text-white {activity.color}",
                            "{activity.icon}"
                        }
                    }
                    div {
                        class: "min-w-0 flex-1",
                        div {
                            p {
                                class: "text-sm text-gray-900",
                                "{activity.title}"
                            }
                            {activity_description}
                        }
                        div {
                            class: "mt-2 text-xs text-gray-500",
                            "{fmt_activity_time(activity.timestamp)}"
                        }
                    }
                }
            }
        }
    }
}

/// Quick actions card component
#[component]
fn QuickActionsCard(actions: Vec<QuickAction>) -> Element {
    rsx! {
        div {
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "Quick Actions"
                }
            }
            div {
                class: "px-4 py-5 sm:p-6",
                div {
                    class: "space-y-3",
                    for action in actions {
                        QuickActionItem { key: "{action.id}", action: action }
                    }
                }
            }
        }
    }
}

/// Individual quick action item
#[component]
fn QuickActionItem(action: QuickAction) -> Element {
    if let Some(route) = &action.route {
        rsx! {
            Link {
                to: route.clone(),
                class: "group flex items-center p-3 rounded-md hover:bg-gray-50 transition-colors",
                div {
                    class: "flex-shrink-0",
                    span {
                        class: "text-2xl",
                        "{action.icon}"
                    }
                }
                div {
                    class: "ml-3 flex-1",
                    p {
                        class: "text-sm font-medium text-gray-900 group-hover:text-blue-600",
                        "{action.title}"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "{action.description}"
                    }
                }
                div {
                    class: "ml-3 flex-shrink-0",
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
    } else {
        rsx! {
            div {
                class: "flex items-center p-3 rounded-md",
                div {
                    class: "flex-shrink-0",
                    span {
                        class: "text-2xl",
                        "{action.icon}"
                    }
                }
                div {
                    class: "ml-3 flex-1",
                    p {
                        class: "text-sm font-medium text-gray-900",
                        "{action.title}"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "{action.description}"
                    }
                }
            }
        }
    }
}

/// System health monitoring card
#[component]
fn SystemHealthCard() -> Element {
    // Mock system health data
    let health_metrics = vec![
        ("CPU Usage", "23%", "bg-green-500"),
        ("Memory", "67%", "bg-yellow-500"),
        ("Storage", "45%", "bg-green-500"),
        ("Network", "12%", "bg-green-500"),
    ];

    rsx! {
        div {
            class: "mt-6",
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900",
                        "System Health"
                    }
                    p {
                        class: "mt-1 max-w-2xl text-sm text-gray-500",
                        "Current system performance metrics"
                    }
                }
                div {
                    class: "px-4 py-5 sm:p-6",
                    div {
                        class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                        for (name, value, color) in health_metrics {
                            SystemHealthMetric {
                                key: "{name}",
                                name: name.to_string(),
                                value: value.to_string(),
                                color: color.to_string()
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Individual system health metric
#[component]
fn SystemHealthMetric(name: String, value: String, color: String) -> Element {
    rsx! {
        div {
            class: "text-center",
            div {
                class: "mx-auto w-16 h-16 rounded-full {color} flex items-center justify-center text-white font-bold",
                "{value}"
            }
            p {
                class: "mt-2 text-sm font-medium text-gray-900",
                "{name}"
            }
        }
    }
}

// Data structures for dashboard
#[derive(Debug, Clone, PartialEq)]
struct DashboardStat {
    id: String,
    title: String,
    value: String,
    change: Option<String>,
    trend: Option<StatTrend>,
    icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct Activity {
    id: String,
    title: String,
    description: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
    icon: String,
    color: String,
}

#[derive(Debug, Clone, PartialEq)]
struct QuickAction {
    id: String,
    title: String,
    description: String,
    icon: String,
    route: Option<Route>,
}

fn get_dashboard_stats() -> Vec<DashboardStat> {
    vec![
        DashboardStat {
            id: "users".to_string(),
            title: "Total Users".to_string(),
            value: "1,234".to_string(),
            change: Some("+12%".to_string()),
            trend: Some(StatTrend::Up),
            icon: Some("👥".to_string()),
        },
        DashboardStat {
            id: "plugins".to_string(),
            title: "Active Plugins".to_string(),
            value: "8".to_string(),
            change: Some("+2".to_string()),
            trend: Some(StatTrend::Up),
            icon: Some("🧩".to_string()),
        },
        DashboardStat {
            id: "sessions".to_string(),
            title: "Active Sessions".to_string(),
            value: "87".to_string(),
            change: Some("-5%".to_string()),
            trend: Some(StatTrend::Down),
            icon: Some("🔐".to_string()),
        },
        DashboardStat {
            id: "uptime".to_string(),
            title: "System Uptime".to_string(),
            value: "99.9%".to_string(),
            change: None,
            trend: None,
            icon: Some("⚡".to_string()),
        },
    ]
}

fn get_recent_activities() -> Vec<Activity> {
    let now = chrono::Utc::now();
    vec![
        Activity {
            id: "1".to_string(),
            title: "New user registered".to_string(),
            description: Some("john.doe@example.com joined the platform".to_string()),
            timestamp: now - chrono::Duration::minutes(15),
            icon: "👤".to_string(),
            color: "bg-green-500".to_string(),
        },
        Activity {
            id: "2".to_string(),
            title: "Plugin installed".to_string(),
            description: Some("Inventory Management plugin was activated".to_string()),
            timestamp: now - chrono::Duration::hours(2),
            icon: "🧩".to_string(),
            color: "bg-blue-500".to_string(),
        },
        Activity {
            id: "3".to_string(),
            title: "System backup completed".to_string(),
            description: Some("Daily backup finished successfully".to_string()),
            timestamp: now - chrono::Duration::hours(6),
            icon: "💾".to_string(),
            color: "bg-gray-500".to_string(),
        },
        Activity {
            id: "4".to_string(),
            title: "Security scan completed".to_string(),
            description: Some("No vulnerabilities detected".to_string()),
            timestamp: now - chrono::Duration::hours(12),
            icon: "🔒".to_string(),
            color: "bg-green-600".to_string(),
        },
    ]
}

fn get_quick_actions() -> Vec<QuickAction> {
    vec![
        QuickAction {
            id: "profile".to_string(),
            title: "Update Profile".to_string(),
            description: "Manage your account settings".to_string(),
            icon: "👤".to_string(),
            route: Some(Route::Profile {}),
        },
        QuickAction {
            id: "plugins".to_string(),
            title: "Browse Plugins".to_string(),
            description: "Discover and install new plugins".to_string(),
            icon: "🧩".to_string(),
            route: Some(Route::Plugins {}),
        },
        QuickAction {
            id: "settings".to_string(),
            title: "System Settings".to_string(),
            description: "Configure application preferences".to_string(),
            icon: "⚙️".to_string(),
            route: Some(Route::Settings {}),
        },
        QuickAction {
            id: "admin".to_string(),
            title: "Administration".to_string(),
            description: "Manage users and system".to_string(),
            icon: "👑".to_string(),
            route: Some(Route::Admin {}),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_stats() {
        let stats = get_dashboard_stats();
        assert!(!stats.is_empty());
        assert!(stats.iter().any(|s| s.id == "users"));
    }

    #[test]
    fn test_dashboard_activities() {
        let activities = get_recent_activities();
        assert!(!activities.is_empty());
    }

    #[test]
    fn test_quick_actions() {
        let actions = get_quick_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.id == "profile"));
    }
}