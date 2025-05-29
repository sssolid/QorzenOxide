// src/ui/desktop_css.rs - CSS injection for desktop builds

/// Embed Tailwind CSS for desktop applications
pub const TAILWIND_CSS: &str = include_str!("../../public/tailwind.css");

/// Inject CSS into the desktop webview
#[cfg(not(target_arch = "wasm32"))]
pub fn inject_css_into_webview() -> String {
    format!(
        r#"
        <style>
            {}
        </style>
        "#,
        TAILWIND_CSS
    )
}

/// Get the complete CSS string for embedding
pub fn get_embedded_css() -> &'static str {
    TAILWIND_CSS
}