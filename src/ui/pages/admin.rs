// src/ui/pages/admin.rs - Administrative dashboard and user management

use dioxus::prelude::*;

use crate::ui::pages::{EmptyState, PageWrapper, StatCard, StatTrend};

/// Main admin page component
#[component]
pub fn Admin() -> Element {
    let mut active_tab = use_signal(|| "overview".to_string());

    let page_actions = rsx! {
        div {
            class: "flex space-x-3",
            button {
                r#type: "button",
                class: "inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
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
                        d: "M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                    }
                }
                "Export Report"
            }
            button {
                r#type: "button",
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
                        d: "M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z"
                    }
                }
                "Add User"
            }
        }
    };

    let admin_warning_banner = rsx! {
        div {
            class: "mb-6 bg-red-50 border-l-4 border-red-400 p-4",
            div {
                class: "flex",
                div {
                    class: "flex-shrink-0",
                    svg {
                        class: "h-5 w-5 text-red-400",
                        xmlns: "http://www.w3.org/2000/svg",
                        view_box: "0 0 20 20",
                        fill: "currentColor",
                        path {
                            fill_rule: "evenodd",
                            d: "M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z",
                            clip_rule: "evenodd"
                        }
                    }
                }
                div {
                    class: "ml-3",
                    h3 {
                        class: "text-sm font-medium text-red-800",
                        "Administrative Access"
                    }
                    div {
                        class: "mt-2 text-sm text-red-700",
                        p {
                            "You have administrative privileges. Please use these tools responsibly. Changes made here can affect all users and the entire system."
                        }
                    }
                }
            }
        }
    };

    let navigation_tabs = rsx! {
        div {
            class: "border-b border-gray-200 mb-6",
            nav {
                class: "-mb-px flex space-x-8",
                button {
                    r#type: "button",
                    class: if active_tab() == "overview" {
                        "py-2 px-1 border-b-2 font-medium text-sm border-blue-500 text-blue-600"
                    } else {
                        "py-2 px-1 border-b-2 font-medium text-sm border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                    },
                    onclick: move |_| active_tab.set("overview".to_string()),
                    "Overview"
                }
                button {
                    r#type: "button",
                    class: if active_tab() == "users" {
                        "py-2 px-1 border-b-2 font-medium text-sm border-blue-500 text-blue-600"
                    } else {
                        "py-2 px-1 border-b-2 font-medium text-sm border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                    },
                    onclick: move |_| active_tab.set("users".to_string()),
                    "Users"
                }
                button {
                    r#type: "button",
                    class: if active_tab() == "system" {
                        "py-2 px-1 border-b-2 font-medium text-sm border-blue-500 text-blue-600"
                    } else {
                        "py-2 px-1 border-b-2 font-medium text-sm border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                    },
                    onclick: move |_| active_tab.set("system".to_string()),
                    "System"
                }
                button {
                    r#type: "button",
                    class: if active_tab() == "plugins" {
                        "py-2 px-1 border-b-2 font-medium text-sm border-blue-500 text-blue-600"
                    } else {
                        "py-2 px-1 border-b-2 font-medium text-sm border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                    },
                    onclick: move |_| active_tab.set("plugins".to_string()),
                    "Plugins"
                }
                button {
                    r#type: "button",
                    class: if active_tab() == "logs" {
                        "py-2 px-1 border-b-2 font-medium text-sm border-blue-500 text-blue-600"
                    } else {
                        "py-2 px-1 border-b-2 font-medium text-sm border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                    },
                    onclick: move |_| active_tab.set("logs".to_string()),
                    "Logs"
                }
            }
        }
    };

    let tab_content = match active_tab().as_str() {
        "overview" => rsx! { OverviewTab {} },
        "users" => rsx! { UsersTab {} },
        "system" => rsx! { SystemTab {} },
        "plugins" => rsx! { PluginsTab {} },
        "logs" => rsx! { LogsTab {} },
        _ => rsx! { div { "Unknown tab" } },
    };

    rsx! {
        PageWrapper {
            title: "Administration".to_string(),
            subtitle: Some("Manage users, system settings, and monitor application health".to_string()),
            actions: Some(page_actions),

            {admin_warning_banner}
            {navigation_tabs}
            {tab_content}
        }
    }
}

/// Overview tab with system statistics
#[component]
fn OverviewTab() -> Element {
    let stats = get_system_stats();

    let statistics_grid = rsx! {
        div {
            class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6",
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
    };

    let system_health_grid = rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-2 gap-6",

            // Recent activity
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900",
                        "Recent Admin Activity"
                    }
                }
                div {
                    class: "px-4 py-5 sm:p-6",
                    RecentActivityList {}
                }
            }

            // System alerts
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900",
                        "System Alerts"
                    }
                }
                div {
                    class: "px-4 py-5 sm:p-6",
                    SystemAlerts {}
                }
            }
        }
    };

    rsx! {
        div {
            class: "space-y-6",
            {statistics_grid}
            {system_health_grid}
        }
    }
}

/// Users management tab
#[component]
fn UsersTab() -> Element {
    let mut search_query = use_signal(String::new);
    let users = get_mock_users();

    let filtered_users: Vec<_> = users
        .into_iter()
        .filter(|user| {
            if search_query().is_empty() {
                true
            } else {
                let query = search_query().to_lowercase();
                user.name.to_lowercase().contains(&query)
                    || user.email.to_lowercase().contains(&query)
                    || user.role.to_lowercase().contains(&query)
            }
        })
        .collect();

    let active_users = filtered_users
        .iter()
        .filter(|u| u.status == "Active")
        .count();
    let inactive_users = filtered_users
        .iter()
        .filter(|u| u.status == "Inactive")
        .count();

    let search_bar = rsx! {
        div {
            class: "flex justify-between items-center",
            div {
                class: "flex-1 max-w-md",
                div {
                    class: "relative",
                    input {
                        r#type: "text",
                        placeholder: "Search users...",
                        class: "block w-full pr-12 border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 sm:text-sm",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value())
                    }
                    div {
                        class: "absolute inset-y-0 right-0 pr-3 flex items-center pointer-events-none",
                        svg {
                            class: "h-5 w-5 text-gray-400",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                }
            }

            // User statistics
            div {
                class: "flex space-x-4 text-sm text-gray-500",
                span { "Total: {filtered_users.len()}" }
                span {
                    "Active: {active_users}",
                }
                span {
                    "Inactive: {inactive_users}"
                }
            }
        }
    };

    let users_table = if filtered_users.is_empty() {
        rsx! {
            div {
                class: "bg-white shadow overflow-hidden sm:rounded-md",
                div {
                    class: "p-6",
                    EmptyState {
                        icon: "👥".to_string(),
                        title: "No users found".to_string(),
                        description: "Try adjusting your search criteria".to_string(),
                    }
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "bg-white shadow overflow-hidden sm:rounded-md",
                ul {
                    class: "divide-y divide-gray-200",
                    for user in filtered_users {
                        UserListItem { key: "{user.id}", user: user.clone() }
                    }
                }
            }
        }
    };

    rsx! {
        div {
            class: "space-y-6",
            {search_bar}
            {users_table}
        }
    }
}

/// Individual user list item component
#[component]
fn UserListItem(user: MockUser) -> Element {
    let user_initial = user.name.chars().next().unwrap_or('U');
    let status_class = match user.status.as_str() {
        "Active" => "bg-green-100 text-green-800",
        "Inactive" => "bg-gray-100 text-gray-800",
        _ => "bg-red-100 text-red-800",
    };

    rsx! {
        li {
            class: "px-6 py-4 hover:bg-gray-50",
            div {
                class: "flex items-center justify-between",
                div {
                    class: "flex items-center",
                    div {
                        class: "flex-shrink-0 h-10 w-10",
                        div {
                            class: "h-10 w-10 rounded-full bg-blue-500 flex items-center justify-center",
                            span {
                                class: "text-sm font-medium text-white",
                                "{user_initial}"
                            }
                        }
                    }
                    div {
                        class: "ml-4",
                        div {
                            class: "flex items-center",
                            p {
                                class: "text-sm font-medium text-gray-900",
                                "{user.name}"
                            }
                            span {
                                class: "ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {status_class}",
                                "{user.status}"
                            }
                        }
                        p {
                            class: "text-sm text-gray-500",
                            "{user.email}"
                        }
                        p {
                            class: "text-sm text-gray-500",
                            "Role: {user.role} • Last login: {user.last_login}"
                        }
                    }
                }
                div {
                    class: "flex items-center space-x-2",
                    button {
                        r#type: "button",
                        class: "text-blue-600 hover:text-blue-900 text-sm font-medium",
                        "Edit"
                    }
                    button {
                        r#type: "button",
                        class: "text-red-600 hover:text-red-900 text-sm font-medium",
                        "Disable"
                    }
                }
            }
        }
    }
}

/// System monitoring tab
#[component]
fn SystemTab() -> Element {
    let system_metrics = rsx! {
        div {
            class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6",

            MetricCard {
                title: "CPU Usage".to_string(),
                value: "23%".to_string(),
                status: "healthy".to_string(),
                icon: "🖥️".to_string()
            }

            MetricCard {
                title: "Memory Usage".to_string(),
                value: "67%".to_string(),
                status: "warning".to_string(),
                icon: "💾".to_string()
            }

            MetricCard {
                title: "Storage".to_string(),
                value: "45%".to_string(),
                status: "healthy".to_string(),
                icon: "💿".to_string()
            }

            MetricCard {
                title: "Network".to_string(),
                value: "12 MB/s".to_string(),
                status: "healthy".to_string(),
                icon: "🌐".to_string()
            }
        }
    };

    let system_services = rsx! {
        div {
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "System Services"
                }
            }
            div {
                class: "px-4 py-5 sm:p-6",
                SystemServicesList {}
            }
        }
    };

    rsx! {
        div {
            class: "space-y-6",
            {system_metrics}
            {system_services}
        }
    }
}

/// Admin plugins management tab
#[component]
fn PluginsTab() -> Element {
    let plugins = get_admin_plugins();

    let plugin_header = rsx! {
        div {
            class: "px-4 py-5 sm:px-6 border-b border-gray-200",
            div {
                class: "flex justify-between items-center",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "Plugin Management"
                }
                button {
                    r#type: "button",
                    class: "inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                    "Install Plugin"
                }
            }
        }
    };

    let plugin_list = rsx! {
        div {
            class: "px-4 py-5 sm:p-6",
            div {
                class: "space-y-4",
                for plugin in plugins {
                    PluginListItem { key: "{plugin.id}", plugin: plugin.clone() }
                }
            }
        }
    };

    rsx! {
        div {
            class: "space-y-6",
            div {
                class: "bg-white shadow rounded-lg",
                {plugin_header}
                {plugin_list}
            }
        }
    }
}

/// Individual plugin list item component
#[component]
fn PluginListItem(plugin: AdminPlugin) -> Element {
    let action_buttons = if plugin.status == "Active" {
        rsx! {
            button {
                r#type: "button",
                class: "text-yellow-600 hover:text-yellow-900 text-sm font-medium",
                "Disable"
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                class: "text-green-600 hover:text-green-900 text-sm font-medium",
                "Enable"
            }
        }
    };

    rsx! {
        div {
            class: "flex items-center justify-between p-4 border border-gray-200 rounded-lg",
            div {
                class: "flex items-center",
                span {
                    class: "text-2xl mr-4",
                    "{plugin.icon}"
                }
                div {
                    h4 {
                        class: "text-sm font-medium text-gray-900",
                        "{plugin.name}"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "v{plugin.version} • {plugin.status}"
                    }
                }
            }
            div {
                class: "flex items-center space-x-2",
                {action_buttons}
                button {
                    r#type: "button",
                    class: "text-blue-600 hover:text-blue-900 text-sm font-medium",
                    "Configure"
                }
                button {
                    r#type: "button",
                    class: "text-red-600 hover:text-red-900 text-sm font-medium",
                    "Uninstall"
                }
            }
        }
    }
}

/// System logs tab
#[component]
fn LogsTab() -> Element {
    let logs = get_system_logs();

    let logs_header = rsx! {
        div {
            class: "px-4 py-5 sm:px-6 border-b border-gray-200",
            div {
                class: "flex justify-between items-center",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "System Logs"
                }
                div {
                    class: "flex space-x-2",
                    button {
                        r#type: "button",
                        class: "inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                        "Clear Logs"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex items-center px-3 py-2 border border-gray-300 shadow-sm text-sm leading-4 font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                        "Export"
                    }
                }
            }
        }
    };

    let logs_list = rsx! {
        div {
            class: "px-4 py-5 sm:p-6",
            div {
                class: "max-h-96 overflow-y-auto",
                div {
                    class: "space-y-2",
                    for log in logs {
                        LogEntry { key: "{log.id}", log: log.clone() }
                    }
                }
            }
        }
    };

    rsx! {
        div {
            class: "space-y-6",
            div {
                class: "bg-white shadow rounded-lg",
                {logs_header}
                {logs_list}
            }
        }
    }
}

/// Individual log entry component
#[component]
fn LogEntry(log: SystemLog) -> Element {
    let log_class = match log.level.as_str() {
        "ERROR" => "bg-red-50 text-red-800 border-l-4 border-red-400",
        "WARN" => "bg-yellow-50 text-yellow-800 border-l-4 border-yellow-400",
        "INFO" => "bg-blue-50 text-blue-800 border-l-4 border-blue-400",
        _ => "bg-gray-50 text-gray-800 border-l-4 border-gray-400",
    };

    let log_details = if !log.details.is_empty() {
        rsx! {
            div {
                class: "mt-1 text-xs opacity-75",
                "{log.details}"
            }
        }
    } else {
        rsx! {}
    };

    rsx! {
        div {
            class: "p-3 rounded-md text-sm font-mono {log_class}",
            div {
                class: "flex justify-between items-start",
                div {
                    class: "flex-1",
                    span {
                        class: "font-semibold mr-2",
                        "[{log.level}]"
                    }
                    span { "{log.message}" }
                }
                span {
                    class: "text-xs opacity-75 ml-4",
                    "{log.timestamp}"
                }
            }
            {log_details}
        }
    }
}

// Helper components
#[component]
fn RecentActivityList() -> Element {
    let activities = get_recent_admin_activities();

    rsx! {
        div {
            class: "space-y-3",
            for activity in activities {
                div {
                    key: "{activity.id}",
                    class: "flex items-start space-x-3",
                    div {
                        class: "flex-shrink-0",
                        span {
                            class: "inline-flex items-center justify-center h-8 w-8 rounded-full text-white text-sm {activity.color}",
                            "{activity.icon}"
                        }
                    }
                    div {
                        class: "min-w-0 flex-1",
                        p {
                            class: "text-sm text-gray-900",
                            "{activity.action}"
                        }
                        p {
                            class: "text-sm text-gray-500",
                            "by {activity.user} • {activity.timestamp}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SystemAlerts() -> Element {
    let alerts = get_system_alerts();

    if alerts.is_empty() {
        rsx! {
            div {
                class: "text-center py-4",
                span {
                    class: "text-2xl mb-2 block",
                    "✅"
                }
                p {
                    class: "text-sm text-gray-500",
                    "No active alerts"
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "space-y-3",
                for alert in alerts {
                    SystemAlertItem { key: "{alert.id}", alert: alert.clone() }
                }
            }
        }
    }
}

#[component]
fn SystemAlertItem(alert: SystemAlert) -> Element {
    let alert_class = match alert.severity.as_str() {
        "critical" => "bg-red-50 border border-red-200",
        "warning" => "bg-yellow-50 border border-yellow-200",
        _ => "bg-blue-50 border border-blue-200",
    };

    let text_color = match alert.severity.as_str() {
        "critical" => ("text-red-800", "text-red-700"),
        "warning" => ("text-yellow-800", "text-yellow-700"),
        _ => ("text-blue-800", "text-blue-700"),
    };

    rsx! {
        div {
            class: "p-3 rounded-md {alert_class}",
            div {
                class: "flex items-start",
                div {
                    class: "flex-shrink-0",
                    span {
                        class: "text-lg",
                        "{alert.icon}"
                    }
                }
                div {
                    class: "ml-3 flex-1",
                    p {
                        class: "text-sm font-medium {text_color.0}",
                        "{alert.title}"
                    }
                    p {
                        class: "text-sm {text_color.1}",
                        "{alert.message}"
                    }
                }
            }
        }
    }
}

#[component]
fn MetricCard(title: String, value: String, status: String, icon: String) -> Element {
    let status_color = match status.as_str() {
        "healthy" => "text-green-600",
        "warning" => "text-yellow-600",
        "critical" => "text-red-600",
        _ => "text-gray-600",
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
                        span {
                            class: "text-2xl",
                            "{icon}"
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
                                    class: "text-2xl font-semibold {status_color}",
                                    "{value}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SystemServicesList() -> Element {
    let services = get_system_services();

    rsx! {
        div {
            class: "space-y-3",
            for service in services {
                SystemServiceItem { key: "{service.name}", service: service.clone() }
            }
        }
    }
}

#[component]
fn SystemServiceItem(service: SystemService) -> Element {
    let status_dot_class = if service.running {
        "bg-green-400"
    } else {
        "bg-red-400"
    };
    let status_badge_class = if service.running {
        "bg-green-100 text-green-800"
    } else {
        "bg-red-100 text-red-800"
    };
    let status_text = if service.running {
        "Running"
    } else {
        "Stopped"
    };

    let action_button = if service.running {
        rsx! {
            button {
                r#type: "button",
                class: "text-red-600 hover:text-red-900 text-sm font-medium",
                "Stop"
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                class: "text-green-600 hover:text-green-900 text-sm font-medium",
                "Start"
            }
        }
    };

    rsx! {
        div {
            class: "flex items-center justify-between py-2",
            div {
                class: "flex items-center",
                div {
                    class: "h-3 w-3 rounded-full mr-3 {status_dot_class}"
                }
                div {
                    p {
                        class: "text-sm font-medium text-gray-900",
                        "{service.name}"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "{service.description}"
                    }
                }
            }
            div {
                class: "flex items-center space-x-2",
                span {
                    class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {status_badge_class}",
                    "{status_text}"
                }
                {action_button}
            }
        }
    }
}

// Mock data structures and functions
#[derive(Debug, Clone)]
struct AdminStat {
    id: String,
    title: String,
    value: String,
    change: Option<String>,
    trend: Option<StatTrend>,
    icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct MockUser {
    id: String,
    name: String,
    email: String,
    role: String,
    status: String,
    last_login: String,
}

#[derive(Debug, Clone, PartialEq)]
struct AdminActivity {
    id: String,
    action: String,
    user: String,
    timestamp: String,
    icon: String,
    color: String,
}

#[derive(Debug, Clone, PartialEq)]
struct SystemAlert {
    id: String,
    title: String,
    message: String,
    severity: String,
    icon: String,
}

#[derive(Debug, Clone, PartialEq)]
struct AdminPlugin {
    id: String,
    name: String,
    version: String,
    status: String,
    icon: String,
}

#[derive(Debug, Clone, PartialEq)]
struct SystemService {
    name: String,
    description: String,
    running: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct SystemLog {
    id: String,
    level: String,
    message: String,
    timestamp: String,
    details: String,
}

fn get_system_stats() -> Vec<AdminStat> {
    vec![
        AdminStat {
            id: "total_users".to_string(),
            title: "Total Users".to_string(),
            value: "1,234".to_string(),
            change: Some("+12%".to_string()),
            trend: Some(StatTrend::Up),
            icon: Some("👥".to_string()),
        },
        AdminStat {
            id: "active_sessions".to_string(),
            title: "Active Sessions".to_string(),
            value: "87".to_string(),
            change: Some("-5%".to_string()),
            trend: Some(StatTrend::Down),
            icon: Some("🔐".to_string()),
        },
        AdminStat {
            id: "system_uptime".to_string(),
            title: "System Uptime".to_string(),
            value: "99.9%".to_string(),
            change: None,
            trend: None,
            icon: Some("⚡".to_string()),
        },
        AdminStat {
            id: "storage_used".to_string(),
            title: "Storage Used".to_string(),
            value: "45%".to_string(),
            change: Some("+3%".to_string()),
            trend: Some(StatTrend::Up),
            icon: Some("💾".to_string()),
        },
    ]
}

fn get_mock_users() -> Vec<MockUser> {
    vec![
        MockUser {
            id: "1".to_string(),
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            role: "Administrator".to_string(),
            status: "Active".to_string(),
            last_login: "2 hours ago".to_string(),
        },
        MockUser {
            id: "2".to_string(),
            name: "Jane Smith".to_string(),
            email: "jane.smith@example.com".to_string(),
            role: "User".to_string(),
            status: "Active".to_string(),
            last_login: "1 day ago".to_string(),
        },
        MockUser {
            id: "3".to_string(),
            name: "Bob Johnson".to_string(),
            email: "bob.johnson@example.com".to_string(),
            role: "Moderator".to_string(),
            status: "Inactive".to_string(),
            last_login: "1 week ago".to_string(),
        },
    ]
}

fn get_recent_admin_activities() -> Vec<AdminActivity> {
    vec![
        AdminActivity {
            id: "1".to_string(),
            action: "User created".to_string(),
            user: "admin".to_string(),
            timestamp: "5 minutes ago".to_string(),
            icon: "👤".to_string(),
            color: "bg-green-500".to_string(),
        },
        AdminActivity {
            id: "2".to_string(),
            action: "Plugin installed".to_string(),
            user: "admin".to_string(),
            timestamp: "1 hour ago".to_string(),
            icon: "🧩".to_string(),
            color: "bg-blue-500".to_string(),
        },
        AdminActivity {
            id: "3".to_string(),
            action: "System backup completed".to_string(),
            user: "system".to_string(),
            timestamp: "3 hours ago".to_string(),
            icon: "💾".to_string(),
            color: "bg-gray-500".to_string(),
        },
    ]
}

fn get_system_alerts() -> Vec<SystemAlert> {
    vec![SystemAlert {
        id: "1".to_string(),
        title: "High Memory Usage".to_string(),
        message: "System memory usage is above 75%".to_string(),
        severity: "warning".to_string(),
        icon: "⚠️".to_string(),
    }]
}

fn get_admin_plugins() -> Vec<AdminPlugin> {
    vec![
        AdminPlugin {
            id: "inventory".to_string(),
            name: "Inventory Management".to_string(),
            version: "2.1.0".to_string(),
            status: "Active".to_string(),
            icon: "📦".to_string(),
        },
        AdminPlugin {
            id: "backup".to_string(),
            name: "Backup & Sync".to_string(),
            version: "3.0.1".to_string(),
            status: "Active".to_string(),
            icon: "☁️".to_string(),
        },
        AdminPlugin {
            id: "analytics".to_string(),
            name: "Analytics Dashboard".to_string(),
            version: "1.5.2".to_string(),
            status: "Inactive".to_string(),
            icon: "📊".to_string(),
        },
    ]
}

fn get_system_services() -> Vec<SystemService> {
    vec![
        SystemService {
            name: "Web Server".to_string(),
            description: "HTTP/HTTPS web server".to_string(),
            running: true,
        },
        SystemService {
            name: "Database".to_string(),
            description: "Primary database service".to_string(),
            running: true,
        },
        SystemService {
            name: "Cache Service".to_string(),
            description: "Redis cache service".to_string(),
            running: false,
        },
        SystemService {
            name: "Background Tasks".to_string(),
            description: "Task queue processor".to_string(),
            running: true,
        },
    ]
}

fn get_system_logs() -> Vec<SystemLog> {
    vec![
        SystemLog {
            id: "1".to_string(),
            level: "INFO".to_string(),
            message: "User login successful".to_string(),
            timestamp: "2024-01-15 10:30:25".to_string(),
            details: "user_id=123, ip=192.168.1.100".to_string(),
        },
        SystemLog {
            id: "2".to_string(),
            level: "WARN".to_string(),
            message: "High memory usage detected".to_string(),
            timestamp: "2024-01-15 10:25:15".to_string(),
            details: "memory_usage=78%, threshold=75%".to_string(),
        },
        SystemLog {
            id: "3".to_string(),
            level: "ERROR".to_string(),
            message: "Database connection failed".to_string(),
            timestamp: "2024-01-15 10:20:45".to_string(),
            details: "connection_timeout=5000ms, retries=3".to_string(),
        },
        SystemLog {
            id: "4".to_string(),
            level: "INFO".to_string(),
            message: "Plugin installed successfully".to_string(),
            timestamp: "2024-01-15 10:15:30".to_string(),
            details: "plugin=inventory-management, version=2.1.0".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_component_creation() {
        let _admin = rsx! { Admin {} };
    }

    #[test]
    fn test_mock_data() {
        let users = get_mock_users();
        let activities = get_recent_admin_activities();
        let alerts = get_system_alerts();

        assert!(!users.is_empty());
        assert!(!activities.is_empty());
        // alerts might be empty (no current alerts)
    }
}
