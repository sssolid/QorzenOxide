// src/ui/state.rs - Application state management and context

use dioxus::prelude::*;

pub(crate) use crate::auth::{User, UserSession};
use crate::ui::{UILayout, Theme, Notification};

/// Application state context that provides global state to all components
#[derive(Debug, Clone)]
pub struct AppStateContext {
    pub current_user: Option<User>,
    pub current_session: Option<UserSession>,
    pub current_layout: UILayout,
    pub current_theme: Theme,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub notifications: Vec<Notification>,
    pub sidebar_collapsed: bool,
    pub mobile_menu_open: bool,
}

impl Default for AppStateContext {
    fn default() -> Self {
        Self {
            current_user: None,
            current_session: None,
            current_layout: UILayout::default(),
            current_theme: Theme::default(),
            is_loading: false,
            error_message: None,
            notifications: Vec::new(),
            sidebar_collapsed: false,
            mobile_menu_open: false,
        }
    }
}

/// Actions that can be performed on the application state
#[derive(Debug, Clone)]
pub enum AppAction {
    SetUser(Option<User>),
    SetSession(Option<UserSession>),
    SetLayout(UILayout),
    SetTheme(Theme),
    SetLoading(bool),
    SetError(Option<String>),
    AddNotification(Notification),
    RemoveNotification(uuid::Uuid),
    MarkNotificationRead(uuid::Uuid),
    ClearNotifications,
    ToggleSidebar,
    SetSidebarCollapsed(bool),
    ToggleMobileMenu,
    SetMobileMenuOpen(bool),
}

/// State reducer function
pub fn app_state_reducer(state: &AppStateContext, action: AppAction) -> AppStateContext {
    let mut new_state = state.clone();

    match action {
        AppAction::SetUser(user) => {
            new_state.current_user = user;
        }
        AppAction::SetSession(session) => {
            new_state.current_session = session;
        }
        AppAction::SetLayout(layout) => {
            new_state.current_layout = layout;
        }
        AppAction::SetTheme(theme) => {
            new_state.current_theme = theme;
        }
        AppAction::SetLoading(loading) => {
            new_state.is_loading = loading;
        }
        AppAction::SetError(error) => {
            new_state.error_message = error;
        }
        AppAction::AddNotification(notification) => {
            new_state.notifications.push(notification);
        }
        AppAction::RemoveNotification(id) => {
            new_state.notifications.retain(|n| n.id != id);
        }
        AppAction::MarkNotificationRead(id) => {
            if let Some(notification) = new_state.notifications.iter_mut().find(|n| n.id == id) {
                notification.read = true;
            }
        }
        AppAction::ClearNotifications => {
            new_state.notifications.clear();
        }
        AppAction::ToggleSidebar => {
            new_state.sidebar_collapsed = !new_state.sidebar_collapsed;
        }
        AppAction::SetSidebarCollapsed(collapsed) => {
            new_state.sidebar_collapsed = collapsed;
        }
        AppAction::ToggleMobileMenu => {
            new_state.mobile_menu_open = !new_state.mobile_menu_open;
        }
        AppAction::SetMobileMenuOpen(open) => {
            new_state.mobile_menu_open = open;
        }
    }

    new_state
}

/// Application state provider component
#[component]
pub fn AppStateProvider(children: Element) -> Element {
    // Create separate signals for different parts of the state
    let mut current_user = use_signal(|| None::<User>);
    let mut current_session = use_signal(|| None::<UserSession>);
    let mut current_layout = use_signal(|| UILayout::default());
    let mut current_theme = use_signal(|| Theme::default());
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut notifications = use_signal(|| Vec::<Notification>::new());
    let mut sidebar_collapsed = use_signal(|| false);
    let mut mobile_menu_open = use_signal(|| false);

    // Create state accessor that builds the context from individual signals
    let get_state = use_callback(move |_: ()| AppStateContext {
        current_user: current_user(),
        current_session: current_session(),
        current_layout: current_layout(),
        current_theme: current_theme(),
        is_loading: is_loading(),
        error_message: error_message(),
        notifications: notifications(),
        sidebar_collapsed: sidebar_collapsed(),
        mobile_menu_open: mobile_menu_open(),
    });

    // Create dispatch function that updates individual signals
    let dispatch = use_callback(move |action: AppAction| {
        match action {
            AppAction::SetUser(user) => current_user.set(user),
            AppAction::SetSession(session) => current_session.set(session),
            AppAction::SetLayout(layout) => current_layout.set(layout),
            AppAction::SetTheme(theme) => current_theme.set(theme),
            AppAction::SetLoading(loading) => is_loading.set(loading),
            AppAction::SetError(error) => error_message.set(error),
            AppAction::AddNotification(notification) => {
                notifications.with_mut(|n| n.push(notification));
            }
            AppAction::RemoveNotification(id) => {
                notifications.with_mut(|n| n.retain(|notification| notification.id != id));
            }
            AppAction::MarkNotificationRead(id) => {
                notifications.with_mut(|n| {
                    if let Some(notification) = n.iter_mut().find(|notification| notification.id == id) {
                        notification.read = true;
                    }
                });
            }
            AppAction::ClearNotifications => notifications.set(Vec::new()),
            AppAction::ToggleSidebar => sidebar_collapsed.set(!sidebar_collapsed()),
            AppAction::SetSidebarCollapsed(collapsed) => sidebar_collapsed.set(collapsed),
            AppAction::ToggleMobileMenu => mobile_menu_open.set(!mobile_menu_open()),
            AppAction::SetMobileMenuOpen(open) => mobile_menu_open.set(open),
        }
    });

    // Provide the state accessor and dispatch functions
    use_context_provider(|| get_state);
    use_context_provider(|| dispatch);

    // Initialize mock data (only once)
    let mut initialized = use_signal(|| false);
    if !initialized() {
        initialized.set(true);

        // Add mock notifications
        notifications.with_mut(|n| {
            n.push(Notification {
                id: uuid::Uuid::new_v4(),
                title: "Welcome to Qorzen!".to_string(),
                message: "Your application is ready to use.".to_string(),
                notification_type: crate::ui::NotificationType::Info,
                timestamp: chrono::Utc::now(),
                read: false,
                actions: vec![],
            });

            n.push(Notification {
                id: uuid::Uuid::new_v4(),
                title: "System Update".to_string(),
                message: "A new version is available for download.".to_string(),
                notification_type: crate::ui::NotificationType::System,
                timestamp: chrono::Utc::now() - chrono::Duration::hours(2),
                read: false,
                actions: vec![],
            });
        });
    }

    rsx! {
        {children}
    }
}

/// Hook to access the current application state
pub fn use_app_state() -> AppStateContext {
    let get_state = use_context::<Callback<(), AppStateContext>>();
    get_state(())
}

/// Hook to dispatch actions to the application state
pub fn use_app_dispatch() -> Callback<AppAction> {
    use_context::<Callback<AppAction>>()
}

/// Hook that provides both state and dispatch
pub fn use_app_state_with_dispatch() -> (AppStateContext, Callback<AppAction>) {
    let state = use_app_state();
    let dispatch = use_app_dispatch();
    (state, dispatch)
}

/// Authentication helper hooks
pub mod auth {
    use super::*;
    use crate::auth::{User, UserSession, Credentials};

    /// Hook for login functionality
    pub fn use_login() -> Callback<Credentials, ()> {
        let dispatch = use_app_dispatch();

        use_callback(move |credentials: Credentials| {
            let dispatch = dispatch.clone();

            // Mock login - in real app, this would call the auth service
            spawn({
                let dispatch = dispatch.clone();
                async move {
                    dispatch(AppAction::SetLoading(true));

                    // Simulate API call delay
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(1000).await;

                    // Mock successful login
                    let mock_user = User {
                        id: uuid::Uuid::new_v4(),
                        username: "demo_user".to_string(),
                        email: "demo@qorzen.com".to_string(),
                        roles: vec![],
                        permissions: vec![],
                        preferences: crate::auth::UserPreferences::default(),
                        profile: crate::auth::UserProfile {
                            display_name: "Demo User".to_string(),
                            avatar_url: None,
                            bio: Some("A demonstration user account".to_string()),
                            department: Some("Engineering".to_string()),
                            title: Some("Software Developer".to_string()),
                            contact_info: crate::auth::ContactInfo {
                                phone: None,
                                address: None,
                                emergency_contact: None,
                            },
                        },
                        created_at: chrono::Utc::now() - chrono::Duration::days(30),
                        last_login: Some(chrono::Utc::now()),
                        is_active: true,
                    };

                    let mock_session = UserSession {
                        id: uuid::Uuid::new_v4(),
                        user_id: mock_user.id,
                        created_at: chrono::Utc::now(),
                        expires_at: chrono::Utc::now() + chrono::Duration::hours(8),
                        last_activity: chrono::Utc::now(),
                        ip_address: Some("127.0.0.1".to_string()),
                        user_agent: Some("Qorzen App".to_string()),
                        is_active: true,
                    };

                    dispatch(AppAction::SetUser(Some(mock_user)));
                    dispatch(AppAction::SetSession(Some(mock_session)));
                    dispatch(AppAction::SetLoading(false));
                    dispatch(AppAction::AddNotification(Notification {
                        id: uuid::Uuid::new_v4(),
                        title: "Login Successful".to_string(),
                        message: "Welcome back! You have been successfully logged in.".to_string(),
                        notification_type: crate::ui::NotificationType::Success,
                        timestamp: chrono::Utc::now(),
                        read: false,
                        actions: vec![],
                    }));
                }
            });
        })
    }

    /// Hook for logout functionality
    pub fn use_logout() -> Callback<(), ()> {
        let dispatch = use_app_dispatch();

        use_callback(move |_| {
            let dispatch = dispatch.clone();

            spawn({
                let dispatch = dispatch.clone();
                async move {
                    dispatch(AppAction::SetLoading(true));

                    // Simulate API call delay
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(500).await;

                    dispatch(AppAction::SetUser(None));
                    dispatch(AppAction::SetSession(None));
                    dispatch(AppAction::ClearNotifications);
                    dispatch(AppAction::SetLoading(false));
                }
            });
        })
    }

    /// Hook to check if user is authenticated
    pub fn use_is_authenticated() -> bool {
        let state = use_app_state();
        state.current_user.is_some()
    }

    /// Hook to get current user
    pub fn use_current_user() -> Option<User> {
        let state = use_app_state();
        state.current_user
    }

    /// Hook to check user permissions
    pub fn use_has_permission() -> impl Fn(&str, &str) -> bool {
        let get_state = use_context::<Callback<(), AppStateContext>>();

        move |resource: &str, action: &str| {
            let state = get_state(());

            match &state.current_user {
                Some(user) => {
                    // Check direct permissions
                    let direct = user.permissions.iter().any(|perm| {
                        (perm.resource == resource || perm.resource == "*") &&
                            (perm.action == action || perm.action == "*")
                    });

                    // Check role permissions
                    let role_based = user.roles.iter().any(|role| {
                        role.permissions.iter().any(|perm| {
                            (perm.resource == resource || perm.resource == "*") &&
                                (perm.action == action || perm.action == "*")
                        })
                    });

                    direct || role_based
                }
                None => false,
            }
        }
    }
}

/// UI state helper hooks
pub mod ui {
    use super::*;

    /// Hook for sidebar state management
    pub fn use_sidebar() -> (bool, Callback<(), ()>, Callback<bool, ()>) {
        let state = use_app_state();
        let dispatch = use_app_dispatch();

        let toggle = use_callback({
            let dispatch = dispatch.clone();
            move |_| dispatch(AppAction::ToggleSidebar)
        });

        let set_collapsed = use_callback({
            move |collapsed: bool| dispatch(AppAction::SetSidebarCollapsed(collapsed))
        });

        (state.sidebar_collapsed, toggle, set_collapsed)
    }

    /// Hook for mobile menu state management
    pub fn use_mobile_menu() -> (bool, Callback<(), ()>, Callback<bool, ()>) {
        let state = use_app_state();
        let dispatch = use_app_dispatch();

        let toggle = use_callback({
            let dispatch = dispatch.clone();
            move |_| dispatch(AppAction::ToggleMobileMenu)
        });

        let set_open = use_callback({
            move |open: bool| dispatch(AppAction::SetMobileMenuOpen(open))
        });

        (state.mobile_menu_open, toggle, set_open)
    }

    /// Hook for notification management
    pub fn use_notifications() -> (Vec<Notification>, Callback<uuid::Uuid, ()>, Callback<uuid::Uuid, ()>, Callback<(), ()>) {
        let state = use_app_state();
        let dispatch = use_app_dispatch();

        let remove = use_callback({
            let dispatch = dispatch.clone();
            move |id: uuid::Uuid| dispatch(AppAction::RemoveNotification(id))
        });

        let mark_read = use_callback({
            let dispatch = dispatch.clone();
            move |id: uuid::Uuid| dispatch(AppAction::MarkNotificationRead(id))
        });

        let clear_all = use_callback({
            move |_| dispatch(AppAction::ClearNotifications)
        });

        (state.notifications, remove, mark_read, clear_all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_app_state() {
        let state = AppStateContext::default();
        assert!(state.current_user.is_none());
        assert!(state.current_session.is_none());
        assert!(!state.is_loading);
        assert!(state.error_message.is_none());
        assert!(state.notifications.is_empty());
        assert!(!state.sidebar_collapsed);
        assert!(!state.mobile_menu_open);
    }

    #[test]
    fn test_app_state_reducer() {
        let initial_state = AppStateContext::default();

        // Test setting loading state
        let new_state = app_state_reducer(&initial_state, AppAction::SetLoading(true));
        assert!(new_state.is_loading);

        // Test toggling sidebar
        let new_state = app_state_reducer(&initial_state, AppAction::ToggleSidebar);
        assert!(new_state.sidebar_collapsed);

        // Test setting error
        let error_msg = "Test error".to_string();
        let new_state = app_state_reducer(&initial_state, AppAction::SetError(Some(error_msg.clone())));
        assert_eq!(new_state.error_message, Some(error_msg));
    }

    #[test]
    fn test_notification_actions() {
        let initial_state = AppStateContext::default();
        let notification = Notification {
            id: uuid::Uuid::new_v4(),
            title: "Test".to_string(),
            message: "Test message".to_string(),
            notification_type: crate::ui::NotificationType::Info,
            timestamp: chrono::Utc::now(),
            read: false,
            actions: vec![],
        };

        // Test adding notification
        let new_state = app_state_reducer(&initial_state, AppAction::AddNotification(notification.clone()));
        assert_eq!(new_state.notifications.len(), 1);

        // Test marking as read
        let newer_state = app_state_reducer(&new_state, AppAction::MarkNotificationRead(notification.id));
        assert!(newer_state.notifications[0].read);

        // Test removing notification
        let final_state = app_state_reducer(&newer_state, AppAction::RemoveNotification(notification.id));
        assert!(final_state.notifications.is_empty());
    }
}