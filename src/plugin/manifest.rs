// src/plugin/manifest.rs

use crate::auth::Permission;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin manifest file structure (plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginMetadata,
    /// Build configuration
    pub build: BuildConfig,
    /// Target platform specifications
    pub targets: HashMap<String, TargetConfig>,
    /// Plugin dependencies
    pub dependencies: HashMap<String, DependencySpec>,
    /// Required permissions
    pub permissions: Vec<String>,
    /// API hooks this plugin provides
    pub provides: Vec<String>,
    /// API hooks this plugin requires
    pub requires: Vec<String>,
    /// Search integration configuration
    pub search: Option<SearchConfig>,
    /// Plugin settings schema
    pub settings: Option<serde_json::Value>,
}

/// Plugin metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub minimum_core_version: String,
    pub api_version: String,
}

/// Build configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Entry point for the plugin
    pub entry: String,
    /// Additional source files
    pub sources: Vec<String>,
    /// Build features to enable
    pub features: Vec<String>,
    /// Whether this plugin supports hot reloading
    pub hot_reload: bool,
    /// Build-time dependencies
    pub build_dependencies: HashMap<String, String>,
}

/// Target platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Platform this target supports (web, desktop, mobile)
    pub platform: String,
    /// Architecture constraints
    pub arch: Option<Vec<String>>,
    /// OS constraints
    pub os: Option<Vec<String>>,
    /// Platform-specific entry point
    pub entry: Option<String>,
    /// Platform-specific features
    pub features: Vec<String>,
    /// Platform-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    pub version: String,
    pub optional: bool,
    pub features: Vec<String>,
    pub platform: Option<String>,
}

/// Search integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Search providers this plugin implements
    pub providers: Vec<SearchProvider>,
    /// Search result types this plugin can return
    pub result_types: Vec<String>,
    /// Fields that should be indexed for search
    pub indexed_fields: Vec<String>,
    /// Custom search filters
    pub filters: HashMap<String, FilterConfig>,
}

/// Search provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProvider {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: i32,
    pub supports_facets: bool,
    pub supports_autocomplete: bool,
}

/// Filter configuration for search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub field_type: String,
    pub operators: Vec<String>,
    pub values: Option<Vec<serde_json::Value>>,
}

#[allow(dead_code)]
impl PluginManifest {
    /// Load plugin manifest from TOML file (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn load_from_file(path: &std::path::Path) -> Result<Self> {
        use tokio::fs;

        let content = fs::read_to_string(path).await.map_err(|e| {
            Error::file(
                path.display().to_string(),
                crate::error::FileOperation::Read,
                format!("Failed to read manifest file: {}", e),
            )
        })?;

        let manifest: PluginManifest = toml::from_str(&content).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Failed to parse manifest TOML: {}", e),
            )
        })?;

        manifest.validate()?;
        Ok(manifest)
    }

    /// Load plugin manifest from TOML file (WASM - from platform manager)
    #[cfg(target_arch = "wasm32")]
    pub async fn load_from_file(path: &std::path::Path) -> Result<Self> {
        // In WASM, we need to use the platform manager's file system
        // For now, return an error indicating files should be loaded differently
        Err(Error::platform(
            "wasm",
            "filesystem",
            format!("Direct file loading not supported in WASM. Use load_from_str with content fetched via platform manager: {}", path.display())
        ))
    }

    /// Load plugin manifest from platform file system
    pub async fn load_from_platform(
        path: &str,
        filesystem: &dyn crate::platform::filesystem::FileSystemProvider,
    ) -> Result<Self> {
        let content_bytes = filesystem.read_file(path).await.map_err(|e| {
            Error::file(
                path.to_string(),
                crate::error::FileOperation::Read,
                format!("Failed to read manifest file via platform: {}", e),
            )
        })?;

        let content = String::from_utf8(content_bytes).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Invalid UTF-8 in manifest file: {}", e),
            )
        })?;

        Self::load_from_str(&content)
    }

    /// Load plugin manifest from TOML string
    pub fn load_from_str(content: &str) -> Result<Self> {
        let manifest: PluginManifest = toml::from_str(content).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Failed to parse manifest TOML: {}", e),
            )
        })?;

        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest for correctness
    pub fn validate(&self) -> Result<()> {
        // Validate plugin ID format
        if !self
            .plugin
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Error::plugin(
                &self.plugin.id,
                "Plugin ID must contain only alphanumeric characters, underscores, and hyphens",
            ));
        }

        if self.plugin.id.is_empty() {
            return Err(Error::plugin(&self.plugin.id, "Plugin ID cannot be empty"));
        }

        // Validate version format
        if self.plugin.version.is_empty() {
            return Err(Error::plugin(
                &self.plugin.id,
                "Plugin version cannot be empty",
            ));
        }

        if !self.plugin.version.chars().any(|c| c.is_numeric()) {
            return Err(Error::plugin(
                &self.plugin.id,
                "Plugin version must contain at least one number",
            ));
        }

        // Validate API version compatibility
        let core_api_version = env!("CARGO_PKG_VERSION");
        if !self.is_api_compatible(core_api_version) {
            return Err(Error::plugin(
                &self.plugin.id,
                format!(
                    "Plugin API version {} is not compatible with core version {}",
                    self.plugin.api_version, core_api_version
                ),
            ));
        }

        // Validate build configuration
        if self.build.entry.is_empty() {
            return Err(Error::plugin(
                &self.plugin.id,
                "Build entry point cannot be empty",
            ));
        }

        // Validate search configuration if present
        if let Some(ref search_config) = self.search {
            for provider in &search_config.providers {
                if provider.id.is_empty() {
                    return Err(Error::plugin(
                        &self.plugin.id,
                        "Search provider ID cannot be empty",
                    ));
                }
                if provider.name.is_empty() {
                    return Err(Error::plugin(
                        &self.plugin.id,
                        "Search provider name cannot be empty",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if plugin is compatible with target platform
    pub fn is_platform_compatible(&self, platform: &str) -> bool {
        if self.targets.is_empty() {
            return true; // No platform restrictions
        }

        self.targets
            .values()
            .any(|target| target.platform == platform || target.platform == "all")
    }

    /// Check if API version is compatible
    pub fn is_api_compatible(&self, core_version: &str) -> bool {
        // Simple semantic version check
        let plugin_parts: Vec<&str> = self.plugin.api_version.split('.').collect();
        let core_parts: Vec<&str> = core_version.split('.').collect();

        if plugin_parts.len() < 2 || core_parts.len() < 2 {
            return false;
        }

        // Major version must match, minor version can be equal or lower
        plugin_parts[0] == core_parts[0]
            && plugin_parts[1].parse::<u32>().unwrap_or(0)
                <= core_parts[1].parse::<u32>().unwrap_or(0)
    }

    /// Get target configuration for specific platform
    pub fn get_target_config(&self, platform: &str) -> Option<&TargetConfig> {
        self.targets
            .values()
            .find(|target| target.platform == platform || target.platform == "all")
    }

    /// Get required permissions as Permission structs
    pub fn get_required_permissions(&self) -> Vec<Permission> {
        self.permissions
            .iter()
            .filter_map(|perm_str| {
                let parts: Vec<&str> = perm_str.split('.').collect();
                if parts.len() == 2 {
                    Some(Permission {
                        resource: parts[0].to_string(),
                        action: parts[1].to_string(),
                        scope: crate::auth::PermissionScope::Global,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if plugin provides a specific capability
    pub fn provides_capability(&self, capability: &str) -> bool {
        self.provides.contains(&capability.to_string())
    }

    /// Check if plugin requires a specific capability
    pub fn requires_capability(&self, capability: &str) -> bool {
        self.requires.contains(&capability.to_string())
    }

    /// Get all dependencies for a specific platform
    pub fn get_platform_dependencies(&self, platform: &str) -> Vec<(&String, &DependencySpec)> {
        self.dependencies
            .iter()
            .filter(|(_, dep)| dep.platform.as_ref().is_none_or(|p| p == platform))
            .collect()
    }

    /// Serialize manifest to TOML string
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string(self).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::Serialization,
                format!("Failed to serialize manifest to TOML: {}", e),
            )
        })
    }

    /// Generate example manifest for plugin development
    pub fn example() -> Self {
        Self {
            plugin: PluginMetadata {
                id: "example_plugin".to_string(),
                name: "Example Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "An example plugin demonstrating the plugin system".to_string(),
                author: "Plugin Developer".to_string(),
                license: "MIT".to_string(),
                homepage: Some("https://example.com".to_string()),
                repository: Some("https://github.com/example/plugin".to_string()),
                keywords: vec!["example".to_string(), "demo".to_string()],
                categories: vec!["utility".to_string()],
                minimum_core_version: "0.1.0".to_string(),
                api_version: "0.1.0".to_string(),
            },
            build: BuildConfig {
                entry: "src/lib.rs".to_string(),
                sources: vec!["src/**/*.rs".to_string()],
                features: vec!["default".to_string()],
                hot_reload: true,
                build_dependencies: HashMap::new(),
            },
            targets: {
                let mut targets = HashMap::new();
                targets.insert(
                    "web".to_string(),
                    TargetConfig {
                        platform: "web".to_string(),
                        arch: Some(vec!["wasm32".to_string()]),
                        os: None,
                        entry: None,
                        features: vec!["web".to_string()],
                        settings: HashMap::new(),
                    },
                );
                targets.insert(
                    "desktop".to_string(),
                    TargetConfig {
                        platform: "desktop".to_string(),
                        arch: Some(vec!["x86_64".to_string(), "aarch64".to_string()]),
                        os: Some(vec![
                            "windows".to_string(),
                            "macos".to_string(),
                            "linux".to_string(),
                        ]),
                        entry: None,
                        features: vec!["desktop".to_string()],
                        settings: HashMap::new(),
                    },
                );
                targets
            },
            dependencies: HashMap::new(),
            permissions: vec!["data.read".to_string(), "ui.render".to_string()],
            provides: vec!["search.provider".to_string(), "api.routes".to_string()],
            requires: vec!["database.query".to_string(), "http.client".to_string()],
            search: Some(SearchConfig {
                providers: vec![SearchProvider {
                    id: "example_search".to_string(),
                    name: "Example Search".to_string(),
                    description: "Example search provider".to_string(),
                    priority: 100,
                    supports_facets: true,
                    supports_autocomplete: true,
                }],
                result_types: vec!["example_item".to_string()],
                indexed_fields: vec!["title".to_string(), "description".to_string()],
                filters: HashMap::new(),
            }),
            settings: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "api_endpoint": {
                        "type": "string",
                        "description": "API endpoint URL"
                    },
                    "cache_duration": {
                        "type": "integer",
                        "description": "Cache duration in seconds",
                        "default": 300
                    }
                }
            })),
        }
    }

    /// Create a minimal manifest for testing
    pub fn minimal(id: &str, name: &str) -> Self {
        Self {
            plugin: PluginMetadata {
                id: id.to_string(),
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: "A minimal plugin".to_string(),
                author: "Test Author".to_string(),
                license: "MIT".to_string(),
                homepage: None,
                repository: None,
                keywords: vec![],
                categories: vec![],
                minimum_core_version: "0.1.0".to_string(),
                api_version: "0.1.0".to_string(),
            },
            build: BuildConfig {
                entry: "src/lib.rs".to_string(),
                sources: vec!["src/**/*.rs".to_string()],
                features: vec!["default".to_string()],
                hot_reload: false,
                build_dependencies: HashMap::new(),
            },
            targets: HashMap::new(),
            dependencies: HashMap::new(),
            permissions: vec![],
            provides: vec![],
            requires: vec![],
            search: None,
            settings: None,
        }
    }
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self::minimal("default_plugin", "Default Plugin")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_validation() {
        let manifest = PluginManifest::example();
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_manifest_validation_errors() {
        let mut manifest = PluginManifest::example();

        // Test empty plugin ID
        manifest.plugin.id = String::new();
        assert!(manifest.validate().is_err());

        // Test invalid plugin ID
        manifest.plugin.id = "invalid plugin id!".to_string();
        assert!(manifest.validate().is_err());

        // Reset to valid ID
        manifest.plugin.id = "valid_plugin_id".to_string();

        // Test empty version
        manifest.plugin.version = String::new();
        assert!(manifest.validate().is_err());

        // Test invalid version
        manifest.plugin.version = "invalid".to_string();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_platform_compatibility() {
        let manifest = PluginManifest::example();
        assert!(manifest.is_platform_compatible("web"));
        assert!(manifest.is_platform_compatible("desktop"));
        assert!(!manifest.is_platform_compatible("mobile"));
    }

    #[test]
    fn test_api_compatibility() {
        let manifest = PluginManifest::example();
        assert!(manifest.is_api_compatible("0.1.5"));
        assert!(manifest.is_api_compatible("0.2.0"));
        assert!(!manifest.is_api_compatible("1.0.0"));
    }

    #[test]
    fn test_permission_parsing() {
        let manifest = PluginManifest::example();
        let permissions = manifest.get_required_permissions();

        assert_eq!(permissions.len(), 2);
        assert!(permissions
            .iter()
            .any(|p| p.resource == "data" && p.action == "read"));
        assert!(permissions
            .iter()
            .any(|p| p.resource == "ui" && p.action == "render"));
    }

    #[test]
    fn test_capabilities() {
        let manifest = PluginManifest::example();

        assert!(manifest.provides_capability("search.provider"));
        assert!(manifest.provides_capability("api.routes"));
        assert!(!manifest.provides_capability("non.existent"));

        assert!(manifest.requires_capability("database.query"));
        assert!(manifest.requires_capability("http.client"));
        assert!(!manifest.requires_capability("non.existent"));
    }

    #[tokio::test]
    async fn test_manifest_serialization() {
        let manifest = PluginManifest::example();
        let toml_str = manifest.to_toml_string().unwrap();
        let parsed = PluginManifest::load_from_str(&toml_str).unwrap();
        assert_eq!(manifest.plugin.id, parsed.plugin.id);
        assert_eq!(manifest.plugin.name, parsed.plugin.name);
    }

    #[test]
    fn test_minimal_manifest() {
        let manifest = PluginManifest::minimal("test", "Test Plugin");
        assert_eq!(manifest.plugin.id, "test");
        assert_eq!(manifest.plugin.name, "Test Plugin");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_platform_dependencies() {
        let mut manifest = PluginManifest::example();

        // Add platform-specific dependency
        manifest.dependencies.insert(
            "web_dependency".to_string(),
            DependencySpec {
                version: "1.0.0".to_string(),
                optional: false,
                features: vec![],
                platform: Some("web".to_string()),
            },
        );

        manifest.dependencies.insert(
            "universal_dependency".to_string(),
            DependencySpec {
                version: "1.0.0".to_string(),
                optional: false,
                features: vec![],
                platform: None,
            },
        );

        let web_deps = manifest.get_platform_dependencies("web");
        assert_eq!(web_deps.len(), 2); // web_dependency + universal_dependency

        let desktop_deps = manifest.get_platform_dependencies("desktop");
        assert_eq!(desktop_deps.len(), 1); // Only universal_dependency
    }
}
