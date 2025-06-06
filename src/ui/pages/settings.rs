// src/ui/pages/settings.rs - Application settings and configuration

use dioxus::prelude::*;

use crate::ui::pages::PageWrapper;

/// Main settings page component
#[component]
pub fn Settings() -> Element {
    let mut active_section = use_signal(|| "general".to_string());

    rsx! {
        PageWrapper {
            title: "Settings".to_string(),
            subtitle: Some("Configure your application preferences".to_string()),

            div {
                class: "lg:grid lg:grid-cols-12 lg:gap-x-8",

                // Settings navigation
                aside {
                    class: "py-6 px-2 sm:px-6 lg:py-0 lg:px-0 lg:col-span-3",
                    nav {
                        class: "space-y-1",

                        // General
                        button {
                            r#type: "button",
                            class: format!(
                                "group rounded-md px-3 py-2 flex items-center text-sm font-medium w-full text-left {}",
                                if active_section() == "general" {
                                    "bg-blue-50 text-blue-700 hover:text-blue-700 hover:bg-blue-50"
                                } else {
                                    "text-gray-900 hover:text-gray-900 hover:bg-gray-50"
                                }
                            ),
                            onclick: move |_| active_section.set("general".to_string()),
                            span {
                                class: "text-lg mr-3",
                                "⚙️"
                            }
                            "General"
                        }

                        // Appearance
                        button {
                            r#type: "button",
                            class: format!(
                                "group rounded-md px-3 py-2 flex items-center text-sm font-medium w-full text-left {}",
                                if active_section() == "appearance" {
                                    "bg-blue-50 text-blue-700 hover:text-blue-700 hover:bg-blue-50"
                                } else {
                                    "text-gray-900 hover:text-gray-900 hover:bg-gray-50"
                                }
                            ),
                            onclick: move |_| active_section.set("appearance".to_string()),
                            span {
                                class: "text-lg mr-3",
                                "🎨"
                            }
                            "Appearance"
                        }

                        // Notifications
                        button {
                            r#type: "button",
                            class: format!(
                                "group rounded-md px-3 py-2 flex items-center text-sm font-medium w-full text-left {}",
                                if active_section() == "notifications" {
                                    "bg-blue-50 text-blue-700 hover:text-blue-700 hover:bg-blue-50"
                                } else {
                                    "text-gray-900 hover:text-gray-900 hover:bg-gray-50"
                                }
                            ),
                            onclick: move |_| active_section.set("notifications".to_string()),
                            span {
                                class: "text-lg mr-3",
                                "🔔"
                            }
                            "Notifications"
                        }

                        // Security
                        button {
                            r#type: "button",
                            class: format!(
                                "group rounded-md px-3 py-2 flex items-center text-sm font-medium w-full text-left {}",
                                if active_section() == "security" {
                                    "bg-blue-50 text-blue-700 hover:text-blue-700 hover:bg-blue-50"
                                } else {
                                    "text-gray-900 hover:text-gray-900 hover:bg-gray-50"
                                }
                            ),
                            onclick: move |_| active_section.set("security".to_string()),
                            span {
                                class: "text-lg mr-3",
                                "🔒"
                            }
                            "Security"
                        }

                        // System
                        button {
                            r#type: "button",
                            class: format!(
                                "group rounded-md px-3 py-2 flex items-center text-sm font-medium w-full text-left {}",
                                if active_section() == "system" {
                                    "bg-blue-50 text-blue-700 hover:text-blue-700 hover:bg-blue-50"
                                } else {
                                    "text-gray-900 hover:text-gray-900 hover:bg-gray-50"
                                }
                            ),
                            onclick: move |_| active_section.set("system".to_string()),
                            span {
                                class: "text-lg mr-3",
                                "🖥️"
                            }
                            "System"
                        }

                        // About
                        button {
                            r#type: "button",
                            class: format!(
                                "group rounded-md px-3 py-2 flex items-center text-sm font-medium w-full text-left {}",
                                if active_section() == "about" {
                                    "bg-blue-50 text-blue-700 hover:text-blue-700 hover:bg-blue-50"
                                } else {
                                    "text-gray-900 hover:text-gray-900 hover:bg-gray-50"
                                }
                            ),
                            onclick: move |_| active_section.set("about".to_string()),
                            span {
                                class: "text-lg mr-3",
                                "ℹ️"
                            }
                            "About"
                        }
                    }
                }

                // Settings content
                main {
                    class: "lg:col-span-9",
                    match active_section().as_str() {
                        "general" => rsx! { GeneralSettings {} },
                        "appearance" => rsx! { AppearanceSettings {} },
                        "notifications" => rsx! { NotificationSettings {} },
                        "security" => rsx! { SecuritySettings {} },
                        "system" => rsx! { SystemSettings {} },
                        "about" => rsx! { AboutSettings {} },
                        _ => rsx! { div { "Unknown section" } }
                    }
                }
            }
        }
    }
}

/// General settings section
#[component]
fn GeneralSettings() -> Element {
    let mut language = use_signal(|| "en".to_string());
    let mut timezone = use_signal(|| "UTC".to_string());
    let mut date_format = use_signal(|| "MM/DD/YYYY".to_string());
    let mut time_format = use_signal(|| "12h".to_string());
    let mut auto_save = use_signal(|| true);
    let mut saving = use_signal(|| false);

    let handle_save = {
        move |_| {
            saving.set(true);
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(1000).await;
                saving.set(false);
            });
        }
    };

    rsx! {
        div {
            class: "space-y-6",

            // Section header
            div {
                class: "bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6",
                div {
                    class: "md:grid md:grid-cols-3 md:gap-6",
                    div {
                        class: "md:col-span-1",
                        h3 {
                            class: "text-lg font-medium leading-6 text-gray-900",
                            "General Settings"
                        }
                        p {
                            class: "mt-1 text-sm text-gray-500",
                            "Configure basic application preferences and regional settings."
                        }
                    }
                    div {
                        class: "mt-5 md:mt-0 md:col-span-2",
                        form {
                            class: "space-y-6",
                            onsubmit: handle_save,

                            // Language selection
                            div {
                                label {
                                    r#for: "language",
                                    class: "block text-sm font-medium text-gray-700",
                                    "Language"
                                }
                                select {
                                    id: "language",
                                    name: "language",
                                    class: "mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md",
                                    value: "{language}",
                                    onchange: move |e| language.set(e.value()),
                                    option { value: "en", "English" }
                                    option { value: "es", "Español" }
                                    option { value: "fr", "Français" }
                                    option { value: "de", "Deutsch" }
                                    option { value: "ja", "日本語" }
                                    option { value: "zh", "中文" }
                                }
                            }

                            // Timezone selection
                            div {
                                label {
                                    r#for: "timezone",
                                    class: "block text-sm font-medium text-gray-700",
                                    "Timezone"
                                }
                                select {
                                    id: "timezone",
                                    name: "timezone",
                                    class: "mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md",
                                    value: "{timezone}",
                                    onchange: move |e| timezone.set(e.value()),
                                    option { value: "UTC", "UTC (Coordinated Universal Time)" }
                                    option { value: "America/New_York", "Eastern Time (ET)" }
                                    option { value: "America/Chicago", "Central Time (CT)" }
                                    option { value: "America/Denver", "Mountain Time (MT)" }
                                    option { value: "America/Los_Angeles", "Pacific Time (PT)" }
                                    option { value: "Europe/London", "London (GMT)" }
                                    option { value: "Europe/Paris", "Paris (CET)" }
                                    option { value: "Asia/Tokyo", "Tokyo (JST)" }
                                }
                            }

                            // Date format
                            div {
                                label {
                                    r#for: "date_format",
                                    class: "block text-sm font-medium text-gray-700",
                                    "Date Format"
                                }
                                select {
                                    id: "date_format",
                                    name: "date_format",
                                    class: "mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md",
                                    value: "{date_format}",
                                    onchange: move |e| date_format.set(e.value()),
                                    option { value: "MM/DD/YYYY", "MM/DD/YYYY (12/31/2024)" }
                                    option { value: "DD/MM/YYYY", "DD/MM/YYYY (31/12/2024)" }
                                    option { value: "YYYY-MM-DD", "YYYY-MM-DD (2024-12-31)" }
                                    option { value: "DD.MM.YYYY", "DD.MM.YYYY (31.12.2024)" }
                                }
                            }

                            // Time format
                            div {
                                label {
                                    r#for: "time_format",
                                    class: "block text-sm font-medium text-gray-700",
                                    "Time Format"
                                }
                                select {
                                    id: "time_format",
                                    name: "time_format",
                                    class: "mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md",
                                    value: "{time_format}",
                                    onchange: move |e| time_format.set(e.value()),
                                    option { value: "12h", "12-hour (2:30 PM)" }
                                    option { value: "24h", "24-hour (14:30)" }
                                }
                            }

                            // Auto-save toggle
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    class: "flex flex-col",
                                    label {
                                        r#for: "auto_save",
                                        class: "text-sm font-medium text-gray-700",
                                        "Auto-save"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Automatically save changes as you work"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                        if auto_save() { "bg-blue-600" } else { "bg-gray-200" }
                                    ),
                                    onclick: move |_| auto_save.set(!auto_save()),
                                    span {
                                        class: format!(
                                            "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                            if auto_save() { "translate-x-5" } else { "translate-x-0" }
                                        )
                                    }
                                }
                            }

                            // Save button
                            div {
                                class: "flex justify-end",
                                button {
                                    r#type: "submit",
                                    class: "bg-blue-600 border border-transparent rounded-md shadow-sm py-2 px-4 inline-flex justify-center text-sm font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                                    disabled: saving(),
                                    if saving() { "Saving..." } else { "Save Changes" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Appearance settings section
#[component]
fn AppearanceSettings() -> Element {
    let mut theme = use_signal(|| "light".to_string());
    let mut sidebar_collapsed = use_signal(|| false);
    let mut compact_mode = use_signal(|| false);
    let mut animations = use_signal(|| true);

    rsx! {
        div {
            class: "space-y-6",

            div {
                class: "bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6",
                div {
                    class: "md:grid md:grid-cols-3 md:gap-6",
                    div {
                        class: "md:col-span-1",
                        h3 {
                            class: "text-lg font-medium leading-6 text-gray-900",
                            "Appearance"
                        }
                        p {
                            class: "mt-1 text-sm text-gray-500",
                            "Customize the look and feel of your application."
                        }
                    }
                    div {
                        class: "mt-5 md:mt-0 md:col-span-2",
                        div {
                            class: "space-y-6",

                            // Theme selection
                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700",
                                    "Theme"
                                }
                                div {
                                    class: "mt-2 grid grid-cols-3 gap-3 sm:grid-cols-3",

                                    // Light theme
                                    label {
                                        class: format!(
                                            "cursor-pointer relative flex items-center justify-center rounded-md border py-3 px-3 text-sm font-medium uppercase hover:bg-gray-50 focus:outline-none {}",
                                            if theme() == "light" {
                                                "bg-blue-50 border-blue-200 text-blue-900"
                                            } else {
                                                "bg-white border-gray-200 text-gray-900"
                                            }
                                        ),
                                        input {
                                            r#type: "radio",
                                            name: "theme",
                                            value: "light",
                                            class: "sr-only",
                                            checked: theme() == "light",
                                            onchange: move |_| theme.set("light".to_string())
                                        }
                                        span { "☀️ Light" }
                                    }

                                    // Dark theme
                                    label {
                                        class: format!(
                                            "cursor-pointer relative flex items-center justify-center rounded-md border py-3 px-3 text-sm font-medium uppercase hover:bg-gray-50 focus:outline-none {}",
                                            if theme() == "dark" {
                                                "bg-blue-50 border-blue-200 text-blue-900"
                                            } else {
                                                "bg-white border-gray-200 text-gray-900"
                                            }
                                        ),
                                        input {
                                            r#type: "radio",
                                            name: "theme",
                                            value: "dark",
                                            class: "sr-only",
                                            checked: theme() == "dark",
                                            onchange: move |_| theme.set("dark".to_string())
                                        }
                                        span { "🌙 Dark" }
                                    }

                                    // Auto theme
                                    label {
                                        class: format!(
                                            "cursor-pointer relative flex items-center justify-center rounded-md border py-3 px-3 text-sm font-medium uppercase hover:bg-gray-50 focus:outline-none {}",
                                            if theme() == "auto" {
                                                "bg-blue-50 border-blue-200 text-blue-900"
                                            } else {
                                                "bg-white border-gray-200 text-gray-900"
                                            }
                                        ),
                                        input {
                                            r#type: "radio",
                                            name: "theme",
                                            value: "auto",
                                            class: "sr-only",
                                            checked: theme() == "auto",
                                            onchange: move |_| theme.set("auto".to_string())
                                        }
                                        span { "🔄 Auto" }
                                    }
                                }
                            }

                            // Interface options
                            div {
                                class: "space-y-4",

                                // Sidebar collapsed by default
                                div {
                                    class: "flex items-center justify-between",
                                    div {
                                        label {
                                            class: "text-sm font-medium text-gray-700",
                                            "Collapse sidebar by default"
                                        }
                                        p {
                                            class: "text-sm text-gray-500",
                                            "Start with a collapsed sidebar for more screen space"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: format!(
                                            "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                            if sidebar_collapsed() { "bg-blue-600" } else { "bg-gray-200" }
                                        ),
                                        onclick: move |_| sidebar_collapsed.set(!sidebar_collapsed()),
                                        span {
                                            class: format!(
                                                "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                                if sidebar_collapsed() { "translate-x-5" } else { "translate-x-0" }
                                            )
                                        }
                                    }
                                }

                                // Compact mode
                                div {
                                    class: "flex items-center justify-between",
                                    div {
                                        label {
                                            class: "text-sm font-medium text-gray-700",
                                            "Compact mode"
                                        }
                                        p {
                                            class: "text-sm text-gray-500",
                                            "Reduce spacing and padding for denser layouts"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: format!(
                                            "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                            if compact_mode() { "bg-blue-600" } else { "bg-gray-200" }
                                        ),
                                        onclick: move |_| compact_mode.set(!compact_mode()),
                                        span {
                                            class: format!(
                                                "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                                if compact_mode() { "translate-x-5" } else { "translate-x-0" }
                                            )
                                        }
                                    }
                                }

                                // Animations
                                div {
                                    class: "flex items-center justify-between",
                                    div {
                                        label {
                                            class: "text-sm font-medium text-gray-700",
                                            "Enable animations"
                                        }
                                        p {
                                            class: "text-sm text-gray-500",
                                            "Use animations and transitions for smoother interactions"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: format!(
                                            "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                            if animations() { "bg-blue-600" } else { "bg-gray-200" }
                                        ),
                                        onclick: move |_| animations.set(!animations()),
                                        span {
                                            class: format!(
                                                "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                                if animations() { "translate-x-5" } else { "translate-x-0" }
                                            )
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

/// Notification settings section
#[component]
fn NotificationSettings() -> Element {
    let mut email_notifications = use_signal(|| true);
    let mut push_notifications = use_signal(|| false);
    let mut desktop_notifications = use_signal(|| true);
    let mut sound_enabled = use_signal(|| false);

    rsx! {
        div {
            class: "space-y-6",

            div {
                class: "bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6",
                div {
                    class: "md:grid md:grid-cols-3 md:gap-6",
                    div {
                        class: "md:col-span-1",
                        h3 {
                            class: "text-lg font-medium leading-6 text-gray-900",
                            "Notifications"
                        }
                        p {
                            class: "mt-1 text-sm text-gray-500",
                            "Choose how you want to be notified about updates and events."
                        }
                    }
                    div {
                        class: "mt-5 md:mt-0 md:col-span-2",
                        div {
                            class: "space-y-6",

                            // Email notifications
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    label {
                                        class: "text-sm font-medium text-gray-700",
                                        "Email notifications"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Receive important updates via email"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                        if email_notifications() { "bg-blue-600" } else { "bg-gray-200" }
                                    ),
                                    onclick: move |_| email_notifications.set(!email_notifications()),
                                    span {
                                        class: format!(
                                            "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                            if email_notifications() { "translate-x-5" } else { "translate-x-0" }
                                        )
                                    }
                                }
                            }

                            // Push notifications
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    label {
                                        class: "text-sm font-medium text-gray-700",
                                        "Push notifications"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Show notifications in your browser"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                        if push_notifications() { "bg-blue-600" } else { "bg-gray-200" }
                                    ),
                                    onclick: move |_| push_notifications.set(!push_notifications()),
                                    span {
                                        class: format!(
                                            "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                            if push_notifications() { "translate-x-5" } else { "translate-x-0" }
                                        )
                                    }
                                }
                            }

                            // Desktop notifications
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    label {
                                        class: "text-sm font-medium text-gray-700",
                                        "Desktop notifications"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Show system notifications on your desktop"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                        if desktop_notifications() { "bg-blue-600" } else { "bg-gray-200" }
                                    ),
                                    onclick: move |_| desktop_notifications.set(!desktop_notifications()),
                                    span {
                                        class: format!(
                                            "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                            if desktop_notifications() { "translate-x-5" } else { "translate-x-0" }
                                        )
                                    }
                                }
                            }

                            // Sound notifications
                            div {
                                class: "flex items-center justify-between",
                                div {
                                    label {
                                        class: "text-sm font-medium text-gray-700",
                                        "Sound notifications"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Play sounds for notifications"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: format!(
                                        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 {}",
                                        if sound_enabled() { "bg-blue-600" } else { "bg-gray-200" }
                                    ),
                                    onclick: move |_| sound_enabled.set(!sound_enabled()),
                                    span {
                                        class: format!(
                                            "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 {}",
                                            if sound_enabled() { "translate-x-5" } else { "translate-x-0" }
                                        )
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

/// Security settings section
#[component]
fn SecuritySettings() -> Element {
    rsx! {
        div {
            class: "space-y-6",

            div {
                class: "bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6",
                div {
                    class: "md:grid md:grid-cols-3 md:gap-6",
                    div {
                        class: "md:col-span-1",
                        h3 {
                            class: "text-lg font-medium leading-6 text-gray-900",
                            "Security"
                        }
                        p {
                            class: "mt-1 text-sm text-gray-500",
                            "Manage your account security settings and preferences."
                        }
                    }
                    div {
                        class: "mt-5 md:mt-0 md:col-span-2",
                        div {
                            class: "space-y-6",

                            // Change password
                            div {
                                class: "flex items-center justify-between py-4 border-b border-gray-200",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-gray-900",
                                        "Password"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Last changed 3 months ago"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "bg-white border border-gray-300 rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Change Password"
                                }
                            }

                            // Two-factor authentication
                            div {
                                class: "flex items-center justify-between py-4 border-b border-gray-200",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-gray-900",
                                        "Two-Factor Authentication"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Add an extra layer of security to your account"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "bg-blue-600 border border-transparent rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Enable 2FA"
                                }
                            }

                            // Session management
                            div {
                                class: "flex items-center justify-between py-4 border-b border-gray-200",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-gray-900",
                                        "Active Sessions"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Manage devices where you're signed in"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "bg-white border border-gray-300 rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Manage Sessions"
                                }
                            }

                            // Account deletion
                            div {
                                class: "pt-4",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-red-900",
                                        "Danger Zone"
                                    }
                                    p {
                                        class: "text-sm text-red-600 mt-1",
                                        "Permanently delete your account and all associated data. This action cannot be undone."
                                    }
                                    button {
                                        r#type: "button",
                                        class: "mt-3 bg-red-600 border border-transparent rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-white hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500",
                                        "Delete Account"
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

/// System settings section
#[component]
fn SystemSettings() -> Element {
    let build = option_env!("BUILD_HASH").unwrap_or("dev");

    rsx! {
        div {
            class: "space-y-6",

            div {
                class: "bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6",
                div {
                    class: "md:grid md:grid-cols-3 md:gap-6",
                    div {
                        class: "md:col-span-1",
                        h3 {
                            class: "text-lg font-medium leading-6 text-gray-900",
                            "System"
                        }
                        p {
                            class: "mt-1 text-sm text-gray-500",
                            "System-level settings and maintenance options."
                        }
                    }
                    div {
                        class: "mt-5 md:mt-0 md:col-span-2",
                        div {
                            class: "space-y-6",

                            // Cache management
                            div {
                                class: "flex items-center justify-between py-4 border-b border-gray-200",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-gray-900",
                                        "Cache"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Clear application cache to free up space"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "bg-white border border-gray-300 rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Clear Cache"
                                }
                            }

                            // Data export
                            div {
                                class: "flex items-center justify-between py-4 border-b border-gray-200",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-gray-900",
                                        "Data Export"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "Download a copy of your data"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "bg-blue-600 border border-transparent rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "Export Data"
                                }
                            }

                            // System logs
                            div {
                                class: "flex items-center justify-between py-4 border-b border-gray-200",
                                div {
                                    h4 {
                                        class: "text-sm font-medium text-gray-900",
                                        "System Logs"
                                    }
                                    p {
                                        class: "text-sm text-gray-500",
                                        "View and download system log files"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "bg-white border border-gray-300 rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                                    "View Logs"
                                }
                            }

                            // System information
                            div {
                                class: "pt-4",
                                div {
                                    class: "bg-gray-50 rounded-lg p-4",
                                    h4 {
                                        class: "text-sm font-medium text-gray-900 mb-3",
                                        "System Information"
                                    }
                                    dl {
                                        class: "grid grid-cols-1 gap-x-4 gap-y-2 sm:grid-cols-2",
                                        div {
                                            dt {
                                                class: "text-sm font-medium text-gray-500",
                                                "Version"
                                            }
                                            dd {
                                                class: "text-sm text-gray-900",
                                                "{crate::VERSION}"
                                            }
                                        }
                                        div {
                                            dt {
                                                class: "text-sm font-medium text-gray-500",
                                                "Platform"
                                            }
                                            dd {
                                                class: "text-sm text-gray-900",
                                                "Web (WASM)"
                                            }
                                        }
                                        div {
                                            dt {
                                                class: "text-sm font-medium text-gray-500",
                                                "Build"
                                            }
                                            dd {
                                                class: "text-sm text-gray-900",
                                                "{build}"
                                            }
                                        }
                                        div {
                                            dt {
                                                class: "text-sm font-medium text-gray-500",
                                                "User Agent"
                                            }
                                            dd {
                                                class: "text-sm text-gray-900",
                                                "Qorzen Application"
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
}

/// About section
#[component]
fn AboutSettings() -> Element {
    rsx! {
        div {
            class: "space-y-6",

            div {
                class: "bg-white shadow px-4 py-5 sm:rounded-lg sm:p-6",
                div {
                    class: "text-center",
                    div {
                        class: "mx-auto h-16 w-16 bg-blue-600 rounded-xl flex items-center justify-center mb-4",
                        span {
                            class: "text-white font-bold text-2xl",
                            "Q"
                        }
                    }
                    h2 {
                        class: "text-2xl font-bold text-gray-900",
                        "Qorzen"
                    }
                    p {
                        class: "text-sm text-gray-500 mt-1",
                        "Version {crate::VERSION}"
                    }
                    p {
                        class: "text-gray-600 mt-4 max-w-2xl mx-auto",
                        "A modular, cross-platform application framework built with Rust and Dioxus.
                         Designed for modern development workflows with extensibility through plugins."
                    }

                    div {
                        class: "mt-8 flex justify-center space-x-6",
                        a {
                            href: "https://github.com/qorzen/qorzen-oxide",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "text-gray-400 hover:text-gray-500",
                            span {
                                class: "sr-only",
                                "GitHub"
                            }
                            "🐙 GitHub"
                        }
                        a {
                            href: "https://docs.qorzen.com",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "text-gray-400 hover:text-gray-500",
                            "📚 Documentation"
                        }
                        a {
                            href: "mailto:support@qorzen.com",
                            class: "text-gray-400 hover:text-gray-500",
                            "📧 Support"
                        }
                    }

                    div {
                        class: "mt-8 pt-8 border-t border-gray-200",
                        p {
                            class: "text-sm text-gray-500",
                            "© 2024 Qorzen. Built with ❤️ and 🦀"
                        }
                        p {
                            class: "text-xs text-gray-400 mt-2",
                            "Licensed under the MIT License"
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

    #[test]
    fn test_settings_component_creation() {
        let _settings = rsx! { Settings {} };
    }

    #[test]
    fn test_general_settings_creation() {
        let _general = rsx! { GeneralSettings {} };
    }

    #[test]
    fn test_appearance_settings_creation() {
        let _appearance = rsx! { AppearanceSettings {} };
    }
}
