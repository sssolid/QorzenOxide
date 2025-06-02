// src/platform/native.rs - Native platform implementations

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;

use crate::error::Error;
use crate::error::Result;
use crate::platform::database::DatabaseBounds;
use crate::platform::network::NetworkBounds;
use crate::platform::storage::StorageBounds;
use crate::platform::*;

/// Creates native platform providers
pub fn create_providers() -> Result<PlatformProviders> {
    Ok(PlatformProviders {
        filesystem: Arc::new(NativeFileSystem::new()?),
        database: Arc::new(SqliteDatabase::new()?),
        network: Arc::new(NativeNetwork::new()),
        storage: Arc::new(NativeStorage::new()?),
    })
}

/// Detects native platform capabilities
pub fn detect_capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        has_filesystem: true,
        has_database: true,
        has_background_tasks: true,
        has_push_notifications: cfg!(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux"
        )),
        has_biometric_auth: cfg!(any(target_os = "macos", target_os = "windows")),
        has_camera: true,
        has_location: true,
        max_file_size: Some(u64::MAX),
        supported_formats: vec![
            "txt".to_string(),
            "json".to_string(),
            "yaml".to_string(),
            "toml".to_string(),
            "xml".to_string(),
            "csv".to_string(),
            "jpg".to_string(),
            "png".to_string(),
            "gif".to_string(),
            "mp4".to_string(),
            "mp3".to_string(),
            "pdf".to_string(),
        ],
        platform_name: std::env::consts::OS.to_string(),
        platform_version: "1.0".to_string(),
    }
}

/// Platform initialization
pub async fn initialize() -> Result<()> {
    // Create required directories
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("data"))
        .join("qorzen");

    fs::create_dir_all(&data_dir).await.map_err(|e| {
        Error::platform(
            "native",
            "filesystem",
            format!("Failed to create data directory: {}", e),
        )
    })?;

    Ok(())
}

/// Platform cleanup
pub async fn cleanup() -> Result<()> {
    // Cleanup temporary files, etc.
    Ok(())
}

/// Native filesystem implementation
pub struct NativeFileSystem {
    base_path: std::path::PathBuf,
}

impl NativeFileSystem {
    pub fn new() -> Result<Self> {
        let base_path = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("data"))
            .join("qorzen");

        Ok(Self { base_path })
    }

    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        if path.starts_with('/') || path.contains(':') {
            // Absolute path
            std::path::PathBuf::from(path)
        } else {
            // Relative to base path
            self.base_path.join(path)
        }
    }
}

impl filesystem::FileSystemBounds for NativeFileSystem {}

#[async_trait]
impl FileSystemProvider for NativeFileSystem {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path);
        fs::read(&full_path).await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to read file {}: {}", path, e),
            )
        })
    }

    async fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let full_path = self.resolve_path(path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::platform(
                    "native",
                    "filesystem",
                    format!("Failed to create directory: {}", e),
                )
            })?;
        }

        fs::write(&full_path, data).await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to write file {}: {}", path, e),
            )
        })
    }

    async fn delete_file(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::remove_file(&full_path).await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to delete file {}: {}", path, e),
            )
        })
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>> {
        let full_path = self.resolve_path(path);
        let mut entries = fs::read_dir(&full_path).await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to read directory {}: {}", path, e),
            )
        })?;

        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to read directory entry: {}", e),
            )
        })? {
            let metadata = entry.metadata().await.map_err(|e| {
                Error::platform(
                    "native",
                    "filesystem",
                    format!("Failed to read metadata: {}", e),
                )
            })?;

            let file_info = FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry.path().to_string_lossy().to_string(),
                size: metadata.len(),
                is_directory: metadata.is_dir(),
                modified: metadata
                    .modified()
                    .map(chrono::DateTime::from)
                    .unwrap_or_else(|_| chrono::Utc::now()),
            };
            files.push(file_info);
        }

        Ok(files)
    }

    async fn create_directory(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path);
        fs::create_dir_all(&full_path).await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to create directory {}: {}", path, e),
            )
        })
    }

    async fn file_exists(&self, path: &str) -> bool {
        let full_path = self.resolve_path(path);
        full_path.exists()
    }

    async fn get_metadata(&self, path: &str) -> Result<FileMetadata> {
        let full_path = self.resolve_path(path);
        let metadata = fs::metadata(&full_path).await.map_err(|e| {
            Error::platform(
                "native",
                "filesystem",
                format!("Failed to get metadata for {}: {}", path, e),
            )
        })?;

        Ok(FileMetadata {
            size: metadata.len(),
            is_directory: metadata.is_dir(),
            is_readonly: metadata.permissions().readonly(),
            created: metadata.created().map(chrono::DateTime::from).ok(),
            modified: metadata
                .modified()
                .map(chrono::DateTime::from)
                .unwrap_or_else(|_| chrono::Utc::now()),
            accessed: metadata.accessed().map(chrono::DateTime::from).ok(),
        })
    }
}

/// SQLite database implementation
pub struct SqliteDatabase {
    // Database connection would be here
    _db_path: std::path::PathBuf,
}

impl SqliteDatabase {
    fn new() -> Result<Self> {
        let db_path = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("data"))
            .join("qorzen")
            .join("app.db");

        Ok(Self { _db_path: db_path })
    }
}

impl DatabaseBounds for SqliteDatabase {}

#[async_trait]
impl DatabaseProvider for SqliteDatabase {
    async fn execute(&self, _query: &str, _params: &[serde_json::Value]) -> Result<QueryResult> {
        // Implementation would use SQLite
        Ok(QueryResult {
            rows_affected: 0,
            last_insert_id: None,
        })
    }

    async fn query(&self, _query: &str, _params: &[serde_json::Value]) -> Result<Vec<Row>> {
        // Implementation would use SQLite
        Ok(Vec::new())
    }

    async fn migrate(&self, _migrations: &[Migration]) -> Result<()> {
        // Implementation would apply migrations
        Ok(())
    }
}

/// Native network implementation
pub struct NativeNetwork {
    client: reqwest::Client,
}

impl NativeNetwork {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl NetworkBounds for NativeNetwork {}

#[async_trait]
impl NetworkProvider for NativeNetwork {
    async fn request(&self, request: NetworkRequest) -> Result<NetworkResponse> {
        let mut req = match request.method.as_str() {
            "GET" => self.client.get(&request.url),
            "POST" => self.client.post(&request.url),
            "PUT" => self.client.put(&request.url),
            "DELETE" => self.client.delete(&request.url),
            _ => {
                return Err(Error::platform(
                    "native",
                    "network",
                    format!("Unsupported HTTP method: {}", request.method),
                ))
            }
        };

        for (key, value) in request.headers {
            req = req.header(&key, &value);
        }

        if let Some(body) = request.body {
            req = req.body(body);
        }

        if let Some(timeout_ms) = request.timeout_ms {
            req = req.timeout(std::time::Duration::from_millis(timeout_ms));
        }

        let response = req.send().await.map_err(|e| {
            Error::platform("native", "network", format!("HTTP request failed: {}", e))
        })?;

        let status_code = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = response
            .bytes()
            .await
            .map_err(|e| {
                Error::platform(
                    "native",
                    "network",
                    format!("Failed to read response body: {}", e),
                )
            })?
            .to_vec();

        Ok(NetworkResponse {
            status_code,
            headers,
            body,
        })
    }

    async fn upload_file(&self, url: &str, file_data: &[u8]) -> Result<NetworkResponse> {
        let request = NetworkRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: Some(file_data.to_vec()),
            timeout_ms: Some(30000),
        };
        self.request(request).await
    }

    async fn download_file(&self, url: &str) -> Result<Vec<u8>> {
        let request = NetworkRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            timeout_ms: Some(30000),
        };
        let response = self.request(request).await?;
        Ok(response.body)
    }
}

/// Native storage implementation (using filesystem)
pub struct NativeStorage {
    storage_path: std::path::PathBuf,
}

impl NativeStorage {
    fn new() -> Result<Self> {
        let storage_path = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("data"))
            .join("qorzen")
            .join("storage");

        Ok(Self { storage_path })
    }

    fn key_to_path(&self, key: &str) -> std::path::PathBuf {
        let safe_key = key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        self.storage_path.join(format!("{}.bin", safe_key))
    }
}

impl StorageBounds for NativeStorage {}

#[async_trait]
impl StorageProvider for NativeStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.key_to_path(key);
        match fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::platform(
                "native",
                "storage",
                format!("Failed to read key {}: {}", key, e),
            )),
        }
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let path = self.key_to_path(key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::platform(
                    "native",
                    "storage",
                    format!("Failed to create storage directory: {}", e),
                )
            })?;
        }

        fs::write(&path, value).await.map_err(|e| {
            Error::platform(
                "native",
                "storage",
                format!("Failed to write key {}: {}", key, e),
            )
        })
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.key_to_path(key);
        fs::remove_file(&path).await.map_err(|e| {
            Error::platform(
                "native",
                "storage",
                format!("Failed to delete key {}: {}", key, e),
            )
        })
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut entries = fs::read_dir(&self.storage_path).await.map_err(|e| {
            Error::platform(
                "native",
                "storage",
                format!("Failed to read storage directory: {}", e),
            )
        })?;

        let mut keys = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            Error::platform(
                "native",
                "storage",
                format!("Failed to read storage entry: {}", e),
            )
        })? {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(key) = name.strip_suffix(".bin") {
                    if key.starts_with(prefix) {
                        keys.push(key.to_string());
                    }
                }
            }
        }

        Ok(keys)
    }

    async fn clear(&self) -> Result<()> {
        if self.storage_path.exists() {
            fs::remove_dir_all(&self.storage_path).await.map_err(|e| {
                Error::platform(
                    "native",
                    "storage",
                    format!("Failed to clear storage: {}", e),
                )
            })?;
        }

        fs::create_dir_all(&self.storage_path).await.map_err(|e| {
            Error::platform(
                "native",
                "storage",
                format!("Failed to recreate storage directory: {}", e),
            )
        })?;

        Ok(())
    }
}
