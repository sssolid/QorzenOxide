// src/ui/layout/header.rs - Top navigation header with branding, user menu, and notifications

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::ui::{
    router::Route,
    state::{auth::use_logout, ui::use_notifications, use_app_state},
};

/// Header component props
#[derive(Props, Clone, PartialEq)]
pub struct HeaderProps {
    /// Callback for mobile menu toggle
    pub on_menu_toggle: Callback<()>,
    /// Callback for sidebar toggle
    pub on_sidebar_toggle: Callback<()>,
}

/// Main header component
#[component]
pub fn Header(props: HeaderProps) -> Element {
    let app_state = use_app_state();
    let logout = use_logout();
    let (notifications, remove_notification, mark_read, clear_all) = use_notifications();

    // State for dropdowns
    let mut user_menu_open = use_signal(|| false);
    let mut notifications_open = use_signal(|| false);

    // Count unread notifications
    let unread_count = notifications.iter().filter(|n| !n.read).count();
    
    let left_side_mobile_button = rsx! {
        // Mobile menu button
        button {
            r#type: "button",
            class: "inline-flex items-center justify-center p-2 rounded-md text-gray-400 hover:text-gray-500 hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-blue-500 lg:hidden",
            onclick: move |_| props.on_menu_toggle.call(()),
            span {
                class: "sr-only",
                "Open main menu"
            }
            // Hamburger icon
            svg {
                class: "h-6 w-6",
                xmlns: "http://www.w3.org/2000/svg",
                fill: "none",
                view_box: "0 0 24 24",
                stroke: "currentColor",
                path {
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    stroke_width: "2",
                    d: "M4 6h16M4 12h16M4 18h16"
                }
            }
        }
    };
    
    let left_side_desktop_sidebar_toggle = rsx! {
        // Desktop sidebar toggle
        button {
            r#type: "button",
            class: "hidden lg:inline-flex items-center justify-center p-2 rounded-md text-gray-400 hover:text-gray-500 hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-blue-500 mr-4",
            onclick: move |_| props.on_sidebar_toggle.call(()),
            span {
                class: "sr-only",
                "Toggle sidebar"
            }
            // Menu icon
            svg {
                class: "h-5 w-5",
                xmlns: "http://www.w3.org/2000/svg",
                fill: "none",
                view_box: "0 0 24 24",
                stroke: "currentColor",
                path {
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    stroke_width: "2",
                    d: "M4 6h16M4 12h8m-8 6h16"
                }
            }
        }
    };
        
    let left_side_logo = rsx! {
        // Logo
        Link {
            to: Route::Dashboard {},
            class: "flex items-center",
            div {
                class: "flex-shrink-0 flex items-center",
                // Logo placeholder - in real app this would be an image
                div {
                    class: "h-8 w-8 bg-blue-600 rounded-lg flex items-center justify-center",
                    span {
                        class: "text-white font-bold text-sm",
                        "Q"
                    }
                }
                span {
                    class: "ml-2 text-xl font-bold text-gray-900 hidden sm:block",
                    "Qorzen"
                }
            }
        }
    };

    let left_side = rsx! {
            // Left side - Logo and mobile menu button
        div {
            class: "flex items-center",
            {left_side_mobile_button}
            {left_side_desktop_sidebar_toggle}
            {left_side_logo}
            }
    };
    
    let right_side_search_bar = rsx! {
        // Search bar (desktop only)
        div {
            class: "hidden md:block",
            div {
                class: "relative",
                input {
                    r#type: "text",
                    placeholder: "Search...",
                    class: "block w-64 pr-10 border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
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
    };
    
    let right_side_notifications_dropdown_bell_icon = rsx! {
        // Bell icon
        svg {
            class: "h-6 w-6",
            xmlns: "http://www.w3.org/2000/svg",
            fill: "none",
            view_box: "0 0 24 24",
            stroke: "currentColor",
            path {
                stroke_linecap: "round",
                stroke_linejoin: "round",
                stroke_width: "2",
                d: "M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
            }
        }
    };
    
    let right_side_notifications_dropdown_notification_badge = rsx! {
        // Notification badge
            if unread_count > 0 {
                span {
                    class: "absolute -top-1 -right-1 h-5 w-5 bg-red-500 text-white text-xs rounded-full flex items-center justify-center",
                    "{unread_count}"
                }
            }
    };
    
    let right_side_notifications_dropdown_open_header = rsx! {
        // Header
        div {
            class: "px-4 py-2 border-b border-gray-200 flex justify-between items-center",
            h3 {
                class: "text-sm font-medium text-gray-900",
                "Notifications"
            }
            if !notifications.is_empty() {
                button {
                    r#type: "button",
                    class: "text-xs text-blue-600 hover:text-blue-800",
                    onclick: move |_| {
                        clear_all.call(());
                        notifications_open.set(false);
                    },
                    "Clear all"
                }
            }
        }
    };

    fn fmt_time(ts: chrono::DateTime<chrono::Utc>) -> String {
        ts.format("%H:%M").to_string()
    }
    
    let right_side_notifications_dropdown_open_list = rsx! {
        // Notifications list
        div {
            class: "max-h-96 overflow-y-auto",
            if notifications.is_empty() {
                div {
                    class: "px-4 py-8 text-center text-sm text-gray-500",
                    "No notifications"
                }
            } else {
                for notification in notifications.clone() {
                    div {
                        key: notification.id,
                        class: format!(
                            "px-4 py-3 hover:bg-gray-50 border-b border-gray-100 last:border-b-0 {}",
                            if notification.read { "opacity-75" } else { "" }
                        ),
                        div {
                            class: "flex justify-between items-start",
                            div {
                                class: "flex-1 min-w-0",
                                p {
                                    class: "text-sm font-medium text-gray-900 truncate",
                                    "{notification.title}"
                                }
                                p {
                                    class: "text-sm text-gray-500 mt-1",
                                    "{notification.message}"
                                }
                                p {
                                    class: "text-xs text-gray-400 mt-1",
                                    {fmt_time(notification.timestamp)}
                                }
                            }
                            div {
                                class: "flex space-x-1 ml-2",
                                if !notification.read {
                                    button {
                                        r#type: "button",
                                        class: "text-xs text-blue-600 hover:text-blue-800",
                                        onclick: move |_| mark_read.call(notification.id),
                                        "Mark read"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "text-xs text-red-600 hover:text-red-800",
                                    onclick: move |_| remove_notification.call(notification.id),
                                    "×"
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    
    let right_side_notifications_dropdown = rsx! {
        // Notifications dropdown
        div {
            class: "relative",
            button {
                r#type: "button",
                class: "relative p-2 text-gray-400 hover:text-gray-500 hover:bg-gray-100 rounded-full focus:outline-none focus:ring-2 focus:ring-blue-500",
                onclick: move |_| notifications_open.set(!notifications_open()),
                span {
                    class: "sr-only",
                    "View notifications"
                }
                {right_side_notifications_dropdown_bell_icon}
                {right_side_notifications_dropdown_notification_badge}
            }

            // Notifications dropdown
            if notifications_open() {
                div {
                    class: "absolute right-0 mt-2 w-80 bg-white rounded-md shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none z-50",
                    div {
                        class: "py-1",
                        {right_side_notifications_dropdown_open_header}
                        {right_side_notifications_dropdown_open_list}
                    }
                }
            }
        }
    };
    
    let user_menu_dropdown = rsx! {
        // User menu dropdown
        div {
            class: "relative",
            button {
                r#type: "button",
                class: "flex items-center text-sm rounded-full focus:outline-none focus:ring-2 focus:ring-blue-500",
                onclick: move |_| user_menu_open.set(!user_menu_open()),
                span {
                    class: "sr-only",
                    "Open user menu"
                }
                // User avatar or initials
                if let Some(user) = &app_state.current_user {
                    if let Some(avatar_url) = &user.profile.avatar_url {
                        img {
                            class: "h-8 w-8 rounded-full",
                            src: "{avatar_url}",
                            alt: "{user.profile.display_name}"
                        }
                    } else {
                        div {
                            class: "h-8 w-8 rounded-full bg-blue-600 flex items-center justify-center",
                            span {
                                class: "text-sm font-medium text-white",
                                "{user.profile.display_name.chars().next().unwrap_or('U')}"
                            }
                        }
                    }
                } else {
                    div {
                        class: "h-8 w-8 rounded-full bg-gray-300 flex items-center justify-center",
                        span {
                            class: "text-sm font-medium text-gray-600",
                            "?"
                        }
                    }
                }
            }

            // User dropdown menu
            if user_menu_open() {
                div {
                    class: "absolute right-0 mt-2 w-48 bg-white rounded-md shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none z-50",
                    div {
                        class: "py-1",
                        if let Some(user) = &app_state.current_user {
                            // User info
                            div {
                                class: "px-4 py-2 border-b border-gray-200",
                                p {
                                    class: "text-sm font-medium text-gray-900",
                                    "{user.profile.display_name}"
                                }
                                p {
                                    class: "text-sm text-gray-500",
                                    "{user.email}"
                                }
                            }

                            // Menu items
                            Link {
                                to: Route::Profile {},
                                class: "block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100",
                                onclick: move |_| user_menu_open.set(false),
                                "👤 Profile"
                            }
                            Link {
                                to: Route::Settings {},
                                class: "block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100",
                                onclick: move |_| user_menu_open.set(false),
                                "⚙️ Settings"
                            }
                            div {
                                class: "border-t border-gray-200"
                            }
                            button {
                                r#type: "button",
                                class: "block w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-100",
                                onclick: move |_| {
                                    logout.call(());
                                    user_menu_open.set(false);
                                },
                                "🚪 Sign out"
                            }
                        } else {
                            Link {
                                to: Route::Login {},
                                class: "block px-4 py-2 text-sm text-gray-700 hover:bg-gray-100",
                                onclick: move |_| user_menu_open.set(false),
                                "🔐 Sign in"
                            }
                        }
                    }
                }
            }
        }
    };
    
    let right_side = rsx! {
        // Right side - Search, notifications, user menu
        div {
            class: "flex items-center space-x-4",

            {right_side_search_bar}
            {right_side_notifications_dropdown}
            {user_menu_dropdown}
        }
    };

    rsx! {
        header {
            class: "bg-white shadow-sm border-b border-gray-200 relative z-50",
            div {
                class: "mx-auto max-w-full px-4 sm:px-6 lg:px-8",
                div {
                    class: "flex justify-between items-center h-16",
                    {left_side}
                    {right_side}
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
    fn test_header_component_creation() {
        let on_menu_toggle = Callback::new(|_| {});
        let on_sidebar_toggle = Callback::new(|_| {});

        let _header = rsx! {
            Header {
                on_menu_toggle: on_menu_toggle,
                on_sidebar_toggle: on_sidebar_toggle
            }
        };
    }
}
