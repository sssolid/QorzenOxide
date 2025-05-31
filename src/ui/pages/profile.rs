// src/ui/pages/profile.rs - User profile management page

use dioxus::prelude::*;

use crate::ui::{
    pages::PageWrapper,
    state::{use_app_dispatch, use_app_state},
};

/// Profile page component
#[component]
pub fn Profile() -> Element {
    let app_state = use_app_state();
    let _dispatch = use_app_dispatch();

    // Clone user data to avoid borrowing issues
    let current_user = app_state.current_user.clone();

    // Form state
    let mut display_name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut bio = use_signal(String::new);
    let mut department = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut phone = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut save_message = use_signal(|| None::<String>);

    // Initialize form with current user data
    use_effect({
        let current_user = current_user.clone();
        move || {
            if let Some(user) = &current_user {
                display_name.set(user.profile.display_name.clone());
                email.set(user.email.clone());
                bio.set(user.profile.bio.clone().unwrap_or_default());
                department.set(user.profile.department.clone().unwrap_or_default());
                title.set(user.profile.title.clone().unwrap_or_default());
                phone.set(user.profile.contact_info.phone.clone().unwrap_or_default());
            }
        }
    });

    let handle_save = {
        // let dispatch = dispatch.clone();
        move |_| {
            save_message.set(None);
            saving.set(true);

            // Simulate save operation
            spawn({
                // let dispatch = dispatch.clone();
                async move {
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(1000).await;

                    // In a real app, this would update the user via API
                    save_message.set(Some("Profile updated successfully!".to_string()));
                    saving.set(false);
                }
            });
        }
    };

    rsx! {
        PageWrapper {
            title: "Profile".to_string(),
            subtitle: Some("Manage your account settings and personal information".to_string()),

            div {
                class: "space-y-6",

                ProfileOverviewCard { user: current_user.clone() }
                EditProfileForm {
                    display_name: display_name,
                    email: email,
                    bio: bio,
                    department: department,
                    title: title,
                    phone: phone,
                    saving: saving,
                    save_message: save_message,
                    on_save: handle_save
                }
                SecuritySection {}
                PreferencesSection {}
            }
        }
    }
}

/// Profile overview card component
#[component]
fn ProfileOverviewCard(user: Option<crate::ui::state::User>) -> Element {
    let profile_header = if let Some(user) = &user {
        let avatar_section = if let Some(avatar_url) = &user.profile.avatar_url {
            rsx! {
                img {
                    class: "h-20 w-20 rounded-full border-4 border-white",
                    src: "{avatar_url}",
                    alt: "{user.profile.display_name}"
                }
            }
        } else {
            let initial = user.profile.display_name.chars().next().unwrap_or('U');
            rsx! {
                div {
                    class: "h-20 w-20 rounded-full bg-white bg-opacity-20 flex items-center justify-center border-4 border-white",
                    span {
                        class: "text-3xl font-bold text-white",
                        "{initial}"
                    }
                }
            }
        };

        let user_title = if let Some(title) = &user.profile.title {
            rsx! {
                p {
                    class: "text-blue-100",
                    "{title}"
                }
            }
        } else {
            rsx! {}
        };

        rsx! {
            div {
                class: "flex items-center",
                div {
                    class: "flex-shrink-0",
                    {avatar_section}
                }
                div {
                    class: "ml-6",
                    h1 {
                        class: "text-2xl font-bold text-white",
                        "{user.profile.display_name}"
                    }
                    p {
                        class: "text-blue-100",
                        "{user.email}"
                    }
                    {user_title}
                    p {
                        class: "text-blue-200 text-sm mt-2",
                        "Member since {user.created_at.format(\"%B %Y\")}"
                    }
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "text-center",
                p {
                    class: "text-white text-lg",
                    "No user data available"
                }
            }
        }
    };

    rsx! {
        div {
            class: "bg-white shadow rounded-lg overflow-hidden",
            div {
                class: "bg-gradient-to-r from-blue-500 to-purple-600 px-6 py-8",
                {profile_header}
            }
        }
    }
}

/// Edit profile form component
#[component]
fn EditProfileForm(
    display_name: Signal<String>,
    email: Signal<String>,
    bio: Signal<String>,
    department: Signal<String>,
    title: Signal<String>,
    phone: Signal<String>,
    saving: Signal<bool>,
    save_message: Signal<Option<String>>,
    on_save: Callback<Event<FormData>>,
) -> Element {
    let success_message = if let Some(message) = save_message() {
        rsx! {
            div {
                class: "mb-6 rounded-md bg-green-50 p-4",
                div {
                    class: "flex",
                    div {
                        class: "flex-shrink-0",
                        svg {
                            class: "h-5 w-5 text-green-400",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 20 20",
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                    div {
                        class: "ml-3",
                        p {
                            class: "text-sm font-medium text-green-800",
                            "{message}"
                        }
                    }
                }
            }
        }
    } else {
        rsx! {}
    };

    let form_fields = rsx! {
        div {
            class: "grid grid-cols-1 gap-y-6 gap-x-4 sm:grid-cols-2",

            // Display name
            div {
                class: "sm:col-span-1",
                label {
                    r#for: "display_name",
                    class: "block text-sm font-medium text-gray-700",
                    "Display Name"
                }
                div {
                    class: "mt-1",
                    input {
                        r#type: "text",
                        name: "display_name",
                        id: "display_name",
                        class: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md",
                        value: "{display_name}",
                        oninput: move |e| display_name.set(e.value())
                    }
                }
            }

            // Email
            div {
                class: "sm:col-span-1",
                label {
                    r#for: "email",
                    class: "block text-sm font-medium text-gray-700",
                    "Email Address"
                }
                div {
                    class: "mt-1",
                    input {
                        r#type: "email",
                        name: "email",
                        id: "email",
                        class: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md",
                        value: "{email}",
                        oninput: move |e| email.set(e.value())
                    }
                }
            }

            // Department
            div {
                class: "sm:col-span-1",
                label {
                    r#for: "department",
                    class: "block text-sm font-medium text-gray-700",
                    "Department"
                }
                div {
                    class: "mt-1",
                    input {
                        r#type: "text",
                        name: "department",
                        id: "department",
                        class: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md",
                        value: "{department}",
                        oninput: move |e| department.set(e.value())
                    }
                }
            }

            // Job title
            div {
                class: "sm:col-span-1",
                label {
                    r#for: "title",
                    class: "block text-sm font-medium text-gray-700",
                    "Job Title"
                }
                div {
                    class: "mt-1",
                    input {
                        r#type: "text",
                        name: "title",
                        id: "title",
                        class: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md",
                        value: "{title}",
                        oninput: move |e| title.set(e.value())
                    }
                }
            }

            // Phone
            div {
                class: "sm:col-span-1",
                label {
                    r#for: "phone",
                    class: "block text-sm font-medium text-gray-700",
                    "Phone Number"
                }
                div {
                    class: "mt-1",
                    input {
                        r#type: "tel",
                        name: "phone",
                        id: "phone",
                        class: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md",
                        value: "{phone}",
                        oninput: move |e| phone.set(e.value())
                    }
                }
            }

            // Bio
            div {
                class: "sm:col-span-2",
                label {
                    r#for: "bio",
                    class: "block text-sm font-medium text-gray-700",
                    "Bio"
                }
                div {
                    class: "mt-1",
                    textarea {
                        id: "bio",
                        name: "bio",
                        rows: "3",
                        class: "shadow-sm focus:ring-blue-500 focus:border-blue-500 block w-full sm:text-sm border-gray-300 rounded-md",
                        placeholder: "Tell us about yourself...",
                        value: "{bio}",
                        oninput: move |e| bio.set(e.value())
                    }
                }
                p {
                    class: "mt-2 text-sm text-gray-500",
                    "Brief description for your profile."
                }
            }
        }
    };

    let action_buttons = rsx! {
        div {
            class: "pt-6 border-t border-gray-200 flex justify-end space-x-3",
            button {
                r#type: "button",
                class: "bg-white py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                "Cancel"
            }
            SaveButton { saving: saving }
        }
    };

    rsx! {
        div {
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "Personal Information"
                }
                p {
                    class: "mt-1 max-w-2xl text-sm text-gray-500",
                    "Update your personal details and contact information."
                }
            }

            form {
                class: "px-4 py-5 sm:p-6",
                onsubmit: on_save,

                {success_message}
                {form_fields}
                {action_buttons}
            }
        }
    }
}

/// Save button component
#[component]
fn SaveButton(saving: Signal<bool>) -> Element {
    if saving() {
        rsx! {
            button {
                r#type: "submit",
                class: "inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50",
                disabled: true,
                span {
                    class: "flex items-center",
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
                            stroke_width: "4"
                        }
                        path {
                            class: "opacity-75",
                            fill: "currentColor",
                            d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                        }
                    }
                    "Saving..."
                }
            }
        }
    } else {
        rsx! {
            button {
                r#type: "submit",
                class: "inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                "Save Changes"
            }
        }
    }
}

/// Security settings section
#[component]
fn SecuritySection() -> Element {
    let security_items = rsx! {
        div {
            class: "space-y-6",

            // Change password
            div {
                class: "flex items-center justify-between",
                div {
                    h4 {
                        class: "text-sm font-medium text-gray-900",
                        "Password"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "Last updated 3 months ago"
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
                class: "flex items-center justify-between",
                div {
                    h4 {
                        class: "text-sm font-medium text-gray-900",
                        "Two-factor Authentication"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "Add an extra layer of security to your account"
                    }
                }
                button {
                    r#type: "button",
                    class: "bg-white border border-gray-300 rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                    "Enable 2FA"
                }
            }

            // Sessions
            div {
                class: "flex items-center justify-between",
                div {
                    h4 {
                        class: "text-sm font-medium text-gray-900",
                        "Active Sessions"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "Manage your active sessions on other devices"
                    }
                }
                button {
                    r#type: "button",
                    class: "bg-white border border-gray-300 rounded-md shadow-sm py-2 px-3 text-sm leading-4 font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500",
                    "View Sessions"
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
                    "Security"
                }
                p {
                    class: "mt-1 max-w-2xl text-sm text-gray-500",
                    "Manage your account security settings."
                }
            }
            div {
                class: "px-4 py-5 sm:p-6",
                {security_items}
            }
        }
    }
}

/// Preferences section
#[component]
fn PreferencesSection() -> Element {
    let mut theme = use_signal(|| "light".to_string());
    let mut language = use_signal(|| "en".to_string());
    let notifications = use_signal(|| true);

    let preferences_items = rsx! {
        div {
            class: "space-y-6",

            // Theme selection
            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "Theme"
                }
                div {
                    class: "mt-1",
                    select {
                        class: "block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md",
                        value: "{theme}",
                        onchange: move |e| theme.set(e.value()),
                        option { value: "light", "Light" }
                        option { value: "dark", "Dark" }
                        option { value: "auto", "Auto (System)" }
                    }
                }
            }

            // Language selection
            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "Language"
                }
                div {
                    class: "mt-1",
                    select {
                        class: "block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm rounded-md",
                        value: "{language}",
                        onchange: move |e| language.set(e.value()),
                        option { value: "en", "English" }
                        option { value: "es", "Español" }
                        option { value: "fr", "Français" }
                        option { value: "de", "Deutsch" }
                    }
                }
            }

            // Notifications toggle
            NotificationToggle { notifications: notifications }
        }
    };

    rsx! {
        div {
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "Preferences"
                }
                p {
                    class: "mt-1 max-w-2xl text-sm text-gray-500",
                    "Customize your application experience."
                }
            }
            div {
                class: "px-4 py-5 sm:p-6",
                {preferences_items}
            }
        }
    }
}

/// Notification toggle component
#[component]
fn NotificationToggle(notifications: Signal<bool>) -> Element {
    let toggle_class = if notifications() {
        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 bg-blue-600"
    } else {
        "relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 bg-gray-200"
    };

    let toggle_dot_class = if notifications() {
        "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 translate-x-5"
    } else {
        "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200 translate-x-0"
    };

    rsx! {
        div {
            class: "flex items-center justify-between",
            div {
                h4 {
                    class: "text-sm font-medium text-gray-900",
                    "Email Notifications"
                }
                p {
                    class: "text-sm text-gray-500",
                    "Receive email updates about your account activity"
                }
            }
            button {
                r#type: "button",
                class: "{toggle_class}",
                onclick: move |_| notifications.set(!notifications()),
                span {
                    class: "{toggle_dot_class}"
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
    fn test_profile_component_creation() {
        let _profile = rsx! { Profile {} };
    }

    #[test]
    fn test_security_section_creation() {
        let _security = rsx! { SecuritySection {} };
    }

    #[test]
    fn test_preferences_section_creation() {
        let _preferences = rsx! { PreferencesSection {} };
    }
}
