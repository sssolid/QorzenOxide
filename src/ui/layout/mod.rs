// src/ui/layout/mod.rs - Layout system components

use dioxus::prelude::*;

// Module declarations
mod footer;
mod header;
mod main_layout;
mod sidebar;

// Re-exports
pub use footer::Footer;
pub use header::Header;
pub use main_layout::Layout;
pub use sidebar::Sidebar;

/// Layout configuration props
#[derive(Props, Clone, PartialEq)]
pub struct LayoutProps {
    /// Children to render in the main content area
    pub children: Element,
    /// Optional custom class for the layout
    #[props(default = "".to_string())]
    pub class: String,
    /// Whether to show the header
    #[props(default = true)]
    pub show_header: bool,
    /// Whether to show the sidebar
    #[props(default = true)]
    pub show_sidebar: bool,
    /// Whether to show the footer
    #[props(default = true)]
    pub show_footer: bool,
}

/// Responsive layout breakpoints (matching Tailwind defaults)
pub struct Breakpoints;

impl Breakpoints {
    pub const SM: &'static str = "640px"; // Tailwind sm
    pub const MD: &'static str = "768px"; // Tailwind md
    pub const LG: &'static str = "1024px"; // Tailwind lg
    pub const XL: &'static str = "1280px"; // Tailwind xl
    pub const XXL: &'static str = "1536px"; // Tailwind 2xl
}

/// Common layout utilities
pub mod utils {
    /// Check if the current viewport is mobile-sized
    pub fn is_mobile() -> bool {
        // In a real app, this would check the actual viewport
        // For now, return false (desktop)
        false
    }

    /// Check if the current viewport is tablet-sized
    pub fn is_tablet() -> bool {
        // In a real app, this would check the actual viewport
        false
    }

    /// Check if the current viewport is desktop-sized
    pub fn is_desktop() -> bool {
        // In a real app, this would check the actual viewport
        true
    }

    /// Get responsive classes based on viewport
    pub fn responsive_classes(_mobile: &str, _tablet: &str, desktop: &str) -> String {
        // In a real app, this would return the appropriate class based on viewport
        // For now, return desktop classes
        desktop.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoints() {
        assert_eq!(Breakpoints::SM, "640px");
        assert_eq!(Breakpoints::MD, "768px");
        assert_eq!(Breakpoints::LG, "1024px");
    }

    #[test]
    fn test_responsive_utils() {
        assert!(utils::is_desktop());
        assert!(!utils::is_mobile());
        assert!(!utils::is_tablet());
    }
}
