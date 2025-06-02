// example_plugin/src/lib.rs

//! Product Catalog Plugin
//!
//! This plugin provides product catalog functionality that can work with both
//! web (API-based) and desktop (direct database) environments.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use qorzen_oxide::{
    plugin::*,
    plugin::search::*,
    auth::{Permission, PermissionScope},
    error::{Error, Result},
    event::Event,
    utils::Time,
    config::SettingsSchema,
};

/// Product data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub price: f64,
    pub currency: String,
    pub stock_quantity: i32,
    pub sku: String,
    pub barcode: Option<String>,
    pub images: Vec<String>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub api_endpoint: Option<String>,
    pub database_url: Option<String>,
    pub use_api: bool,
    pub cache_duration_secs: u64,
    pub search_enabled: bool,
    pub max_results: usize,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            api_endpoint: Some("https://api.example.com/products".to_string()),
            database_url: None,
            use_api: cfg!(target_arch = "wasm32"), // Default to API for web
            cache_duration_secs: 300,
            search_enabled: true,
            max_results: 100,
        }
    }
}

/// Product data source trait
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
trait ProductDataSource: Send + Sync {
    async fn get_products(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Product>>;
    async fn get_product(&self, id: &str) -> Result<Option<Product>>;
    async fn search_products(&self, query: &str, limit: Option<usize>) -> Result<Vec<Product>>;
    async fn get_categories(&self) -> Result<Vec<String>>;
}

/// API-based data source for web environments
#[derive(Debug)]
struct ApiDataSource {
    endpoint: String,
    // Store HTTP client configuration instead of the client itself for better WASM compatibility
    timeout_secs: u64,
}

impl ApiDataSource {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            timeout_secs: 30,
        }
    }

    async fn make_request(&self, url: &str) -> Result<serde_json::Value> {
        // Platform-specific HTTP client creation
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(self.timeout_secs))
                .build()
                .map_err(|e| Error::plugin("product_catalog", format!("Failed to create HTTP client: {}", e)))?;

            let response = client.get(url).send().await
                .map_err(|e| Error::plugin("product_catalog", format!("API request failed: {}", e)))?;

            if !response.status().is_success() {
                return Err(Error::plugin("product_catalog",
                                         format!("API returned status: {}", response.status())));
            }

            let json: serde_json::Value = response.json().await
                .map_err(|e| Error::plugin("product_catalog", format!("Failed to parse response: {}", e)))?;

            Ok(json)
        }

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen_futures::JsFuture;
            use web_sys::*;

            let window = web_sys::window()
                .ok_or_else(|| Error::plugin("product_catalog", "No window object available"))?;

            let request = Request::new_with_str(url)
                .map_err(|_| Error::plugin("product_catalog", "Failed to create request"))?;

            let response_promise = window.fetch_with_request(&request);
            let response_value = JsFuture::from(response_promise).await
                .map_err(|_| Error::plugin("product_catalog", "Fetch failed"))?;

            let response: Response = response_value.dyn_into()
                .map_err(|_| Error::plugin("product_catalog", "Invalid response object"))?;

            if !response.ok() {
                return Err(Error::plugin("product_catalog",
                                         format!("API returned status: {}", response.status())));
            }

            let json_promise = response.json()
                .map_err(|_| Error::plugin("product_catalog", "Failed to get JSON from response"))?;

            let json_value = JsFuture::from(json_promise).await
                .map_err(|_| Error::plugin("product_catalog", "Failed to parse JSON"))?;

            let json: serde_json::Value = json_value.into_serde()
                .map_err(|e| Error::plugin("product_catalog", format!("Failed to deserialize JSON: {}", e)))?;

            Ok(json)
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl ProductDataSource for ApiDataSource {
    async fn get_products(&self, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<Product>> {
        let mut url = format!("{}/products", self.endpoint);
        let mut params = Vec::new();

        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={}", offset));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let json = self.make_request(&url).await?;
        let products: Vec<Product> = serde_json::from_value(json)
            .map_err(|e| Error::plugin("product_catalog", format!("Failed to parse products: {}", e)))?;

        Ok(products)
    }

    async fn get_product(&self, id: &str) -> Result<Option<Product>> {
        let url = format!("{}/products/{}", self.endpoint, id);

        match self.make_request(&url).await {
            Ok(json) => {
                let product: Product = serde_json::from_value(json)
                    .map_err(|e| Error::plugin("product_catalog", format!("Failed to parse product: {}", e)))?;
                Ok(Some(product))
            }
            Err(e) => {
                // Check if it's a 404 error (product not found)
                if e.to_string().contains("404") {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn search_products(&self, query: &str, limit: Option<usize>) -> Result<Vec<Product>> {
        let encoded_query = urlencoding::encode(query);
        let mut url = format!("{}/products/search?q={}", self.endpoint, encoded_query);

        if let Some(limit) = limit {
            url.push_str(&format!("&limit={}", limit));
        }

        let json = self.make_request(&url).await?;
        let products: Vec<Product> = serde_json::from_value(json)
            .map_err(|e| Error::plugin("product_catalog", format!("Failed to parse search results: {}", e)))?;

        Ok(products)
    }

    async fn get_categories(&self) -> Result<Vec<String>> {
        let url = format!("{}/categories", self.endpoint);
        let json = self.make_request(&url).await?;
        let categories: Vec<String> = serde_json::from_value(json)
            .map_err(|e| Error::plugin("product_catalog", format!("Failed to parse categories: {}", e)))?;

        Ok(categories)
    }
}

/// Database data source for desktop environments
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct DatabaseDataSource {
    // In a real implementation, this would hold a database connection
    _database_url: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl DatabaseDataSource {
    fn new(database_url: String) -> Self {
        Self {
            _database_url: database_url,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl ProductDataSource for DatabaseDataSource {
    async fn get_products(&self, _limit: Option<usize>, _offset: Option<usize>) -> Result<Vec<Product>> {
        // Mock implementation - in reality this would query the database
        Ok(vec![
            Product {
                id: "prod_001".to_string(),
                name: "Sample Product".to_string(),
                description: "A sample product from database".to_string(),
                category: "Electronics".to_string(),
                price: 299.99,
                currency: "USD".to_string(),
                stock_quantity: 50,
                sku: "SAMPLE-001".to_string(),
                barcode: Some("1234567890123".to_string()),
                images: vec!["https://example.com/product1.jpg".to_string()],
                attributes: HashMap::new(),
                created_at: Time::now(),
                updated_at: Time::now(),
                is_active: true,
            }
        ])
    }

    async fn get_product(&self, id: &str) -> Result<Option<Product>> {
        // Mock implementation
        if id == "prod_001" {
            Ok(Some(Product {
                id: "prod_001".to_string(),
                name: "Sample Product".to_string(),
                description: "A sample product from database".to_string(),
                category: "Electronics".to_string(),
                price: 299.99,
                currency: "USD".to_string(),
                stock_quantity: 50,
                sku: "SAMPLE-001".to_string(),
                barcode: Some("1234567890123".to_string()),
                images: vec!["https://example.com/product1.jpg".to_string()],
                attributes: HashMap::new(),
                created_at: Time::now(),
                updated_at: Time::now(),
                is_active: true,
            }))
        } else {
            Ok(None)
        }
    }

    async fn search_products(&self, query: &str, _limit: Option<usize>) -> Result<Vec<Product>> {
        // Mock search implementation
        if query.to_lowercase().contains("sample") {
            self.get_products(None, None).await
        } else {
            Ok(vec![])
        }
    }

    async fn get_categories(&self) -> Result<Vec<String>> {
        Ok(vec![
            "Electronics".to_string(),
            "Books".to_string(),
            "Clothing".to_string(),
            "Home & Garden".to_string(),
        ])
    }
}

/// Main plugin implementation
pub struct ProductCatalogPlugin {
    config: PluginConfig,
    data_source: Option<Arc<dyn ProductDataSource>>,
    search_provider: Option<Arc<ProductSearchProvider>>,
    product_cache: Arc<RwLock<HashMap<String, (Product, DateTime<Utc>)>>>,
    context: Option<PluginContext>,
}

impl std::fmt::Debug for ProductCatalogPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProductCatalogPlugin")
            .field("config", &self.config)
            .finish()
    }
}

impl ProductCatalogPlugin {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
            data_source: None,
            search_provider: None,
            product_cache: Arc::new(RwLock::new(HashMap::new())),
            context: None,
        }
    }

    async fn initialize_data_source(&mut self, context: &PluginContext) -> Result<()> {
        // Load configuration from context
        if let Ok(Some(config_value)) = context.api_client.get_config("product_catalog").await {
            if let Ok(config) = serde_json::from_value::<PluginConfig>(config_value) {
                self.config = config;
            }
        }

        // Initialize appropriate data source based on configuration and platform
        let data_source: Arc<dyn ProductDataSource> = if self.config.use_api {
            if let Some(ref endpoint) = self.config.api_endpoint {
                Arc::new(ApiDataSource::new(endpoint.clone()))
            } else {
                return Err(Error::plugin("product_catalog", "API endpoint not configured"));
            }
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(ref db_url) = self.config.database_url {
                    Arc::new(DatabaseDataSource::new(db_url.clone()))
                } else {
                    return Err(Error::plugin("product_catalog", "Database URL not configured"));
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                return Err(Error::plugin("product_catalog", "Database access not available in web environment"));
            }
        };

        self.data_source = Some(data_source.clone());

        // Initialize search provider if enabled
        if self.config.search_enabled {
            self.search_provider = Some(Arc::new(ProductSearchProvider {
                data_source: data_source.clone(),
                config: self.config.clone(),
            }));
        }

        Ok(())
    }

    async fn get_cached_product(&self, id: &str) -> Option<Product> {
        let cache = self.product_cache.read().await;
        if let Some((product, cached_at)) = cache.get(id) {
            let age = Time::now().signed_duration_since(*cached_at);
            if age.num_seconds() < self.config.cache_duration_secs as i64 {
                return Some(product.clone());
            }
        }
        None
    }

    async fn cache_product(&self, product: Product) {
        let mut cache = self.product_cache.write().await;
        cache.insert(product.id.clone(), (product, Time::now()));
    }
}

impl Default for ProductCatalogPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Plugin for ProductCatalogPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "product_catalog".to_string(),
            name: "Product Catalog".to_string(),
            version: "1.0.0".to_string(),
            description: "Product catalog management with search capabilities".to_string(),
            author: "Qorzen Team".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://github.com/qorzen/plugins/product-catalog".to_string()),
            repository: Some("https://github.com/qorzen/plugins".to_string()),
            minimum_core_version: "0.1.0".to_string(),
            supported_platforms: vec![Platform::All],
        }
    }

    fn required_dependencies(&self) -> Vec<PluginDependency> {
        vec![]
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![
            Permission {
                resource: "products".to_string(),
                action: "read".to_string(),
                scope: PermissionScope::Global,
            },
            Permission {
                resource: "search".to_string(),
                action: "provide".to_string(),
                scope: PermissionScope::Global,
            },
            Permission {
                resource: "ui".to_string(),
                action: "render".to_string(),
                scope: PermissionScope::Global,
            },
        ]
    }

    async fn initialize(&mut self, context: PluginContext) -> Result<()> {
        self.initialize_data_source(&context).await?;
        self.context = Some(context);

        // Register search provider if available
        if let Some(ref _search_provider) = self.search_provider {
            // In a real implementation, we would register with the search coordinator
            // context.search_coordinator.register_provider(search_provider.clone()).await?;
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Cleanup resources
        self.data_source = None;
        self.search_provider = None;
        self.context = None;
        Ok(())
    }

    fn ui_components(&self) -> Vec<UIComponent> {
        vec![
            UIComponent {
                id: "product_list".to_string(),
                name: "Product List".to_string(),
                component_type: ComponentType::Page,
                props: serde_json::json!({
                    "title": "Products",
                    "searchable": true
                }),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
            },
            UIComponent {
                id: "product_detail".to_string(),
                name: "Product Detail".to_string(),
                component_type: ComponentType::Page,
                props: serde_json::json!({
                    "editable": false
                }),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
            },
        ]
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "products".to_string(),
                label: "Products".to_string(),
                icon: Some("📦".to_string()),
                route: Some("/plugins/product_catalog/products".to_string()),
                action: None,
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                order: 100,
                children: vec![
                    MenuItem {
                        id: "product_list".to_string(),
                        label: "All Products".to_string(),
                        icon: Some("📋".to_string()),
                        route: Some("/plugins/product_catalog/products".to_string()),
                        action: None,
                        required_permissions: vec![],
                        order: 0,
                        children: vec![],
                    },
                    MenuItem {
                        id: "product_categories".to_string(),
                        label: "Categories".to_string(),
                        icon: Some("🏷️".to_string()),
                        route: Some("/plugins/product_catalog/categories".to_string()),
                        action: None,
                        required_permissions: vec![],
                        order: 1,
                        children: vec![],
                    },
                ],
            }
        ]
    }

    fn settings_schema(&self) -> Option<SettingsSchema> {
        Some(SettingsSchema {
            version: "1.0".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "api_endpoint": {
                        "type": "string",
                        "title": "API Endpoint",
                        "description": "URL for the product API"
                    },
                    "database_url": {
                        "type": "string",
                        "title": "Database URL",
                        "description": "Database connection string"
                    },
                    "use_api": {
                        "type": "boolean",
                        "title": "Use API",
                        "description": "Whether to use API or direct database access",
                        "default": true
                    },
                    "cache_duration_secs": {
                        "type": "integer",
                        "title": "Cache Duration (seconds)",
                        "description": "How long to cache product data",
                        "default": 300,
                        "minimum": 0
                    },
                    "search_enabled": {
                        "type": "boolean",
                        "title": "Enable Search",
                        "description": "Enable search provider functionality",
                        "default": true
                    },
                    "max_results": {
                        "type": "integer",
                        "title": "Max Results",
                        "description": "Maximum number of results to return",
                        "default": 100,
                        "minimum": 1,
                        "maximum": 1000
                    }
                }
            }),
            defaults: serde_json::to_value(PluginConfig::default()).unwrap_or_default(),
        })
    }

    fn api_routes(&self) -> Vec<ApiRoute> {
        vec![
            ApiRoute {
                path: "/api/plugins/product_catalog/products".to_string(),
                method: HttpMethod::GET,
                handler_id: "list_products".to_string(),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                rate_limit: Some(RateLimit {
                    requests_per_minute: 60,
                    burst_limit: 10,
                }),
                documentation: ApiDocumentation {
                    summary: "List products".to_string(),
                    description: "Get a list of products with optional pagination".to_string(),
                    parameters: vec![
                        ApiParameter {
                            name: "limit".to_string(),
                            parameter_type: ParameterType::Query,
                            required: false,
                            description: "Maximum number of products to return".to_string(),
                            example: Some(serde_json::json!(10)),
                        },
                        ApiParameter {
                            name: "offset".to_string(),
                            parameter_type: ParameterType::Query,
                            required: false,
                            description: "Number of products to skip".to_string(),
                            example: Some(serde_json::json!(0)),
                        },
                    ],
                    responses: vec![
                        ApiResponse {
                            status_code: 200,
                            description: "List of products".to_string(),
                            schema: Some(serde_json::json!({
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "id": {"type": "string"},
                                        "name": {"type": "string"},
                                        "price": {"type": "number"}
                                    }
                                }
                            })),
                        }
                    ],
                    examples: vec![],
                },
            }
        ]
    }

    fn event_handlers(&self) -> Vec<EventHandler> {
        vec![
            EventHandler {
                event_type: "product.updated".to_string(),
                handler_id: "handle_product_update".to_string(),
                priority: 100,
            }
        ]
    }

    fn render_component(&self, component_id: &str, _props: serde_json::Value) -> Result<dioxus::prelude::VNode> {
        use dioxus::prelude::*;

        match component_id {
            "product_list" => {
                Ok(rsx! {
                    div { class: "product-list",
                        h2 { "Product Catalog" }
                        p { "This is a placeholder for the product list component." }
                        div { class: "notice",
                            "Component rendering would be implemented with actual product data in a real plugin."
                        }
                    }
                })
            }
            "product_detail" => {
                Ok(rsx! {
                    div { class: "product-detail",
                        h2 { "Product Details" }
                        p { "This is a placeholder for the product detail component." }
                        div { class: "notice",
                            "Component rendering would be implemented with actual product data in a real plugin."
                        }
                    }
                })
            }
            _ => Err(Error::plugin("product_catalog", "Unknown component"))
        }
    }

    async fn handle_api_request(&self, route_id: &str, request: ApiRequest) -> Result<ApiResponse> {
        match route_id {
            "list_products" => {
                if let Some(ref data_source) = self.data_source {
                    let limit = request.query_params.get("limit")
                        .and_then(|s| s.parse().ok());
                    let offset = request.query_params.get("offset")
                        .and_then(|s| s.parse().ok());

                    let products = data_source.get_products(limit, offset).await?;

                    Ok(ApiResponse {
                        status_code: 200,
                        description: "Success".to_string(),
                        schema: Some(serde_json::to_value(&products)
                            .map_err(|e| Error::plugin("product_catalog", format!("Serialization failed: {}", e)))?),
                    })
                } else {
                    Err(Error::plugin("product_catalog", "Data source not initialized"))
                }
            }
            _ => Err(Error::plugin("product_catalog", "Unknown API route"))
        }
    }

    async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()> {
        match handler_id {
            "handle_product_update" => {
                // Handle product update events
                tracing::info!("Product updated: {}", event.event_type());
                Ok(())
            }
            _ => Err(Error::plugin("product_catalog", "Unknown event handler"))
        }
    }
}

/// Search provider implementation for products
#[derive(Debug)]
pub struct ProductSearchProvider {
    data_source: Arc<dyn ProductDataSource>,
    config: PluginConfig,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl SearchProvider for ProductSearchProvider {
    fn provider_id(&self) -> &str {
        "product_catalog_search"
    }

    fn provider_name(&self) -> &str {
        "Product Catalog Search"
    }

    fn description(&self) -> &str {
        "Search products by name, description, and category"
    }

    fn priority(&self) -> i32 {
        200 // Higher priority for product searches
    }

    fn supported_result_types(&self) -> Vec<String> {
        vec!["product".to_string()]
    }

    fn supports_facets(&self) -> bool {
        true
    }

    fn supports_suggestions(&self) -> bool {
        true
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let products = self.data_source.search_products(&query.query, query.limit).await?;

        let mut results = Vec::new();
        for product in products {
            let score = if product.name.to_lowercase().contains(&query.query.to_lowercase()) {
                0.9
            } else if product.description.to_lowercase().contains(&query.query.to_lowercase()) {
                0.7
            } else {
                0.5
            };

            results.push(SearchResult {
                id: product.id.clone(),
                result_type: "product".to_string(),
                title: product.name.clone(),
                description: Some(product.description.clone()),
                score,
                url: Some(format!("/plugins/product_catalog/products/{}", product.id)),
                thumbnail: product.images.first().cloned(),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("price".to_string(), serde_json::json!(product.price));
                    metadata.insert("currency".to_string(), serde_json::json!(product.currency));
                    metadata.insert("category".to_string(), serde_json::json!(product.category));
                    metadata.insert("stock".to_string(), serde_json::json!(product.stock_quantity));
                    metadata
                },
                source_plugin: "product_catalog".to_string(),
                timestamp: product.updated_at,
            });
        }

        Ok(results)
    }

    async fn get_facets(&self, _query: &SearchQuery) -> Result<Vec<SearchFacet>> {
        let categories = self.data_source.get_categories().await?;

        let category_facet = SearchFacet {
            field: "category".to_string(),
            name: "Category".to_string(),
            values: categories.into_iter().map(|cat| FacetValue {
                value: serde_json::Value::String(cat.clone()),
                display_name: cat,
                count: 0, // Would be calculated from actual data
            }).collect(),
        };

        Ok(vec![category_facet])
    }

    async fn get_suggestions(&self, query: &SearchQuery) -> Result<Vec<SearchSuggestion>> {
        // Simple suggestion implementation
        let products = self.data_source.search_products(&query.query, Some(5)).await?;

        let suggestions = products.into_iter().map(|product| SearchSuggestion {
            text: query.query.clone(),
            completion: product.name,
            category: Some("Products".to_string()),
            score: 0.8,
        }).collect();

        Ok(suggestions)
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start = std::time::Instant::now();

        // Test with a simple query
        match self.data_source.get_products(Some(1), None).await {
            Ok(_) => Ok(ProviderHealth {
                is_healthy: true,
                response_time_ms: Some(start.elapsed().as_millis() as u64),
                error_message: None,
                last_check: Time::now(),
            }),
            Err(e) => Ok(ProviderHealth {
                is_healthy: false,
                response_time_ms: Some(start.elapsed().as_millis() as u64),
                error_message: Some(e.to_string()),
                last_check: Time::now(),
            }),
        }
    }
}

// Export the plugin using the framework's macro
qorzen_oxide::export_plugin!(ProductCatalogPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_info() {
        let plugin = ProductCatalogPlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "product_catalog");
        assert_eq!(info.name, "Product Catalog");
        assert!(!info.supported_platforms.is_empty());
    }

    #[test]
    fn test_plugin_permissions() {
        let plugin = ProductCatalogPlugin::new();
        let permissions = plugin.required_permissions();

        assert!(!permissions.is_empty());
        assert!(permissions.iter().any(|p| p.resource == "products" && p.action == "read"));
    }

    #[test]
    fn test_config_defaults() {
        let config = PluginConfig::default();
        assert_eq!(config.use_api, cfg!(target_arch = "wasm32"));
        assert_eq!(config.cache_duration_secs, 300);
        assert!(config.search_enabled);
        assert_eq!(config.max_results, 100);
    }

    #[tokio::test]
    async fn test_api_data_source() {
        let data_source = ApiDataSource::new("https://api.example.com".to_string());

        // This test will fail due to no actual server, but demonstrates the interface
        let result = data_source.get_products(Some(10), None).await;
        assert!(result.is_err()); // Expected to fail without real API
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_database_data_source() {
        let data_source = DatabaseDataSource::new("mock://database".to_string());

        // Test mock implementation
        let products = data_source.get_products(None, None).await.unwrap();
        assert!(!products.is_empty());
        assert_eq!(products[0].id, "prod_001");

        let product = data_source.get_product("prod_001").await.unwrap();
        assert!(product.is_some());

        let missing = data_source.get_product("nonexistent").await.unwrap();
        assert!(missing.is_none());

        let search_results = data_source.search_products("sample", None).await.unwrap();
        assert!(!search_results.is_empty());

        let categories = data_source.get_categories().await.unwrap();
        assert!(categories.contains(&"Electronics".to_string()));
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = ProductCatalogPlugin::new();

        // Test info without initialization
        let info = plugin.info();
        assert_eq!(info.id, "product_catalog");

        // Test UI components
        let components = plugin.ui_components();
        assert!(!components.is_empty());
        assert!(components.iter().any(|c| c.id == "product_list"));

        // Test menu items
        let menu_items = plugin.menu_items();
        assert!(!menu_items.is_empty());
        assert!(menu_items.iter().any(|m| m.id == "products"));

        // Test settings schema
        let schema = plugin.settings_schema();
        assert!(schema.is_some());
    }
}