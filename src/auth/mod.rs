// src/auth/mod.rs - Authentication and authorization system

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::utils::Time;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::manager::{ManagedState, Manager, ManagerStatus, PlatformRequirements};

pub type UserId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub email: String,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
    pub preferences: UserPreferences,
    pub profile: UserProfile,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub ui_layout: Option<String>,
    pub is_system_role: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub resource: String, // "user.profile", "plugin.inventory", "system.config"
    pub action: String, // "read", "write", "delete", "execute"
    pub scope: PermissionScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionScope {
    Own,                // User's own resources
    Department(String), // Department-specific resources
    Global,             // All resources
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserPreferences {
    pub theme: String,
    pub language: String,
    pub timezone: String,
    pub notifications_enabled: bool,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            notifications_enabled: true,
            custom_settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub department: Option<String>,
    pub title: Option<String>,
    pub contact_info: ContactInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub address: Option<String>,
    pub emergency_contact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: UserId,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Credentials {
    Password {
        username: String,
        password: String,
    },
    Token {
        token: String,
    },
    OAuth2 {
        provider: String,
        code: String,
        state: Option<String>,
    },
    Biometric {
        user_id: UserId,
        biometric_data: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub user: User,
    pub session: UserSession,
    pub tokens: TokenPair,
    pub requires_mfa: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user ID
    pub iat: i64,    // issued at
    pub exp: i64,    // expires at
    pub aud: String, // audience
    pub iss: String, // issuer
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthProviderType {
    Local,
    OAuth2 { provider: String },
    SAML { provider: String },
    LDAP { server: String },
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, credentials: &Credentials) -> Result<AuthResult>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair>;
    async fn validate_token(&self, token: &str) -> Result<Claims>;
    fn provider_type(&self) -> AuthProviderType;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait AuthProvider: Sync {
    async fn authenticate(&self, credentials: &Credentials) -> Result<AuthResult>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair>;
    async fn validate_token(&self, token: &str) -> Result<Claims>;
    fn provider_type(&self) -> AuthProviderType;
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: UserSession) -> Result<()>;
    async fn get_session(&self, session_id: Uuid) -> Result<Option<UserSession>>;
    async fn update_session(&self, session: UserSession) -> Result<()>;
    async fn delete_session(&self, session_id: Uuid) -> Result<()>;
    async fn cleanup_expired_sessions(&self) -> Result<u64>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait SessionStore: Sync {
    async fn create_session(&self, session: UserSession) -> Result<()>;
    async fn get_session(&self, session_id: Uuid) -> Result<Option<UserSession>>;
    async fn update_session(&self, session: UserSession) -> Result<()>;
    async fn delete_session(&self, session_id: Uuid) -> Result<()>;
    async fn cleanup_expired_sessions(&self) -> Result<u64>;
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn create_user(&self, user: User) -> Result<()>;
    async fn get_user(&self, user_id: UserId) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn update_user(&self, user: User) -> Result<()>;
    async fn delete_user(&self, user_id: UserId) -> Result<()>;
    async fn list_users(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait UserStore: Sync {
    async fn create_user(&self, user: User) -> Result<()>;
    async fn get_user(&self, user_id: UserId) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn update_user(&self, user: User) -> Result<()>;
    async fn delete_user(&self, user_id: UserId) -> Result<()>;
    async fn list_users(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>>;
}

pub struct PermissionCache {
    cache: HashMap<(UserId, String, String), bool>,
    last_updated: DateTime<Utc>,
}

impl PermissionCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            last_updated: Time::now(),
        }
    }

    fn check_permission(&self, user_id: UserId, resource: &str, action: &str) -> Option<bool> {
        self.cache
            .get(&(user_id, resource.to_string(), action.to_string()))
            .copied()
    }

    fn cache_permission(&mut self, user_id: UserId, resource: &str, action: &str, allowed: bool) {
        self.cache
            .insert((user_id, resource.to_string(), action.to_string()), allowed);
        self.last_updated = Time::now();
    }

    fn clear_user_cache(&mut self, user_id: UserId) {
        self.cache.retain(|(id, _, _), _value| *id != user_id);
        self.last_updated = Time::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub password_min_length: u32,
    pub password_require_uppercase: bool,
    pub password_require_lowercase: bool,
    pub password_require_numbers: bool,
    pub password_require_symbols: bool,
    pub session_timeout_minutes: u64,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
    pub require_mfa: bool,
    pub allowed_login_methods: Vec<AuthProviderType>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_symbols: false,
            session_timeout_minutes: 480, // 8 hours
            max_login_attempts: 5,
            lockout_duration_minutes: 30,
            require_mfa: false,
            allowed_login_methods: vec![AuthProviderType::Local],
        }
    }
}

pub struct AccountManager {
    state: ManagedState,
    auth_providers: HashMap<String, Box<dyn AuthProvider>>,
    session_store: Box<dyn SessionStore>,
    permission_cache: Arc<RwLock<PermissionCache>>,
    user_store: Box<dyn UserStore>,
    security_policy: SecurityPolicy,
    current_user: Arc<RwLock<Option<User>>>,
    current_session: Arc<RwLock<Option<UserSession>>>,
}

impl std::fmt::Debug for AccountManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountManager")
            .field("security_policy", &self.security_policy)
            .finish()
    }
}

impl AccountManager {
    pub fn new(
        session_store: Box<dyn SessionStore>,
        user_store: Box<dyn UserStore>,
        security_policy: SecurityPolicy,
    ) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "account_manager"),
            auth_providers: HashMap::new(),
            session_store,
            permission_cache: Arc::new(RwLock::new(PermissionCache::new())),
            user_store,
            security_policy,
            current_user: Arc::new(RwLock::new(None)),
            current_session: Arc::new(RwLock::new(None)),
        }
    }

    pub fn register_auth_provider(&mut self, name: String, provider: Box<dyn AuthProvider>) {
        self.auth_providers.insert(name, provider);
    }

    pub async fn authenticate(
        &self,
        credentials: Credentials,
        provider: Option<&str>,
    ) -> Result<AuthResult> {
        let provider_name = provider.unwrap_or("local");
        let auth_provider = self.auth_providers.get(provider_name).ok_or_else(|| {
            Error::authentication(format!(
                "Authentication provider '{}' not found",
                provider_name
            ))
        })?;

        let auth_result = auth_provider.authenticate(&credentials).await?;

        // Store session
        self.session_store
            .create_session(auth_result.session.clone())
            .await?;

        // Update current user and session
        *self.current_user.write().await = Some(auth_result.user.clone());
        *self.current_session.write().await = Some(auth_result.session.clone());

        // Clear permission cache for user
        self.permission_cache
            .write()
            .await
            .clear_user_cache(auth_result.user.id);

        Ok(auth_result)
    }

    pub async fn validate_token(&self, token: &str, provider: Option<&str>) -> Result<Claims> {
        let provider_name = provider.unwrap_or("local");
        let auth_provider = self.auth_providers.get(provider_name).ok_or_else(|| {
            Error::authentication(format!(
                "Authentication provider '{}' not found",
                provider_name
            ))
        })?;

        auth_provider.validate_token(token).await
    }

    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        provider: Option<&str>,
    ) -> Result<TokenPair> {
        let provider_name = provider.unwrap_or("local");
        let auth_provider = self.auth_providers.get(provider_name).ok_or_else(|| {
            Error::authentication(format!(
                "Authentication provider '{}' not found",
                provider_name
            ))
        })?;

        auth_provider.refresh_token(refresh_token).await
    }

    pub async fn logout(&self, session_id: Option<Uuid>) -> Result<()> {
        if let Some(id) = session_id {
            self.session_store.delete_session(id).await?;
        } else if let Some(session) = self.current_session.read().await.as_ref() {
            self.session_store.delete_session(session.id).await?;
        }

        // Clear current user and session
        *self.current_user.write().await = None;
        *self.current_session.write().await = None;

        Ok(())
    }

    pub async fn current_user(&self) -> Option<User> {
        self.current_user.read().await.clone()
    }

    pub async fn current_session(&self) -> Option<UserSession> {
        self.current_session.read().await.clone()
    }

    pub async fn check_permission(
        &self,
        user_id: UserId,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        // Check cache first
        if let Some(cached) = self
            .permission_cache
            .read()
            .await
            .check_permission(user_id, resource, action)
        {
            return Ok(cached);
        }

        // Load user and check permissions
        let user = self
            .user_store
            .get_user(user_id)
            .await?
            .ok_or_else(|| Error::authorization(resource, action, "User not found"))?;

        let has_permission = self.user_has_permission(&user, resource, action);

        // Cache the result
        self.permission_cache.write().await.cache_permission(
            user_id,
            resource,
            action,
            has_permission,
        );

        Ok(has_permission)
    }

    pub async fn check_current_user_permission(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        let user = self
            .current_user()
            .await
            .ok_or_else(|| Error::authorization(resource, action, "No authenticated user"))?;

        self.check_permission(user.id, resource, action).await
    }

    pub async fn create_user(&self, mut user: User) -> Result<()> {
        // Set creation timestamp
        user.created_at = Time::now();
        user.id = Uuid::new_v4();

        // Store user
        self.user_store.create_user(user).await?;

        Ok(())
    }

    pub async fn update_user(&self, user: User) -> Result<()> {
        self.user_store.update_user(user.clone()).await?;

        // Clear permission cache for updated user
        self.permission_cache
            .write()
            .await
            .clear_user_cache(user.id);

        // Update current user if it's the same
        if let Some(current) = self.current_user.read().await.as_ref() {
            if current.id == user.id {
                *self.current_user.write().await = Some(user);
            }
        }

        Ok(())
    }

    pub async fn get_user(&self, user_id: UserId) -> Result<Option<User>> {
        self.user_store.get_user(user_id).await
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        self.user_store.get_user_by_username(username).await
    }

    pub async fn list_users(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>> {
        self.user_store.list_users(limit, offset).await
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        self.session_store.cleanup_expired_sessions().await
    }

    fn user_has_permission(&self, user: &User, resource: &str, action: &str) -> bool {
        // Check direct permissions
        for permission in &user.permissions {
            if self.permission_matches(permission, resource, action) {
                return true;
            }
        }

        // Check role permissions
        for role in &user.roles {
            for permission in &role.permissions {
                if self.permission_matches(permission, resource, action) {
                    return true;
                }
            }
        }

        false
    }

    fn permission_matches(&self, permission: &Permission, resource: &str, action: &str) -> bool {
        // Simple string matching - in practice you'd want more sophisticated matching
        (permission.resource == resource || permission.resource == "*")
            && (permission.action == action || permission.action == "*")
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Manager for AccountManager {
    fn name(&self) -> &str {
        "account_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Initialize default auth providers, create admin user if needed, etc.

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Cleanup sessions, etc.
        let _ = self.cleanup_expired_sessions().await;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        let current_user_id = self
            .current_user()
            .await
            .map(|u| u.id.to_string())
            .unwrap_or_else(|| "none".to_string());
        status.add_metadata("current_user", serde_json::Value::String(current_user_id));
        status.add_metadata(
            "auth_providers",
            serde_json::Value::from(self.auth_providers.len()),
        );
        status.add_metadata(
            "security_policy",
            serde_json::to_value(&self.security_policy).unwrap_or_default(),
        );

        status
    }

    fn required_permissions(&self) -> Vec<String> {
        vec![
            "auth.login".to_string(),
            "auth.logout".to_string(),
            "auth.manage_users".to_string(),
        ]
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: false,
            requires_network: true,
            requires_database: true,
            requires_native_apis: false,
            minimum_permissions: self.required_permissions(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Manager for AccountManager {
    fn name(&self) -> &str {
        "account_manager"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Initialize default auth providers, create admin user if needed, etc.

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Cleanup sessions, etc.
        let _ = self.cleanup_expired_sessions().await;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        let current_user_id = self
            .current_user()
            .await
            .map(|u| u.id.to_string())
            .unwrap_or_else(|| "none".to_string());
        status.add_metadata("current_user", serde_json::Value::String(current_user_id));
        status.add_metadata(
            "auth_providers",
            serde_json::Value::from(self.auth_providers.len()),
        );
        status.add_metadata(
            "security_policy",
            serde_json::to_value(&self.security_policy).unwrap_or_default(),
        );

        status
    }

    fn required_permissions(&self) -> Vec<String> {
        vec![
            "auth.login".to_string(),
            "auth.logout".to_string(),
            "auth.manage_users".to_string(),
        ]
    }

    fn platform_requirements(&self) -> PlatformRequirements {
        PlatformRequirements {
            requires_filesystem: false,
            requires_network: true,
            requires_database: true,
            requires_native_apis: false,
            minimum_permissions: self.required_permissions(),
        }
    }
}

pub struct MemorySessionStore {
    sessions: Arc<RwLock<HashMap<Uuid, UserSession>>>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl SessionStore for MemorySessionStore {
    async fn create_session(&self, session: UserSession) -> Result<()> {
        self.sessions.write().await.insert(session.id, session);
        Ok(())
    }

    async fn get_session(&self, session_id: Uuid) -> Result<Option<UserSession>> {
        Ok(self.sessions.read().await.get(&session_id).cloned())
    }

    async fn update_session(&self, session: UserSession) -> Result<()> {
        self.sessions.write().await.insert(session.id, session);
        Ok(())
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        self.sessions.write().await.remove(&session_id);
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let now = Time::now();
        let mut sessions = self.sessions.write().await;
        let original_count = sessions.len();

        sessions.retain(|_, session| session.expires_at > now);

        Ok((original_count - sessions.len()) as u64)
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl SessionStore for MemorySessionStore {
    async fn create_session(&self, session: UserSession) -> Result<()> {
        self.sessions.write().await.insert(session.id, session);
        Ok(())
    }

    async fn get_session(&self, session_id: Uuid) -> Result<Option<UserSession>> {
        Ok(self.sessions.read().await.get(&session_id).cloned())
    }

    async fn update_session(&self, session: UserSession) -> Result<()> {
        self.sessions.write().await.insert(session.id, session);
        Ok(())
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        self.sessions.write().await.remove(&session_id);
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let now = Time::now();
        let mut sessions = self.sessions.write().await;
        let original_count = sessions.len();

        sessions.retain(|_, session| session.expires_at > now);

        Ok((original_count - sessions.len()) as u64)
    }
}

pub struct MemoryUserStore {
    users: Arc<RwLock<HashMap<UserId, User>>>,
}

impl MemoryUserStore {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl UserStore for MemoryUserStore {
    async fn create_user(&self, user: User) -> Result<()> {
        self.users.write().await.insert(user.id, user);
        Ok(())
    }

    async fn get_user(&self, user_id: UserId) -> Result<Option<User>> {
        Ok(self.users.read().await.get(&user_id).cloned())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.username == username)
            .cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn update_user(&self, user: User) -> Result<()> {
        self.users.write().await.insert(user.id, user);
        Ok(())
    }

    async fn delete_user(&self, user_id: UserId) -> Result<()> {
        self.users.write().await.remove(&user_id);
        Ok(())
    }

    async fn list_users(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>> {
        let users: Vec<User> = self.users.read().await.values().cloned().collect();

        let offset = offset.unwrap_or(0) as usize;
        let limit = limit.map(|l| l as usize);

        let mut result: Vec<User> = users.into_iter().skip(offset).collect();

        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl UserStore for MemoryUserStore {
    async fn create_user(&self, user: User) -> Result<()> {
        self.users.write().await.insert(user.id, user);
        Ok(())
    }

    async fn get_user(&self, user_id: UserId) -> Result<Option<User>> {
        Ok(self.users.read().await.get(&user_id).cloned())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.username == username)
            .cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .read()
            .await
            .values()
            .find(|u| u.email == email)
            .cloned())
    }

    async fn update_user(&self, user: User) -> Result<()> {
        self.users.write().await.insert(user.id, user);
        Ok(())
    }

    async fn delete_user(&self, user_id: UserId) -> Result<()> {
        self.users.write().await.remove(&user_id);
        Ok(())
    }

    async fn list_users(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<User>> {
        let users: Vec<User> = self.users.read().await.values().cloned().collect();

        let offset = offset.unwrap_or(0) as usize;
        let limit = limit.map(|l| l as usize);

        let mut result: Vec<User> = users.into_iter().skip(offset).collect();

        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_creation() {
        let user_store = Box::new(MemoryUserStore::new());
        let session_store = Box::new(MemorySessionStore::new());
        let security_policy = SecurityPolicy::default();

        let mut account_manager = AccountManager::new(session_store, user_store, security_policy);
        account_manager.initialize().await.unwrap();

        let user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            roles: vec![],
            permissions: vec![],
            preferences: UserPreferences::default(),
            profile: UserProfile {
                display_name: "Test User".to_string(),
                avatar_url: None,
                bio: None,
                department: None,
                title: None,
                contact_info: ContactInfo {
                    phone: None,
                    address: None,
                    emergency_contact: None,
                },
            },
            created_at: Time::now(),
            last_login: None,
            is_active: true,
        };

        account_manager.create_user(user.clone()).await.unwrap();

        let retrieved_user = account_manager
            .get_user_by_username("testuser")
            .await
            .unwrap();
        assert!(retrieved_user.is_some());
        assert_eq!(retrieved_user.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let user_store = Box::new(MemoryUserStore::new());
        let session_store = Box::new(MemorySessionStore::new());
        let security_policy = SecurityPolicy::default();

        let account_manager = AccountManager::new(session_store, user_store, security_policy);

        let permission = Permission {
            resource: "user.profile".to_string(),
            action: "read".to_string(),
            scope: PermissionScope::Own,
        };

        let user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            roles: vec![],
            permissions: vec![permission],
            preferences: UserPreferences::default(),
            profile: UserProfile {
                display_name: "Test User".to_string(),
                avatar_url: None,
                bio: None,
                department: None,
                title: None,
                contact_info: ContactInfo {
                    phone: None,
                    address: None,
                    emergency_contact: None,
                },
            },
            created_at: Time::now(),
            last_login: None,
            is_active: true,
        };

        account_manager.create_user(user.clone()).await.unwrap();

        let has_permission = account_manager
            .check_permission(user.id, "user.profile", "read")
            .await
            .unwrap();
        assert!(has_permission);

        let no_permission = account_manager
            .check_permission(user.id, "admin.users", "write")
            .await
            .unwrap();
        assert!(!no_permission);
    }
}