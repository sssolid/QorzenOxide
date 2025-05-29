// src/ui/components/mod.rs - Reusable UI components

use dioxus::prelude::*;

/// Button component with consistent styling
#[component]
pub fn Button(
    #[props(default = "button".to_string())] button_type: String,
    #[props(default = "primary".to_string())] variant: String,
    #[props(default = "md".to_string())] size: String,
    #[props(default = false)] disabled: bool,
    #[props(default = false)] loading: bool,
    #[props(default = "".to_string())] class: String,
    #[props(default = None)] onclick: Option<Callback<MouseEvent>>,
    children: Element,
) -> Element {
    let base_classes = "inline-flex items-center border font-medium rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 transition-colors";

    let variant_classes = match variant.as_str() {
        "primary" => "border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500",
        "secondary" => "border-gray-300 text-gray-700 bg-white hover:bg-gray-50 focus:ring-blue-500",
        "danger" => "border-transparent text-white bg-red-600 hover:bg-red-700 focus:ring-red-500",
        "success" => "border-transparent text-white bg-green-600 hover:bg-green-700 focus:ring-green-500",
        "warning" => "border-transparent text-white bg-yellow-600 hover:bg-yellow-700 focus:ring-yellow-500",
        "ghost" => "border-transparent text-gray-700 hover:bg-gray-100 focus:ring-blue-500",
        _ => "border-gray-300 text-gray-700 bg-white hover:bg-gray-50 focus:ring-blue-500",
    };

    let size_classes = match size.as_str() {
        "xs" => "px-2.5 py-1.5 text-xs",
        "sm" => "px-3 py-2 text-sm leading-4",
        "md" => "px-4 py-2 text-sm",
        "lg" => "px-4 py-2 text-base",
        "xl" => "px-6 py-3 text-base",
        _ => "px-4 py-2 text-sm",
    };

    let disabled_classes = if disabled || loading {
        "opacity-50 cursor-not-allowed"
    } else {
        ""
    };

    rsx! {
        button {
            r#type: "{button_type}",
            class: format!("{} {} {} {} {}", base_classes, variant_classes, size_classes, disabled_classes, class),
            disabled: disabled || loading,
            onclick: move |evt| {
                if let Some(handler) = &onclick {
                    handler.call(evt);
                }
            },

            if loading {
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
            }

            {children}
        }
    }
}

/// Input component with consistent styling
#[component]
pub fn Input(
    #[props(default = "text".to_string())] input_type: String,
    #[props(default = "".to_string())] name: String,
    #[props(default = "".to_string())] id: String,
    #[props(default = "".to_string())] placeholder: String,
    #[props(default = "".to_string())] value: String,
    #[props(default = false)] required: bool,
    #[props(default = false)] disabled: bool,
    #[props(default = "".to_string())] class: String,
    #[props(default = None)] oninput: Option<Callback<FormEvent>>,
    #[props(default = None)] onchange: Option<Callback<FormEvent>>,
) -> Element {
    let base_classes = "block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 sm:text-sm";
    let disabled_classes = if disabled { "bg-gray-50 text-gray-500" } else { "" };

    rsx! {
        input {
            r#type: "{input_type}",
            name: "{name}",
            id: "{id}",
            placeholder: "{placeholder}",
            value: "{value}",
            required: required,
            disabled: disabled,
            class: format!("{} {} {}", base_classes, disabled_classes, class),
            oninput: move |evt| {
                if let Some(handler) = &oninput {
                    handler.call(evt);
                }
            },
            onchange: move |evt| {
                if let Some(handler) = &onchange {
                    handler.call(evt);
                }
            }
        }
    }
}

/// Label component
#[component]
pub fn Label(
    #[props(default = "".to_string())] html_for: String,
    #[props(default = false)] required: bool,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        label {
            r#for: "{html_for}",
            class: format!("block text-sm font-medium text-gray-700 {}", class),
            {children}
            if required {
                span {
                    class: "text-red-500 ml-1",
                    "*"
                }
            }
        }
    }
}

/// Form field wrapper component
#[component]
pub fn FormField(
    #[props(default = "".to_string())] label: String,
    #[props(default = "".to_string())] id: String,
    #[props(default = false)] required: bool,
    #[props(default = None)] error: Option<String>,
    #[props(default = None)] help_text: Option<String>,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!("space-y-1 {}", class),

            if !label.is_empty() {
                Label {
                    html_for: id.clone(),
                    required: required,
                    "{label}"
                }
            }

            {children}

            if let Some(error_msg) = error {
                p {
                    class: "text-sm text-red-600",
                    "{error_msg}"
                }
            }

            if let Some(help) = help_text {
                p {
                    class: "text-sm text-gray-500",
                    "{help}"
                }
            }
        }
    }
}

/// Modal component
#[component]
pub fn Modal(
    #[props(default = false)] show: bool,
    #[props(default = "".to_string())] title: String,
    #[props(default = None)] on_close: Option<Callback<()>>,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    if !show {
        return rsx! { div { style: "display: none;" } };
    }

    rsx! {
        div {
            class: "fixed inset-0 z-50 overflow-y-auto",

            // Backdrop
            div {
                class: "fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity",
                onclick: move |_| {
                    if let Some(handler) = &on_close {
                        handler.call(());
                    }
                }
            }

            // Modal content
            div {
                class: "flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0",
                div {
                    class: format!(
                        "relative transform overflow-hidden rounded-lg bg-white text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg {}",
                        class
                    ),
                    onclick: |evt| evt.stop_propagation(),

                    if !title.is_empty() {
                        div {
                            class: "bg-white px-4 pb-4 pt-5 sm:p-6 sm:pb-4",
                            div {
                                class: "flex items-start justify-between",
                                h3 {
                                    class: "text-lg font-medium leading-6 text-gray-900",
                                    "{title}"
                                }
                                if on_close.is_some() {
                                    button {
                                        r#type: "button",
                                        class: "rounded-md bg-white text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500",
                                        onclick: move |_| {
                                            if let Some(handler) = &on_close {
                                                handler.call(());
                                            }
                                        },
                                        span {
                                            class: "sr-only",
                                            "Close"
                                        }
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
                                                d: "M6 18L18 6M6 6l12 12"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "px-4 pb-4 pt-5 sm:p-6",
                        {children}
                    }
                }
            }
        }
    }
}

/// Alert/Banner component
#[component]
pub fn Alert(
    #[props(default = "info".to_string())] variant: String,
    #[props(default = "".to_string())] title: String,
    #[props(default = false)] dismissible: bool,
    #[props(default = None)] on_dismiss: Option<Callback<()>>,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    let (bg_color, border_color, text_color, title_color, icon) = match variant.as_str() {
        "success" => ("bg-green-50", "border-green-200", "text-green-700", "text-green-800", "✅"),
        "warning" => ("bg-yellow-50", "border-yellow-200", "text-yellow-700", "text-yellow-800", "⚠️"),
        "error" => ("bg-red-50", "border-red-200", "text-red-700", "text-red-800", "❌"),
        "info" => ("bg-blue-50", "border-blue-200", "text-blue-700", "text-blue-800", "ℹ️"),
        _ => ("bg-gray-50", "border-gray-200", "text-gray-700", "text-gray-800", "📌"),
    };

    rsx! {
        div {
            class: format!("rounded-md {} border {} p-4 {}", bg_color, border_color, class),
            div {
                class: "flex",
                div {
                    class: "flex-shrink-0",
                    span {
                        class: "text-lg",
                        "{icon}"
                    }
                }
                div {
                    class: "ml-3 flex-1",
                    if !title.is_empty() {
                        h3 {
                            class: format!("text-sm font-medium {}", title_color),
                            "{title}"
                        }
                    }
                    div {
                        class: format!("text-sm {}", text_color),
                        {children}
                    }
                }
                if dismissible {
                    div {
                        class: "ml-auto pl-3",
                        button {
                            r#type: "button",
                            class: format!("inline-flex rounded-md {} hover:bg-opacity-20 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-green-50 focus:ring-green-600 p-1.5", text_color),
                            onclick: move |_| {
                                if let Some(handler) = &on_dismiss {
                                    handler.call(());
                                }
                            },
                            span {
                                class: "sr-only",
                                "Dismiss"
                            }
                            svg {
                                class: "h-5 w-5",
                                xmlns: "http://www.w3.org/2000/svg",
                                view_box: "0 0 20 20",
                                fill: "currentColor",
                                path {
                                    fill_rule: "evenodd",
                                    d: "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z",
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

/// Badge component
#[component]
pub fn Badge(
    #[props(default = "gray".to_string())] variant: String,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    let variant_classes = match variant.as_str() {
        "red" => "bg-red-100 text-red-800",
        "yellow" => "bg-yellow-100 text-yellow-800",
        "green" => "bg-green-100 text-green-800",
        "blue" => "bg-blue-100 text-blue-800",
        "indigo" => "bg-indigo-100 text-indigo-800",
        "purple" => "bg-purple-100 text-purple-800",
        "pink" => "bg-pink-100 text-pink-800",
        _ => "bg-gray-100 text-gray-800",
    };

    rsx! {
        span {
            class: format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {} {}", variant_classes, class),
            {children}
        }
    }
}

/// Loading spinner component
#[component]
pub fn Spinner(
    #[props(default = "md".to_string())] size: String,
    #[props(default = "".to_string())] class: String,
) -> Element {
    let size_classes = match size.as_str() {
        "xs" => "h-3 w-3",
        "sm" => "h-4 w-4",
        "md" => "h-6 w-6",
        "lg" => "h-8 w-8",
        "xl" => "h-12 w-12",
        _ => "h-6 w-6",
    };

    rsx! {
        svg {
            class: format!("animate-spin {} {}", size_classes, class),
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
}

/// Card component
#[component]
pub fn Card(
    #[props(default = "".to_string())] title: String,
    #[props(default = None)] subtitle: Option<String>,
    #[props(default = None)] actions: Option<Element>,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!("bg-white overflow-hidden shadow rounded-lg {}", class),

            if !title.is_empty() || actions.is_some() {
                div {
                    class: "px-4 py-5 sm:px-6 border-b border-gray-200",
                    div {
                        class: "flex items-center justify-between",
                        div {
                            h3 {
                                class: "text-lg leading-6 font-medium text-gray-900",
                                "{title}"
                            }
                            if let Some(sub) = subtitle {
                                p {
                                    class: "mt-1 max-w-2xl text-sm text-gray-500",
                                    "{sub}"
                                }
                            }
                        }
                        if let Some(actions_el) = actions {
                            div {
                                class: "flex space-x-3",
                                {actions_el}
                            }
                        }
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

/// Dropdown menu component
#[component]
pub fn Dropdown(
    #[props(default = false)] open: bool,
    #[props(default = None)] on_toggle: Option<Callback<()>>,
    #[props(default = "".to_string())] button_class: String,
    #[props(default = "".to_string())] menu_class: String,
    trigger: Element,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "relative inline-block text-left",

            // Trigger button
            button {
                r#type: "button",
                class: format!("inline-flex w-full justify-center gap-x-1.5 rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 {}", button_class),
                onclick: move |_| {
                    if let Some(handler) = &on_toggle {
                        handler.call(());
                    }
                },
                {trigger}
                svg {
                    class: "-mr-1 h-5 w-5 text-gray-400",
                    xmlns: "http://www.w3.org/2000/svg",
                    view_box: "0 0 20 20",
                    fill: "currentColor",
                    path {
                        fill_rule: "evenodd",
                        d: "M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z",
                        clip_rule: "evenodd"
                    }
                }
            }

            // Dropdown menu
            if open {
                div {
                    class: format!("absolute right-0 z-10 mt-2 w-56 origin-top-right divide-y divide-gray-100 rounded-md bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none {}", menu_class),
                    {children}
                }
            }
        }
    }
}

/// Tabs component
#[component]
pub fn Tabs(
    #[props(default = "".to_string())] active_tab: String,
    #[props(default = None)] on_tab_change: Option<Callback<String>>,
    tabs: Vec<TabItem>,
    #[props(default = "".to_string())] class: String,
) -> Element {
    rsx! {
        div {
            class: format!("border-b border-gray-200 {}", class),
            nav {
                class: "-mb-px flex space-x-8",
                for tab in tabs {
                    button {
                        key: "{tab.id}",
                        r#type: "button",
                        class: format!(
                            "py-2 px-1 border-b-2 font-medium text-sm {}",
                            if active_tab == tab.id {
                                "border-blue-500 text-blue-600"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                            }
                        ),
                        onclick: move |_| {
                            if let Some(handler) = &on_tab_change {
                                handler.call(tab.id.clone());
                            }
                        },
                        "{tab.label}"
                        if let Some(count) = tab.count {
                            span {
                                class: format!(
                                    "ml-2 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}",
                                    if active_tab == tab.id {
                                        "bg-blue-100 text-blue-600"
                                    } else {
                                        "bg-gray-100 text-gray-900"
                                    }
                                ),
                                "{count}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Tab item data structure
#[derive(Debug, Clone, PartialEq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub count: Option<u32>,
}

/// Toggle/Switch component
#[component]
pub fn Toggle(
    #[props(default = false)] checked: bool,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] on_change: Option<Callback<bool>>,
    #[props(default = "".to_string())] class: String,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: format!(
                "relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {} {}",
                if checked { "bg-blue-600" } else { "bg-gray-200" },
                if disabled { "opacity-50 cursor-not-allowed" } else { "" }
            ),
            disabled: disabled,
            onclick: move |_| {
                if !disabled {
                    if let Some(handler) = &on_change {
                        handler.call(!checked);
                    }
                }
            },
            span {
                class: format!(
                    "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {}",
                    if checked { "translate-x-5" } else { "translate-x-0" }
                )
            }
        }
    }
}

/// Tooltip component (simple implementation)
#[component]
pub fn Tooltip(
    #[props(default = "".to_string())] text: String,
    #[props(default = "top".to_string())] position: String,
    #[props(default = "".to_string())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: format!("relative inline-block {}", class),
            title: "{text}",
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_component() {
        let _button = rsx! {
            Button {
                variant: "primary".to_string(),
                "Click me"
            }
        };
    }

    #[test]
    fn test_input_component() {
        let _input = rsx! {
            Input {
                input_type: "email".to_string(),
                placeholder: "Enter email".to_string()
            }
        };
    }

    #[test]
    fn test_alert_component() {
        let _alert = rsx! {
            Alert {
                variant: "success".to_string(),
                title: "Success".to_string(),
                "Operation completed"
            }
        };
    }

    #[test]
    fn test_tab_item() {
        let tab = TabItem {
            id: "test".to_string(),
            label: "Test Tab".to_string(),
            count: Some(5),
        };
        assert_eq!(tab.id, "test");
        assert_eq!(tab.count, Some(5));
    }
}