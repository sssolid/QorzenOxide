// src/platform/web.rs

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;

use crate::error::{Error, Result};
use crate::platform::*;
use crate::platform::database::DatabaseBounds;
use crate::platform::network::NetworkBounds;
use crate::platform::storage::StorageBounds;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc<'_> = wee_alloc::WeeAlloc::INIT;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

pub fn create_providers() -> Result<PlatformProviders> {
    Ok(PlatformProviders {
        filesystem: Arc::new(WebFileSystem::new()?),
        database: Arc::new(IndexedDbDatabase::new()?),
        network: Arc::new(FetchNetwork::new()),
        storage: Arc::new(WebStorage::new()?),
    })
}

pub fn detect_capabilities() -> PlatformCapabilities {
    let _window = window().unwrap();

    PlatformCapabilities {
        has_filesystem: false,
        has_database: true,
        has_background_tasks: false,
        has_push_notifications: false, // Simplified for now
        has_biometric_auth: false,
        has_camera: false, // Simplified for now
        has_location: false, // Simplified for now
        max_file_size: Some(50 * 1024 * 1024),
        supported_formats: vec![
            "txt".to_string(),
            "json".to_string(),
            "csv".to_string(),
            "jpg".to_string(),
            "png".to_string(),
            "gif".to_string(),
        ],
        platform_name: "web".to_string(),
        platform_version: "1.0".to_string(),
    }
}

pub async fn initialize() -> Result<()> {
    console::log_1(&"Initializing web platform".into());
    Ok(())
}

pub async fn cleanup() -> Result<()> {
    console::log_1(&"Cleaning up web platform".into());
    Ok(())
}

pub struct WebFileSystem;

impl WebFileSystem {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl filesystem::FileSystemBounds for WebFileSystem {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl FileSystemProvider for WebFileSystem {
    async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.fetch_from_url(path).await
        } else {
            Err(Error::platform(
                "web",
                "filesystem",
                "Only HTTP/HTTPS URLs supported in web platform",
            ))
        }
    }

    async fn write_file(&self, _path: &str, _data: &[u8]) -> Result<()> {
        Err(Error::platform(
            "web",
            "filesystem",
            "File writing not supported in web platform",
        ))
    }

    async fn delete_file(&self, _path: &str) -> Result<()> {
        Err(Error::platform(
            "web",
            "filesystem",
            "File deletion not supported in web platform",
        ))
    }

    async fn list_directory(&self, _path: &str) -> Result<Vec<FileInfo>> {
        Err(Error::platform(
            "web",
            "filesystem",
            "Directory listing not supported in web platform",
        ))
    }

    async fn create_directory(&self, _path: &str) -> Result<()> {
        Err(Error::platform(
            "web",
            "filesystem",
            "Directory creation not supported in web platform",
        ))
    }

    async fn file_exists(&self, _path: &str) -> bool {
        false
    }

    async fn get_metadata(&self, _path: &str) -> Result<FileMetadata> {
        Err(Error::platform(
            "web",
            "filesystem",
            "File metadata not available in web platform",
        ))
    }
}

impl WebFileSystem {
    async fn fetch_from_url(&self, url: &str) -> Result<Vec<u8>> {
        let window = window().unwrap();
        let request = Request::new_with_str(url).unwrap();

        let response_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| Error::platform("web", "filesystem", format!("Fetch failed: {:?}", e)))?;

        let response: Response = response_value.dyn_into().unwrap();

        if !response.ok() {
            return Err(Error::platform(
                "web",
                "filesystem",
                format!("HTTP {}", response.status()),
            ));
        }

        let array_buffer = JsFuture::from(response.array_buffer().unwrap())
            .await
            .map_err(|e| {
                Error::platform(
                    "web",
                    "filesystem",
                    format!("Failed to read response: {:?}", e),
                )
            })?;

        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        Ok(uint8_array.to_vec())
    }
}

pub struct IndexedDbDatabase {
    database_name: String,
}

impl IndexedDbDatabase {
    pub fn new() -> Result<Self> {
        Ok(Self {
            database_name: "qorzen_db".to_string(),
        })
    }
}

impl DatabaseBounds for IndexedDbDatabase {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DatabaseProvider for IndexedDbDatabase {
    async fn execute(&self, _query: &str, _params: &[serde_json::Value]) -> Result<QueryResult> {
        Err(Error::platform(
            "web",
            "database",
            "IndexedDB execute not implemented",
        ))
    }

    async fn query(&self, _query: &str, _params: &[serde_json::Value]) -> Result<Vec<Row>> {
        Ok(Vec::new())
    }

    async fn migrate(&self, _migrations: &[Migration]) -> Result<()> {
        Ok(())
    }
}

pub struct FetchNetwork;

impl FetchNetwork {
    pub fn new() -> Self {
        Self
    }
}

impl NetworkBounds for FetchNetwork {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl NetworkProvider for FetchNetwork {
    async fn request(&self, request: NetworkRequest) -> Result<NetworkResponse> {
        let window = web_sys::window().unwrap();

        // Everything non-Send is scoped and dropped before .await
        let fetch_promise = {
            let mut opts = RequestInit::new();
            opts.set_method(&request.method);

            if let Some(body) = request.body {
                let uint8_array = js_sys::Uint8Array::from(&body[..]);
                opts.set_body(&uint8_array.into());
            }

            let req = Request::new_with_str_and_init(&request.url, &opts).map_err(|e| {
                Error::platform("web", "network", format!("Failed to create request: {:?}", e))
            })?;

            window.fetch_with_request(&req) // <- return immediately
        };

        // No web_sys types live across this await
        let response_value = JsFuture::from(fetch_promise)
            .await
            .map_err(|e| Error::platform("web", "network", format!("Fetch failed: {:?}", e)))?;

        let response: Response = response_value.dyn_into().unwrap();
        let status_code = response.status() as u16;

        let headers = HashMap::new(); // Simplified

        let body = JsFuture::from(response.array_buffer().unwrap())
            .await
            .map_err(|e| {
                Error::platform(
                    "web",
                    "network",
                    format!("Failed to read response body: {:?}", e),
                )
            })?;

        let uint8_array = js_sys::Uint8Array::new(&body);

        Ok(NetworkResponse {
            status_code,
            headers,
            body: uint8_array.to_vec(),
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

pub struct WebStorage;

impl WebStorage {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    fn get_storage(&self) -> Result<Storage> {
        window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
            .ok_or_else(|| Error::platform("web", "storage", "localStorage not available"))
    }
}

impl StorageBounds for WebStorage {}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl StorageProvider for WebStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let storage = self.get_storage()?;

        match storage.get_item(key) {
            Ok(Some(value)) => Ok(Some(value.into_bytes())),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::platform(
                "web",
                "storage",
                format!("Failed to get item: {:?}", e),
            )),
        }
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let storage = self.get_storage()?;
        let value_str = String::from_utf8_lossy(value);

        storage
            .set_item(key, &value_str)
            .map_err(|e| Error::platform("web", "storage", format!("Failed to set item: {:?}", e)))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let storage = self.get_storage()?;

        storage.remove_item(key).map_err(|e| {
            Error::platform("web", "storage", format!("Failed to delete item: {:?}", e))
        })
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let storage = self.get_storage()?;
        let mut keys = Vec::new();

        let length = storage.length().map_err(|e| {
            Error::platform(
                "web",
                "storage",
                format!("Failed to get storage length: {:?}", e),
            )
        })?;

        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(prefix) {
                    keys.push(key);
                }
            }
        }

        Ok(keys)
    }

    async fn clear(&self) -> Result<()> {
        let storage = self.get_storage()?;

        storage.clear().map_err(|e| {
            Error::platform(
                "web",
                "storage",
                format!("Failed to clear storage: {:?}", e),
            )
        })
    }
}