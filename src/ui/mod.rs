// src/ui/mod.rs - Dioxus-based UI system with role-based layouts

use std::collections::HashMap;
use std::sync::Arc;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::{Permission, User, UserSession};
use crate::error::{Error, Result};
use crate::manager::{Manager, ManagedState, ManagerStatus, PlatformRequirements};
use crate::plugin::{MenuItem, PluginManager, UIComponent};

/// UI layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Desktop,
    Mobile,
    Tablet,
    Web,
}

/// Header configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    pub show_logo: bool,
    pub show_user_menu: bool,
    pub show_notifications: bool,
    pub menu_items: Vec<MenuItem>,
    pub quick_actions: Vec<QuickAction>,
}

/// Quick action button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub action: String,
    pub required_permissions: Vec<Permission>,
}

/// Sidebar configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarConfig {
    pub show: bool,
    pub collapsible: bool,
    pub default_collapsed: bool,
    pub navigation_items: Vec<NavigationItem>,
    pub plugin_panels: Vec<PluginPanel>,
}

/// Navigation item
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    pub text: String,
    pub color: String,
    pub background: String,
}

/// Plugin panel in sidebar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPanel {
    pub plugin_id: String,
    pub component_id: String,
    pub title: String,
    pub collapsible: bool,
    pub default_collapsed: bool,
}

/// Main content area configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainContentConfig {
    pub padding: String,
    pub max_width: Option<String>,
    pub background: String,
}

/// Footer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterConfig {
    pub show: bool,
    pub content: String,
    pub links: Vec<FooterLink>,
}

/// Footer link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterLink {
    pub label: String,
    pub url: String,
    pub external: bool,
}

/// Responsive breakpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointConfig {
    pub mobile: u32,    // 0-767px
    pub tablet: u32,    // 768-1023px
    pub desktop: u32,   // 1024px+
    pub large: u32,     // 1440px+
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typography {
    pub font_family: String,
    pub font_size_base: String,
    pub font_weight_normal: u16,
    pub font_weight_bold: u16,
    pub line_height: f32,
    pub heading_scale: f32,
}

/// Spacing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub unit: String,
    pub xs: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
}

/// Shadow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shadows {
    pub sm: String,
    pub md: String,
    pub lg: String,
    pub xl: String,
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    plugin_manager: Option<Arc<PluginManager>>,
}

impl std::fmt::Debug for UILayoutManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UILayoutManager").finish()
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
            plugin_manager: None,
        }
    }

    /// Sets the plugin manager for UI integration
    pub fn set_plugin_manager(&mut self, plugin_manager: Arc<PluginManager>) {
        self.plugin_manager = Some(plugin_manager);
    }

    /// Registers a layout
    pub async fn register_layout(&self, layout: UILayout) {
        self.layouts.write().await.insert(layout.layout_id.clone(), layout);
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
                font_family: "-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif".to_string(),
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

#[async_trait::async_trait]
impl Manager for UILayoutManager {
    fn name(&self) -> &str {
        "ui_layout_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::Initializing).await;

        // Register default layout and theme
        let default_layout = self.default_layout().await;
        let default_theme = self.default_theme().await;

        self.register_layout(default_layout.clone()).await;
        self.register_theme(default_theme.clone()).await;

        self.set_current_layout(default_layout).await;
        self.set_current_theme(default_theme).await;

        self.state.set_state(crate::manager::ManagerState::Running).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(crate::manager::ManagerState::ShuttingDown).await;
        self.state.set_state(crate::manager::ManagerState::Shutdown).await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        status.add_metadata("layouts_count", serde_json::Value::from(self.layouts.read().await.len()));
        status.add_metadata("themes_count", serde_json::Value::from(self.themes.read().await.len()));

        if let Some(layout) = self.current_layout().await {
            status.add_metadata("current_layout", serde_json::Value::String(layout.layout_id));
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

/// Main application component
#[component]
pub fn App() -> Element {
    // Use shared state for application state
    let app_state = use_context::<AppState>();

    rsx! {
        div { class: "qorzen-app",
            style { {get_theme_css(&app_state.current_theme)} }

            match &app_state.current_user {
                Some(user) => rsx! {
                    AuthenticatedApp { user: user.clone(), layout: app_state.current_layout.clone() }
                },
                None => rsx! {
                    LoginScreen {}
                }
            }
        }
    }
}

/// Authenticated application layout
#[component]
fn AuthenticatedApp(user: User, layout: UILayout) -> Element {
    let sidebar_collapsed = use_signal(|| layout.sidebar.default_collapsed);

    rsx! {
        div { class: "app-container",
            if layout.header.show_logo || layout.header.show_user_menu {
                AppHeader {
                    user: user.clone(),
                    config: layout.header.clone(),
                    on_toggle_sidebar: move |_| {
                        *sidebar_collapsed.write() = !*sidebar_collapsed.read();
                    }
                }
            }

            div { class: "app-body",
                if layout.sidebar.show {
                    AppSidebar {
                        user: user.clone(),
                        config: layout.sidebar.clone(),
                        collapsed: *sidebar_collapsed.read()
                    }
                }

                AppMainContent {
                    user: user.clone(),
                    config: layout.main_content.clone(),
                    sidebar_shown: layout.sidebar.show,
                    sidebar_collapsed: *sidebar_collapsed.read()
                }
            }

            if layout.footer.show {
                AppFooter { config: layout.footer.clone() }
            }
        }
    }
}

/// Application header component
#[component]
fn AppHeader(user: User, config: HeaderConfig, on_toggle_sidebar: EventHandler<()>) -> Element {
    rsx! {
        header { class: "app-header",
            div { class: "header-left",
                if config.show_logo {
                    button {
                        class: "sidebar-toggle",
                        onclick: move |_| on_toggle_sidebar.call(()),
                        "☰"
                    }
                    div { class: "logo",
                        "Qorzen"
                    }
                }
            }

            div { class: "header-center",
                nav { class: "header-nav",
                    for item in config.menu_items {
                        HeaderMenuItem { item: item }
                    }
                }
            }

            div { class: "header-right",
                if config.show_notifications {
                    NotificationButton {}
                }

                for action in config.quick_actions {
                    QuickActionButton { action: action }
                }

                if config.show_user_menu {
                    UserMenu { user: user.clone() }
                }
            }
        }
    }
}

/// Header menu item
#[component]
fn HeaderMenuItem(item: MenuItem) -> Element {
    rsx! {
        a {
            href: item.route.unwrap_or_default(),
            class: "header-menu-item",

            if let Some(icon) = item.icon {
                span { class: "menu-icon", "{icon}" }
            }

            span { "{item.label}" }
        }
    }
}

/// Notification button
#[component]
fn NotificationButton() -> Element {
    let notification_count = use_signal(|| 0);

    rsx! {
        button { class: "notification-button",
            span { class: "notification-icon", "🔔" }
            if *notification_count.read() > 0 {
                span { class: "notification-badge", "{notification_count}" }
            }
        }
    }
}

/// Quick action button
#[component]
fn QuickActionButton(action: QuickAction) -> Element {
    rsx! {
        button {
            class: "quick-action-button",
            title: "{action.label}",
            onclick: move |_| {
                // Handle action
            },

            span { class: "action-icon", "{action.icon}" }
        }
    }
}

/// User menu dropdown
#[component]
fn UserMenu(user: User) -> Element {
    let menu_open = use_signal(|| false);

    rsx! {
        div { class: "user-menu",
            button {
                class: "user-menu-button",
                onclick: move |_| {
                    *menu_open.write() = !*menu_open.read();
                },

                div { class: "user-avatar",
                    if let Some(avatar_url) = &user.profile.avatar_url {
                        img { src: "{avatar_url}", alt: "User Avatar" }
                    } else {
                        span { "{user.profile.display_name.chars().next().unwrap_or('U')}" }
                    }
                }

                span { class: "user-name", "{user.profile.display_name}" }
                span { class: "dropdown-arrow", "▼" }
            }

            if *menu_open.read() {
                div { class: "user-menu-dropdown",
                    a { href: "/profile", class: "menu-item", "Profile" }
                    a { href: "/settings", class: "menu-item", "Settings" }
                    hr { class: "menu-divider" }
                    button {
                        class: "menu-item logout",
                        onclick: move |_| {
                            // Handle logout
                        },
                        "Logout"
                    }
                }
            }
        }
    }
}

/// Application sidebar
#[component]
fn AppSidebar(user: User, config: SidebarConfig, collapsed: bool) -> Element {
    rsx! {
        aside {
            class: if collapsed { "app-sidebar collapsed" } else { "app-sidebar" },

            nav { class: "sidebar-nav",
                for item in config.navigation_items {
                    SidebarNavItem { item: item, collapsed: collapsed }
                }
            }

            div { class: "sidebar-plugins",
                for panel in config.plugin_panels {
                    PluginSidebarPanel { panel: panel, collapsed: collapsed }
                }
            }
        }
    }
}

/// Sidebar navigation item
#[component]
fn SidebarNavItem(item: NavigationItem, collapsed: bool) -> Element {
    let is_active = use_signal(|| false); // Would check current route

    rsx! {
        div { class: "nav-item",
            a {
                href: "{item.route}",
                class: if *is_active.read() { "nav-link active" } else { "nav-link" },

                if let Some(icon) = item.icon {
                    span { class: "nav-icon", "{icon}" }
                }

                if !collapsed {
                    span { class: "nav-label", "{item.label}" }

                    if let Some(badge) = item.badge {
                        span {
                            class: "nav-badge",
                            style: "color: {badge.color}; background: {badge.background};",
                            "{badge.text}"
                        }
                    }
                }
            }

            if !collapsed && !item.children.is_empty() {
                div { class: "nav-children",
                    for child in item.children {
                        SidebarNavItem { item: child, collapsed: false }
                    }
                }
            }
        }
    }
}

/// Plugin sidebar panel
#[component]
fn PluginSidebarPanel(panel: PluginPanel, collapsed: bool) -> Element {
    let panel_collapsed = use_signal(|| panel.default_collapsed);

    if collapsed {
        return rsx! { div {} }; // Hide plugin panels when sidebar is collapsed
    }

    rsx! {
        div { class: "plugin-panel",
            if panel.collapsible {
                button {
                    class: "panel-header",
                    onclick: move |_| {
                        *panel_collapsed.write() = !*panel_collapsed.read();
                    },

                    span { class: "panel-title", "{panel.title}" }
                    span {
                        class: "panel-toggle",
                        if *panel_collapsed.read() { "▶" } else { "▼" }
                    }
                }
            } else {
                div { class: "panel-header",
                    span { class: "panel-title", "{panel.title}" }
                }
            }

            if !*panel_collapsed.read() {
                div { class: "panel-content",
                    PluginComponent {
                        plugin_id: panel.plugin_id.clone(),
                        component_id: panel.component_id.clone()
                    }
                }
            }
        }
    }
}

/// Plugin component renderer
#[component]
fn PluginComponent(plugin_id: String, component_id: String) -> Element {
    let plugin_manager = use_context::<Arc<PluginManager>>();

    // In a real implementation, this would render the actual plugin component
    rsx! {
        div { class: "plugin-component",
            "Plugin: {plugin_id} / Component: {component_id}"
        }
    }
}

/// Main content area
#[component]
fn AppMainContent(user: User, config: MainContentConfig, sidebar_shown: bool, sidebar_collapsed: bool) -> Element {
    rsx! {
        main {
            class: "app-main-content",
            style: "padding: {config.padding}; background: {config.background};",

            // Route-based content would go here
            div { class: "content-area",
                "Main application content for user: {user.username}"
            }
        }
    }
}

/// Application footer
#[component]
fn AppFooter(config: FooterConfig) -> Element {
    rsx! {
        footer { class: "app-footer",
            div { class: "footer-content",
                p { "{config.content}" }

                if !config.links.is_empty() {
                    nav { class: "footer-nav",
                        for link in config.links {
                            a {
                                href: "{link.url}",
                                target: if link.external { "_blank" } else { "_self" },
                                rel: if link.external { "noopener noreferrer" } else { "" },
                                "{link.label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Login screen for unauthenticated users
#[component]
fn LoginScreen() -> Element {
    let username = use_signal(|| String::new());
    let password = use_signal(|| String::new());
    let is_loading = use_signal(|| false);
    let error_message = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "login-screen",
            div { class: "login-container",
                div { class: "login-header",
                    h1 { "Welcome to Qorzen" }
                    p { "Please sign in to continue" }
                }

                form {
                    class: "login-form",
                    onsubmit: move |e| {
                        e.prevent_default();
                        // Handle login
                    },

                    div { class: "form-group",
                        label { r#for: "username", "Username" }
                        input {
                            r#type: "text",
                            id: "username",
                            value: "{username}",
                            oninput: move |e| {
                                *username.write() = e.value();
                            },
                            required: true
                        }
                    }

                    div { class: "form-group",
                        label { r#for: "password", "Password" }
                        input {
                            r#type: "password",
                            id: "password",
                            value: "{password}",
                            oninput: move |e| {
                                *password.write() = e.value();
                            },
                            required: true
                        }
                    }

                    if let Some(error) = error_message.read().as_ref() {
                        div { class: "error-message", "{error}" }
                    }

                    button {
                        r#type: "submit",
                        class: "login-button",
                        disabled: *is_loading.read(),

                        if *is_loading.read() {
                            "Signing in..."
                        } else {
                            "Sign In"
                        }
                    }
                }
            }
        }
    }
}

/// Generates CSS from theme configuration
fn get_theme_css(theme: &Theme) -> String {
    format!(
        r#"
        :root {{
            --color-primary: {primary};
            --color-secondary: {secondary};
            --color-accent: {accent};
            --color-background: {background};
            --color-surface: {surface};
            --color-error: {error};
            --color-warning: {warning};
            --color-success: {success};
            --color-info: {info};
            --color-text-primary: {text_primary};
            --color-text-secondary: {text_secondary};
            --color-border: {border};

            --font-family: {font_family};
            --font-size-base: {font_size_base};
            --font-weight-normal: {font_weight_normal};
            --font-weight-bold: {font_weight_bold};
            --line-height: {line_height};

            --spacing-xs: {spacing_xs};
            --spacing-sm: {spacing_sm};
            --spacing-md: {spacing_md};
            --spacing-lg: {spacing_lg};
            --spacing-xl: {spacing_xl};

            --shadow-sm: {shadow_sm};
            --shadow-md: {shadow_md};
            --shadow-lg: {shadow_lg};
            --shadow-xl: {shadow_xl};

            --duration-fast: {duration_fast};
            --duration-normal: {duration_normal};
            --duration-slow: {duration_slow};
            --easing: {easing};
        }}

        * {{
            box-sizing: border-box;
        }}

        body {{
            font-family: var(--font-family);
            font-size: var(--font-size-base);
            font-weight: var(--font-weight-normal);
            line-height: var(--line-height);
            color: var(--color-text-primary);
            background-color: var(--color-background);
            margin: 0;
            padding: 0;
        }}

        .qorzen-app {{
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }}

        .app-container {{
            display: flex;
            flex-direction: column;
            min-height: 100vh;
        }}

        .app-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: var(--spacing-md);
            background-color: var(--color-surface);
            border-bottom: 1px solid var(--color-border);
            box-shadow: var(--shadow-sm);
        }}

        .app-body {{
            display: flex;
            flex: 1;
        }}

        .app-sidebar {{
            width: 250px;
            background-color: var(--color-surface);
            border-right: 1px solid var(--color-border);
            transition: width var(--duration-normal) var(--easing);
        }}

        .app-sidebar.collapsed {{
            width: 60px;
        }}

        .app-main-content {{
            flex: 1;
            overflow-y: auto;
        }}

        .app-footer {{
            padding: var(--spacing-md);
            background-color: var(--color-surface);
            border-top: 1px solid var(--color-border);
            text-align: center;
            color: var(--color-text-secondary);
        }}

        .login-screen {{
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            background-color: var(--color-background);
        }}

        .login-container {{
            background-color: var(--color-surface);
            padding: var(--spacing-xl);
            border-radius: 8px;
            box-shadow: var(--shadow-lg);
            width: 100%;
            max-width: 400px;
        }}

        .form-group {{
            margin-bottom: var(--spacing-md);
        }}

        .form-group label {{
            display: block;
            margin-bottom: var(--spacing-xs);
            font-weight: var(--font-weight-bold);
        }}

        .form-group input {{
            width: 100%;
            padding: var(--spacing-sm);
            border: 1px solid var(--color-border);
            border-radius: 4px;
            font-size: var(--font-size-base);
        }}

        .login-button {{
            width: 100%;
            padding: var(--spacing-md);
            background-color: var(--color-primary);
            color: white;
            border: none;
            border-radius: 4px;
            font-size: var(--font-size-base);
            font-weight: var(--font-weight-bold);
            cursor: pointer;
            transition: background-color var(--duration-fast) var(--easing);
        }}

        .login-button:hover {{
            background-color: color-mix(in srgb, var(--color-primary) 90%, black);
        }}

        .login-button:disabled {{
            opacity: 0.6;
            cursor: not-allowed;
        }}

        .error-message {{
            color: var(--color-error);
            margin-bottom: var(--spacing-md);
            padding: var(--spacing-sm);
            background-color: color-mix(in srgb, var(--color-error) 10%, transparent);
            border: 1px solid var(--color-error);
            border-radius: 4px;
        }}

        .sidebar-nav {{
            padding: var(--spacing-md);
        }}

        .nav-item {{
            margin-bottom: var(--spacing-xs);
        }}

        .nav-link {{
            display: flex;
            align-items: center;
            padding: var(--spacing-sm);
            color: var(--color-text-primary);
            text-decoration: none;
            border-radius: 4px;
            transition: background-color var(--duration-fast) var(--easing);
        }}

        .nav-link:hover {{
            background-color: color-mix(in srgb, var(--color-primary) 10%, transparent);
        }}

        .nav-link.active {{
            background-color: var(--color-primary);
            color: white;
        }}

        .nav-icon {{
            margin-right: var(--spacing-sm);
            width: 20px;
            text-align: center;
        }}

        .nav-badge {{
            margin-left: auto;
            padding: 2px 6px;
            border-radius: 10px;
            font-size: 0.75rem;
            font-weight: var(--font-weight-bold);
        }}

        .plugin-panel {{
            border-bottom: 1px solid var(--color-border);
        }}

        .panel-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: var(--spacing-sm);
            background: none;
            border: none;
            width: 100%;
            text-align: left;
            cursor: pointer;
            font-weight: var(--font-weight-bold);
        }}

        .panel-header:hover {{
            background-color: color-mix(in srgb, var(--color-primary) 5%, transparent);
        }}

        .panel-content {{
            padding: var(--spacing-sm);
        }}

        .user-menu {{
            position: relative;
        }}

        .user-menu-dropdown {{
            position: absolute;
            top: 100%;
            right: 0;
            background-color: var(--color-surface);
            border: 1px solid var(--color-border);
            border-radius: 4px;
            box-shadow: var(--shadow-md);
            min-width: 200px;
            z-index: 1000;
        }}

        .menu-item {{
            display: block;
            padding: var(--spacing-sm);
            color: var(--color-text-primary);
            text-decoration: none;
            border: none;
            background: none;
            width: 100%;
            text-align: left;
            cursor: pointer;
        }}

        .menu-item:hover {{
            background-color: color-mix(in srgb, var(--color-primary) 10%, transparent);
        }}

        .menu-divider {{
            margin: 0;
            border: none;
            border-top: 1px solid var(--color-border);
        }}

        .user-avatar {{
            width: 32px;
            height: 32px;
            border-radius: 50%;
            background-color: var(--color-primary);
            color: white;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: var(--font-weight-bold);
            margin-right: var(--spacing-sm);
        }}

        .user-avatar img {{
            width: 100%;
            height: 100%;
            border-radius: 50%;
            object-fit: cover;
        }}
        "#,
        primary = theme.colors.primary,
        secondary = theme.colors.secondary,
        accent = theme.colors.accent,
        background = theme.colors.background,
        surface = theme.colors.surface,
        error = theme.colors.error,
        warning = theme.colors.warning,
        success = theme.colors.success,
        info = theme.colors.info,
        text_primary = theme.colors.text_primary,
        text_secondary = theme.colors.text_secondary,
        border = theme.colors.border,
        font_family = theme.typography.font_family,
        font_size_base = theme.typography.font_size_base,
        font_weight_normal = theme.typography.font_weight_normal,
        font_weight_bold = theme.typography.font_weight_bold,
        line_height = theme.typography.line_height,
        spacing_xs = theme.spacing.xs,
        spacing_sm = theme.spacing.sm,
        spacing_md = theme.spacing.md,
        spacing_lg = theme.spacing.lg,
        spacing_xl = theme.spacing.xl,
        shadow_sm = theme.shadows.sm,
        shadow_md = theme.shadows.md,
        shadow_lg = theme.shadows.lg,
        shadow_xl = theme.shadows.xl,
        duration_fast = theme.animations.duration_fast,
        duration_normal = theme.animations.duration_normal,
        duration_slow = theme.animations.duration_slow,
        easing = theme.animations.easing,
    )
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
    fn test_theme_css_generation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = UILayoutManager::new();
            let theme = manager.default_theme().await;
            let css = get_theme_css(&theme);

            assert!(css.contains("--color-primary"));
            assert!(css.contains("--font-family"));
            assert!(css.contains(".qorzen-app"));
        });
    }
}