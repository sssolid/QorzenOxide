// src/ui/mod.rs - UI system coordinator

use std::collections::HashMap;
use std::sync::Arc;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::{Permission, User, UserSession};
use crate::error::Result;
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};
use crate::plugin::MenuItem;

// Re-export main app component
pub use app::App;

// Module declarations
pub mod app;
pub mod components;
pub mod layout;
pub mod pages;
pub mod router;
pub mod state;

// Re-exports for convenience
pub use components::*;
pub use layout::*;
pub use pages::{Admin, Dashboard, Login, NotFound, Plugins, Profile, Settings};
pub use router::Route;
pub use state::*;

/// UI layout configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UILayout {
    pub layout_id: String,
    pub name: String,
    pub for_roles: Vec<String>,
    pub for_platforms: Vec<Platform>,
    pub header: HeaderConfig,
    pub sidebar: SidebarConfig,
    pub main_content: MainContentConfig,
    pub footer: FooterConfig,
    pub breakpoints: BreakpointConfig,
}

/// Platform types for UI adaptation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Platform {
    Desktop,
    Mobile,
    Tablet,
    Web,
}

/// Header configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeaderConfig {
    pub show_logo: bool,
    pub show_user_menu: bool,
    pub show_notifications: bool,
    pub menu_items: Vec<MenuItem>,
    pub quick_actions: Vec<QuickAction>,
}

/// Quick action button
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuickAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub action: String,
    pub required_permissions: Vec<Permission>,
}

/// Sidebar configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SidebarConfig {
    pub show: bool,
    pub collapsible: bool,
    pub default_collapsed: bool,
    pub navigation_items: Vec<NavigationItem>,
    pub plugin_panels: Vec<PluginPanel>,
}

/// Navigation item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NavigationItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub route: String,
    pub required_permissions: Vec<Permission>,
    pub badge: Option<Badge>,
    pub children: Vec<NavigationItem>,
}

/// Badge for navigation items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Badge {
    pub text: String,
    pub color: String,
    pub background: String,
}

/// Plugin panel in sidebar
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginPanel {
    pub plugin_id: String,
    pub component_id: String,
    pub title: String,
    pub collapsible: bool,
    pub default_collapsed: bool,
}

/// Main content area configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MainContentConfig {
    pub padding: String,
    pub max_width: Option<String>,
    pub background: String,
}

/// Footer configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FooterConfig {
    pub show: bool,
    pub content: String,
    pub links: Vec<FooterLink>,
}

/// Footer link
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FooterLink {
    pub label: String,
    pub url: String,
    pub external: bool,
}

/// Responsive breakpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BreakpointConfig {
    pub mobile: u32,  // 0-767px
    pub tablet: u32,  // 768-1023px
    pub desktop: u32, // 1024px+
    pub large: u32,   // 1440px+
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub colors: ColorPalette,
    pub typography: Typography,
    pub spacing: Spacing,
    pub shadows: Shadows,
    pub animations: Animations,
}

/// Color palette
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ColorPalette {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub background: String,
    pub surface: String,
    pub error: String,
    pub warning: String,
    pub success: String,
    pub info: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub border: String,
}

/// Typography configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Typography {
    pub font_family: String,
    pub font_size_base: String,
    pub font_weight_normal: u16,
    pub font_weight_bold: u16,
    pub line_height: f32,
    pub heading_scale: f32,
}

/// Spacing configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Spacing {
    pub unit: String,
    pub xs: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
}

/// Shadow configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Shadows {
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Animations {
    pub duration_fast: String,
    pub duration_normal: String,
    pub duration_slow: String,
    pub easing: String,
}

/// Application state for UI
#[derive(Debug, Clone)]
pub struct AppState {
    pub current_user: Option<User>,
    pub current_session: Option<UserSession>,
    pub current_layout: UILayout,
    pub current_theme: Theme,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub notifications: Vec<Notification>,
}

/// Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub read: bool,
    pub actions: Vec<NotificationAction>,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
    System,
}

/// Notification action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub label: String,
    pub action: String,
    pub style: ActionStyle,
}

/// Action button styles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStyle {
    Primary,
    Secondary,
    Danger,
    Link,
}

/// UI Layout Manager
pub struct UILayoutManager {
    state: ManagedState,
    layouts: Arc<RwLock<HashMap<String, UILayout>>>,
    themes: Arc<RwLock<HashMap<String, Theme>>>,
    current_layout: Arc<RwLock<Option<UILayout>>>,
    current_theme: Arc<RwLock<Option<Theme>>>,
}

impl std::fmt::Debug for UILayoutManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UILayoutManager").finish()
    }
}

impl Default for UILayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UILayoutManager {
    /// Creates a new UI layout manager
    pub fn new() -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "ui_layout_manager"),
            layouts: Arc::new(RwLock::new(HashMap::new())),
            themes: Arc::new(RwLock::new(HashMap::new())),
            current_layout: Arc::new(RwLock::new(None)),
            current_theme: Arc::new(RwLock::new(None)),
        }
    }

    /// Registers a layout
    pub async fn register_layout(&self, layout: UILayout) {
        self.layouts
            .write()
            .await
            .insert(layout.layout_id.clone(), layout);
    }

    /// Registers a theme
    pub async fn register_theme(&self, theme: Theme) {
        self.themes.write().await.insert(theme.id.clone(), theme);
    }

    /// Gets a layout by ID
    pub async fn get_layout(&self, layout_id: &str) -> Option<UILayout> {
        self.layouts.read().await.get(layout_id).cloned()
    }

    /// Gets a theme by ID
    pub async fn get_theme(&self, theme_id: &str) -> Option<Theme> {
        self.themes.read().await.get(theme_id).cloned()
    }

    /// Sets the current layout
    pub async fn set_current_layout(&self, layout: UILayout) {
        *self.current_layout.write().await = Some(layout);
    }

    /// Sets the current theme
    pub async fn set_current_theme(&self, theme: Theme) {
        *self.current_theme.write().await = Some(theme);
    }

    /// Gets the current layout
    pub async fn current_layout(&self) -> Option<UILayout> {
        self.current_layout.read().await.clone()
    }

    /// Gets the current theme
    pub async fn current_theme(&self) -> Option<Theme> {
        self.current_theme.read().await.clone()
    }

    /// Finds appropriate layout for user and platform
    pub async fn find_layout_for_user(&self, user: &User, platform: Platform) -> Option<UILayout> {
        let layouts = self.layouts.read().await;

        for layout in layouts.values() {
            // Check if layout supports this platform
            if !layout.for_platforms.is_empty() && !layout.for_platforms.contains(&platform) {
                continue;
            }

            // Check if user has required roles
            if layout.for_roles.is_empty() {
                return Some(layout.clone());
            }

            for user_role in &user.roles {
                if layout.for_roles.contains(&user_role.id) {
                    return Some(layout.clone());
                }
            }
        }

        None
    }

    /// Gets default layout
    pub async fn default_layout(&self) -> UILayout {
        UILayout {
            layout_id: "default".to_string(),
            name: "Default Layout".to_string(),
            for_roles: Vec::new(),
            for_platforms: vec![Platform::Desktop, Platform::Web],
            header: HeaderConfig {
                show_logo: true,
                show_user_menu: true,
                show_notifications: true,
                menu_items: Vec::new(),
                quick_actions: Vec::new(),
            },
            sidebar: SidebarConfig {
                show: true,
                collapsible: true,
                default_collapsed: false,
                navigation_items: Vec::new(),
                plugin_panels: Vec::new(),
            },
            main_content: MainContentConfig {
                padding: "1rem".to_string(),
                max_width: None,
                background: "#ffffff".to_string(),
            },
            footer: FooterConfig {
                show: true,
                content: "© 2024 Qorzen".to_string(),
                links: Vec::new(),
            },
            breakpoints: BreakpointConfig {
                mobile: 768,
                tablet: 1024,
                desktop: 1440,
                large: 1920,
            },
        }
    }

    /// Gets default theme
    pub async fn default_theme(&self) -> Theme {
        Theme {
            id: "default".to_string(),
            name: "Default Theme".to_string(),
            colors: ColorPalette {
                primary: "#3b82f6".to_string(),
                secondary: "#64748b".to_string(),
                accent: "#8b5cf6".to_string(),
                background: "#ffffff".to_string(),
                surface: "#f8fafc".to_string(),
                error: "#ef4444".to_string(),
                warning: "#f59e0b".to_string(),
                success: "#10b981".to_string(),
                info: "#06b6d4".to_string(),
                text_primary: "#1e293b".to_string(),
                text_secondary: "#64748b".to_string(),
                border: "#e2e8f0".to_string(),
            },
            typography: Typography {
                font_family: "-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif"
                    .to_string(),
                font_size_base: "16px".to_string(),
                font_weight_normal: 400,
                font_weight_bold: 600,
                line_height: 1.5,
                heading_scale: 1.25,
            },
            spacing: Spacing {
                unit: "rem".to_string(),
                xs: "0.25rem".to_string(),
                sm: "0.5rem".to_string(),
                md: "1rem".to_string(),
                lg: "1.5rem".to_string(),
                xl: "2rem".to_string(),
            },
            shadows: Shadows {
                sm: "0 1px 2px 0 rgba(0, 0, 0, 0.05)".to_string(),
                md: "0 4px 6px -1px rgba(0, 0, 0, 0.1)".to_string(),
                lg: "0 10px 15px -3px rgba(0, 0, 0, 0.1)".to_string(),
                xl: "0 25px 50px -12px rgba(0, 0, 0, 0.25)".to_string(),
            },
            animations: Animations {
                duration_fast: "150ms".to_string(),
                duration_normal: "300ms".to_string(),
                duration_slow: "500ms".to_string(),
                easing: "cubic-bezier(0.4, 0, 0.2, 1)".to_string(),
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
impl Manager for UILayoutManager {
    fn name(&self) -> &str {
        "ui_layout_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Register default layout and theme
        let default_layout = self.default_layout().await;
        let default_theme = self.default_theme().await;

        self.register_layout(default_layout.clone()).await;
        self.register_theme(default_theme.clone()).await;

        self.set_current_layout(default_layout).await;
        self.set_current_theme(default_theme).await;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;
        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        status.add_metadata(
            "layouts_count",
            serde_json::Value::from(self.layouts.read().await.len()),
        );
        status.add_metadata(
            "themes_count",
            serde_json::Value::from(self.themes.read().await.len()),
        );

        if let Some(layout) = self.current_layout().await {
            status.add_metadata(
                "current_layout",
                serde_json::Value::String(layout.layout_id),
            );
        }

        if let Some(theme) = self.current_theme().await {
            status.add_metadata("current_theme", serde_json::Value::String(theme.id));
        }

        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: false,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec!["ui.access".to_string()],
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait(?Send)]
impl Manager for UILayoutManager {
    fn name(&self) -> &str {
        "ui_layout_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Register default layout and theme
        let default_layout = self.default_layout().await;
        let default_theme = self.default_theme().await;

        self.register_layout(default_layout.clone()).await;
        self.register_theme(default_theme.clone()).await;

        self.set_current_layout(default_layout).await;
        self.set_current_theme(default_theme).await;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;
        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        status.add_metadata(
            "layouts_count",
            serde_json::Value::from(self.layouts.read().await.len()),
        );
        status.add_metadata(
            "themes_count",
            serde_json::Value::from(self.themes.read().await.len()),
        );

        if let Some(layout) = self.current_layout().await {
            status.add_metadata(
                "current_layout",
                serde_json::Value::String(layout.layout_id),
            );
        }

        if let Some(theme) = self.current_theme().await {
            status.add_metadata("current_theme", serde_json::Value::String(theme.id));
        }

        status
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: false,
            requires_network: false,
            requires_database: false,
            requires_native_apis: false,
            minimum_permissions: vec!["ui.access".to_string()],
        }
    }
}

/// Main app entry point - simple wrapper for the App component
pub fn app() -> Element {
    rsx! { App {} }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_layout_creation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = UILayoutManager::new();
            let layout = manager.default_layout().await;

            assert_eq!(layout.layout_id, "default");
            assert!(layout.header.show_logo);
            assert!(layout.sidebar.show);
        });
    }

    #[test]
    fn test_platform_equality() {
        assert_eq!(Platform::Desktop, Platform::Desktop);
        assert_ne!(Platform::Desktop, Platform::Mobile);
    }
}
