// src/ui/pages/login.rs - Authentication login page

use dioxus::prelude::*;
#[allow(unused_imports)]
use dioxus_router::prelude::*;

use crate::{
    auth::Credentials,
    ui::{
        router::Route,
        state::{auth::use_login, use_app_state},
    },
};

/// Login page component
#[component]
pub fn Login() -> Element {
    let app_state = use_app_state();
    let login = use_login();
    let navigator = use_navigator();

    // Form state
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut remember_me = use_signal(|| false);
    let mut login_error = use_signal(|| None::<String>);

    // Redirect if already authenticated
    use_effect({
        move || {
            if app_state.current_user.is_some() {
                navigator.push(Route::Dashboard {});
            }
        }
    });

    let handle_submit = {
        move |_| {
            let credentials = Credentials::Password {
                username: username(),
                password: password(),
            };

            login_error.set(None);

            // Validate form
            if username().trim().is_empty() {
                login_error.set(Some("Username is required".to_string()));
                return;
            }

            if password().trim().is_empty() {
                login_error.set(Some("Password is required".to_string()));
                return;
            }

            // Attempt login
            spawn({
                let navigator = navigator;
                async move {
                    login.call(credentials);

                    // Wait a bit for state to update, then navigate
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(1500).await;

                    navigator.push(Route::Dashboard {});
                }
            });
        }
    };

    rsx! {
        div {
            class: "min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8",
            div {
                class: "max-w-md w-full space-y-8",

                // Header
                div {
                    class: "text-center",
                    div {
                        class: "mx-auto h-32 w-32 flex items-center justify-center mb-4",
                        img {
                            class: "h-32 w-32", // adjust as needed
                            src: "/static/qorzen.ico", // or wherever your image is
                            alt: "Q Logo"
                        }
                    }
                    h2 {
                        class: "text-3xl font-extrabold text-gray-900",
                        "Sign in to Qorzen"
                    }
                    p {
                        class: "mt-2 text-sm text-gray-600",
                        "Welcome back! Please sign in to your account."
                    }
                }

                // Login form
                form {
                    class: "mt-8 space-y-6",
                    onsubmit: handle_submit,

                    // Error message
                    if let Some(error) = login_error() {
                        div {
                            class: "rounded-md bg-red-50 p-4",
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
                                            d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z",
                                            clip_rule: "evenodd"
                                        }
                                    }
                                }
                                div {
                                    class: "ml-3",
                                    h3 {
                                        class: "text-sm font-medium text-red-800",
                                        "Authentication Error"
                                    }
                                    div {
                                        class: "mt-2 text-sm text-red-700",
                                        "{error}"
                                    }
                                }
                            }
                        }
                    }

                    // Demo credentials notice
                    div {
                        class: "rounded-md bg-blue-50 p-4",
                        div {
                            class: "flex",
                            div {
                                class: "flex-shrink-0",
                                svg {
                                    class: "h-5 w-5 text-blue-400",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    view_box: "0 0 20 20",
                                    fill: "currentColor",
                                    path {
                                        fill_rule: "evenodd",
                                        d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z",
                                        clip_rule: "evenodd"
                                    }
                                }
                            }
                            div {
                                class: "ml-3 flex-1 md:flex md:justify-between",
                                p {
                                    class: "text-sm text-blue-700",
                                    "Demo Mode: Enter any username and password to continue."
                                }
                            }
                        }
                    }

                    div {
                        class: "space-y-4",

                        // Username field
                        div {
                            label {
                                r#for: "username",
                                class: "block text-sm font-medium text-gray-700",
                                "Username or Email"
                            }
                            div {
                                class: "mt-1 relative",
                                input {
                                    id: "username",
                                    name: "username",
                                    r#type: "text",
                                    autocomplete: "username",
                                    required: true,
                                    class: "appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm",
                                    placeholder: "Enter your username or email",
                                    value: "{username}",
                                    oninput: move |e| username.set(e.value())
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
                                            d: "M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z",
                                            clip_rule: "evenodd"
                                        }
                                    }
                                }
                            }
                        }

                        // Password field
                        div {
                            label {
                                r#for: "password",
                                class: "block text-sm font-medium text-gray-700",
                                "Password"
                            }
                            div {
                                class: "mt-1 relative",
                                input {
                                    id: "password",
                                    name: "password",
                                    r#type: "password",
                                    autocomplete: "current-password",
                                    required: true,
                                    class: "appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm",
                                    placeholder: "Enter your password",
                                    value: "{password}",
                                    oninput: move |e| password.set(e.value())
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
                                            d: "M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z",
                                            clip_rule: "evenodd"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Remember me and forgot password
                    div {
                        class: "flex items-center justify-between",
                        div {
                            class: "flex items-center",
                            input {
                                id: "remember-me",
                                name: "remember-me",
                                r#type: "checkbox",
                                class: "h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded",
                                checked: remember_me(),
                                onchange: move |e| remember_me.set(e.checked())
                            }
                            label {
                                r#for: "remember-me",
                                class: "ml-2 block text-sm text-gray-900",
                                "Remember me"
                            }
                        }
                        div {
                            class: "text-sm",
                            a {
                                href: "#",
                                class: "font-medium text-blue-600 hover:text-blue-500",
                                "Forgot your password?"
                            }
                        }
                    }

                    // Submit button
                    div {
                        button {
                            r#type: "submit",
                            class: "group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: app_state.is_loading,

                            // Loading indicator
                            if app_state.is_loading {
                                span {
                                    class: "absolute left-0 inset-y-0 flex items-center pl-3",
                                    svg {
                                        class: "animate-spin h-5 w-5 text-blue-300",
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
                                }
                            } else {
                                span {
                                    class: "absolute left-0 inset-y-0 flex items-center pl-3",
                                    svg {
                                        class: "h-5 w-5 text-blue-500 group-hover:text-blue-400",
                                        xmlns: "http://www.w3.org/2000/svg",
                                        view_box: "0 0 20 20",
                                        fill: "currentColor",
                                        path {
                                            fill_rule: "evenodd",
                                            d: "M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z",
                                            clip_rule: "evenodd"
                                        }
                                    }
                                }
                            }

                            if app_state.is_loading {
                                "Signing in..."
                            } else {
                                "Sign in"
                            }
                        }
                    }
                }

                // Alternative login methods
                div {
                    class: "mt-6",
                    div {
                        class: "relative",
                        div {
                            class: "absolute inset-0 flex items-center",
                            div {
                                class: "w-full border-t border-gray-300"
                            }
                        }
                        div {
                            class: "relative flex justify-center text-sm",
                            span {
                                class: "px-2 bg-gray-50 text-gray-500",
                                "Or continue with"
                            }
                        }
                    }

                    div {
                        class: "mt-6 grid grid-cols-2 gap-3",

                        // OAuth providers (mock)
                        button {
                            r#type: "button",
                            class: "w-full inline-flex justify-center py-2 px-4 border border-gray-300 rounded-md shadow-sm bg-white text-sm font-medium text-gray-500 hover:bg-gray-50",
                            onclick: move |_| {
                                // Mock OAuth login
                                login_error.set(Some("OAuth login not implemented in demo".to_string()));
                            },
                            span {
                                class: "sr-only",
                                "Sign in with Google"
                            }
                            span {
                                class: "text-lg",
                                "🔍"
                            }
                            span {
                                class: "ml-2",
                                "Google"
                            }
                        }

                        button {
                            r#type: "button",
                            class: "w-full inline-flex justify-center py-2 px-4 border border-gray-300 rounded-md shadow-sm bg-white text-sm font-medium text-gray-500 hover:bg-gray-50",
                            onclick: move |_| {
                                // Mock GitHub login
                                login_error.set(Some("GitHub login not implemented in demo".to_string()));
                            },
                            span {
                                class: "sr-only",
                                "Sign in with GitHub"
                            }
                            span {
                                class: "text-lg",
                                "🐙"
                            }
                            span {
                                class: "ml-2",
                                "GitHub"
                            }
                        }
                    }
                }

                // Sign up link
                div {
                    class: "text-center mt-6",
                    p {
                        class: "text-sm text-gray-600",
                        "Don't have an account? "
                        a {
                            href: "#",
                            class: "font-medium text-blue-600 hover:text-blue-500",
                            "Sign up here"
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
    fn test_login_component_creation() {
        let _login = rsx! { Login {} };
    }
}
