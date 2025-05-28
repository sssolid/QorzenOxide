use qorzen_oxide::{
    plugin::{Plugin, PluginInfo, PluginContext, PluginDependency},
    manager::{Manager, ManagedState, ManagerStatus},
    error::{Result, Error},
    event::{Event, EventHandler},
    api::{ApiRoute, ApiHandler, HttpMethod},
    ui::{UIComponent, MenuItem},
    types::{Permission, Metadata},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

mod ui;
mod api;
mod models;
mod config;
mod ui;
mod config;
mod models;

use ui::ProductCatalogUI;
use api::ProductApiHandlers;
use models::{Product, Category, Inventory};
use config::ProductCatalogConfig;

pub struct ProductCatalogPlugin {
    state: ManagedState,
    context: Option<PluginContext>,
    config: ProductCatalogConfig,
    ui_components: Vec<UIComponent>,
    api_handlers: ProductApiHandlers,
}

impl ProductCatalogPlugin {
    pub fn new() -> Self {
        Self {
            state: ManagedState::new(
                Uuid::new_v4(),
                "product_catalog_plugin"
            ),
            context: None,
            config: ProductCatalogConfig::default(),
            ui_components: Vec::new(),
            api_handlers: ProductApiHandlers::new(),
        }
    }
}

#[async_trait]
impl Plugin for ProductCatalogPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "com.example.product-catalog".to_string(),
            name: "Product Catalog".to_string(),
            version: "1.0.0".to_string(),
            description: "Product catalog management with inventory tracking".to_string(),
            author: "Example Company".to_string(),
            license: "MIT".to_string(),
            homepage: Some("https://example.com/plugins/product-catalog".to_string()),
            repository: Some("https://github.com/example/product-catalog-plugin".to_string()),
            minimum_core_version: "0.1.0".to_string(),
            supported_platforms: vec![
                Platform::Desktop,
                Platform::Mobile,
                Platform::Wasm,
            ],
        }
    }

    fn required_dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency {
                plugin_id: "com.qorzen.database".to_string(),
                version_requirement: ">=1.0.0".to_string(),
                optional: false,
            }
        ]
    }

    fn required_permissions(&self) -> Vec<Permission> {
        vec![
            Permission {
                resource: "database".to_string(),
                action: "read".to_string(),
                scope: PermissionScope::Global,
            },
            Permission {
                resource: "database".to_string(),
                action: "write".to_string(),
                scope: PermissionScope::Global,
            },
            Permission {
                resource: "api".to_string(),
                action: "expose".to_string(),
                scope: PermissionScope::Global,
            },
            Permission {
                resource: "ui".to_string(),
                action: "menu".to_string(),
                scope: PermissionScope::Global,
            },
        ]
    }

    async fn initialize(&mut self, context: PluginContext) -> Result<()> {
        self.state.set_state(ManagerState::Initializing).await;

        // Store context for later use
        self.context = Some(context);

        // Load plugin configuration
        if let Some(ctx) = &self.context {
            self.config = ctx.config.get_typed::<ProductCatalogConfig>()?;
        }

        // Initialize database tables
        self.initialize_database().await?;

        // Setup UI components
        self.setup_ui_components().await?;

        // Register API handlers
        self.register_api_handlers().await?;

        self.state.set_state(ManagerState::Running).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(ManagerState::ShuttingDown).await;

        // Cleanup resources
        self.api_handlers.shutdown().await?;

        self.state.set_state(ManagerState::Shutdown).await;
        Ok(())
    }

    fn ui_components(&self) -> Vec<UIComponent> {
        vec![
            UIComponent {
                id: "product_catalog_main".to_string(),
                name: "Product Catalog".to_string(),
                component_type: UIComponentType::Page,
                route: "/products".to_string(),
                icon: Some("shopping-bag".to_string()),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                render_fn: Box::new(ProductCatalogUI::render_main_page),
            },
            UIComponent {
                id: "product_catalog_sidebar".to_string(),
                name: "Quick Add Product".to_string(),
                component_type: UIComponentType::SidebarPanel,
                route: "".to_string(),
                icon: Some("plus".to_string()),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "write".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                render_fn: Box::new(ProductCatalogUI::render_quick_add_panel),
            },
        ]
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "products_menu".to_string(),
                label: "Products".to_string(),
                icon: Some("shopping-bag".to_string()),
                route: "/products".to_string(),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                children: vec![
                    MenuItem {
                        id: "products_list".to_string(),
                        label: "All Products".to_string(),
                        icon: Some("list".to_string()),
                        route: "/products/list".to_string(),
                        required_permissions: vec![],
                        children: vec![],
                        order: 1,
                    },
                    MenuItem {
                        id: "products_categories".to_string(),
                        label: "Categories".to_string(),
                        icon: Some("folder".to_string()),
                        route: "/products/categories".to_string(),
                        required_permissions: vec![],
                        children: vec![],
                        order: 2,
                    },
                    MenuItem {
                        id: "products_inventory".to_string(),
                        label: "Inventory".to_string(),
                        icon: Some("warehouse".to_string()),
                        route: "/products/inventory".to_string(),
                        required_permissions: vec![
                            Permission {
                                resource: "inventory".to_string(),
                                action: "read".to_string(),
                                scope: PermissionScope::Global,
                            }
                        ],
                        children: vec![],
                        order: 3,
                    },
                ],
                order: 10,
            },
        ]
    }

    fn api_routes(&self) -> Vec<ApiRoute> {
        vec![
            ApiRoute {
                path: "/api/products".to_string(),
                method: HttpMethod::Get,
                handler: Box::new(self.api_handlers.list_products()),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                rate_limit: Some(RateLimit {
                    requests_per_minute: 100,
                    burst_limit: 10,
                    scope: RateLimitScope::PerUser,
                }),
                documentation: ApiDocumentation {
                    summary: "List all products".to_string(),
                    description: "Retrieve a paginated list of products with optional filtering".to_string(),
                    parameters: vec![
                        ApiParameter {
                            name: "page".to_string(),
                            param_type: "query".to_string(),
                            data_type: "integer".to_string(),
                            required: false,
                            description: "Page number for pagination".to_string(),
                        },
                        ApiParameter {
                            name: "limit".to_string(),
                            param_type: "query".to_string(),
                            data_type: "integer".to_string(),
                            required: false,
                            description: "Number of items per page".to_string(),
                        },
                        ApiParameter {
                            name: "category".to_string(),
                            param_type: "query".to_string(),
                            data_type: "string".to_string(),
                            required: false,
                            description: "Filter by category ID".to_string(),
                        },
                    ],
                    responses: vec![
                        ApiResponse {
                            status_code: 200,
                            description: "Successfully retrieved products".to_string(),
                            schema: Some("ProductListResponse".to_string()),
                        },
                        ApiResponse {
                            status_code: 403,
                            description: "Insufficient permissions".to_string(),
                            schema: None,
                        },
                    ],
                },
            },
            ApiRoute {
                path: "/api/products/{id}".to_string(),
                method: HttpMethod::Get,
                handler: Box::new(self.api_handlers.get_product()),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "read".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                rate_limit: Some(RateLimit {
                    requests_per_minute: 200,
                    burst_limit: 20,
                    scope: RateLimitScope::PerUser,
                }),
                documentation: ApiDocumentation {
                    summary: "Get product by ID".to_string(),
                    description: "Retrieve detailed information about a specific product".to_string(),
                    parameters: vec![
                        ApiParameter {
                            name: "id".to_string(),
                            param_type: "path".to_string(),
                            data_type: "string".to_string(),
                            required: true,
                            description: "Product UUID".to_string(),
                        },
                    ],
                    responses: vec![
                        ApiResponse {
                            status_code: 200,
                            description: "Product found".to_string(),
                            schema: Some("Product".to_string()),
                        },
                        ApiResponse {
                            status_code: 404,
                            description: "Product not found".to_string(),
                            schema: None,
                        },
                    ],
                },
            },
            ApiRoute {
                path: "/api/products".to_string(),
                method: HttpMethod::Post,
                handler: Box::new(self.api_handlers.create_product()),
                required_permissions: vec![
                    Permission {
                        resource: "products".to_string(),
                        action: "write".to_string(),
                        scope: PermissionScope::Global,
                    }
                ],
                rate_limit: Some(RateLimit {
                    requests_per_minute: 20,
                    burst_limit: 5,
                    scope: RateLimitScope::PerUser,
                }),
                documentation: ApiDocumentation {
                    summary: "Create new product".to_string(),
                    description: "Create a new product in the catalog".to_string(),
                    parameters: vec![],
                    responses: vec![
                        ApiResponse {
                            status_code: 201,
                            description: "Product created successfully".to_string(),
                            schema: Some("Product".to_string()),
                        },
                        ApiResponse {
                            status_code: 400,
                            description: "Invalid product data".to_string(),
                            schema: Some("ValidationError".to_string()),
                        },
                    ],
                },
            },
        ]
    }

    fn event_handlers(&self) -> Vec<Box<dyn EventHandler>> {
        vec![
            Box::new(ProductInventoryUpdateHandler::new()),
            Box::new(ProductPriceChangeHandler::new()),
        ]
    }

    fn settings_schema(&self) -> Option<SettingsSchema> {
        Some(SettingsSchema {
            category: "Product Catalog".to_string(),
            subcategory: None,
            settings: vec![
                SettingDefinition {
                    key: "default_currency".to_string(),
                    display_name: "Default Currency".to_string(),
                    description: "Default currency for product prices".to_string(),
                    setting_type: SettingType::Enum {
                        options: vec!["USD".to_string(), "EUR".to_string(), "GBP".to_string()],
                    },
                    default_value: serde_json::Value::String("USD".to_string()),
                    validation_rules: vec![],
                    requires_restart: false,
                    is_sensitive: false,
                },
                SettingDefinition {
                    key: "enable_inventory_tracking".to_string(),
                    display_name: "Enable Inventory Tracking".to_string(),
                    description: "Track product inventory levels".to_string(),
                    setting_type: SettingType::Boolean,
                    default_value: serde_json::Value::Bool(true),
                    validation_rules: vec![],
                    requires_restart: false,
                    is_sensitive: false,
                },
                SettingDefinition {
                    key: "low_stock_threshold".to_string(),
                    display_name: "Low Stock Threshold".to_string(),
                    description: "Alert when inventory falls below this level".to_string(),
                    setting_type: SettingType::Integer { min: Some(0), max: Some(1000) },
                    default_value: serde_json::Value::Number(10.into()),
                    validation_rules: vec![],
                    requires_restart: false,
                    is_sensitive: false,
                },
            ],
            required_permissions: vec![
                Permission {
                    resource: "products".to_string(),
                    action: "configure".to_string(),
                    scope: PermissionScope::Global,
                }
            ],
            ui_hints: UIHints {
                icon: Some("settings".to_string()),
                color: None,
                order: 100,
            },
        })
    }
}

#[async_trait]
impl Manager for ProductCatalogPlugin {
    fn name(&self) -> &str {
        "product_catalog_plugin"
    }

    fn id(&self) -> Uuid {
        self.state.id()
    }

    async fn initialize(&mut self) -> Result<()> {
        // Plugin initialization is handled by the Plugin trait
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Plugin shutdown is handled by the Plugin trait
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        // Add plugin-specific status information
        if let Some(ctx) = &self.context {
            if let Ok(product_count) = self.get_product_count().await {
                status.add_metadata("product_count", serde_json::Value::from(product_count));
            }

            if let Ok(category_count) = self.get_category_count().await {
                status.add_metadata("category_count", serde_json::Value::from(category_count));
            }
        }

        status
    }
}

impl ProductCatalogPlugin {
    async fn initialize_database(&self) -> Result<()> {
        if let Some(ctx) = &self.context {
            let db = ctx.database.as_ref().ok_or_else(|| {
                Error::plugin("com.example.product-catalog", "Database dependency not available")
            })?;

            // Run database migrations
            let migrations = vec![
                Migration {
                    version: 1,
                    description: "Create products table".to_string(),
                    sql: include_str!("../migrations/001_create_products.sql").to_string(),
                },
                Migration {
                    version: 2,
                    description: "Create categories table".to_string(),
                    sql: include_str!("../migrations/002_create_categories.sql").to_string(),
                },
                Migration {
                    version: 3,
                    description: "Create inventory table".to_string(),
                    sql: include_str!("../migrations/003_create_inventory.sql").to_string(),
                },
            ];

            db.migrate(&migrations).await?;
        }

        Ok(())
    }

    async fn setup_ui_components(&mut self) -> Result<()> {
        // UI components are defined in the ui_components() method
        Ok(())
    }

    async fn register_api_handlers(&mut self) -> Result<()> {
        // API handlers are defined in the api_routes() method
        Ok(())
    }

    async fn get_product_count(&self) -> Result<u64> {
        if let Some(ctx) = &self.context {
            if let Some(db) = &ctx.database {
                let result = db.query("SELECT COUNT(*) as count FROM products", &[]).await?;
                if let Some(row) = result.first() {
                    return Ok(row.get::<u64>("count"));
                }
            }
        }
        Ok(0)
    }

    async fn get_category_count(&self) -> Result<u64> {
        if let Some(ctx) = &self.context {
            if let Some(db) = &ctx.database {
                let result = db.query("SELECT COUNT(*) as count FROM categories", &[]).await?;
                if let Some(row) = result.first() {
                    return Ok(row.get::<u64>("count"));
                }
            }
        }
        Ok(0)
    }
}

// Event handlers
struct ProductInventoryUpdateHandler;

impl ProductInventoryUpdateHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventHandler for ProductInventoryUpdateHandler {
    async fn handle(&self, event: &dyn Event) -> Result<()> {
        if event.event_type() == "inventory.updated" {
            // Handle inventory update logic
            tracing::info!("Processing inventory update event");
            // Update low stock alerts, reorder notifications, etc.
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "product_inventory_update_handler"
    }

    fn event_types(&self) -> Vec<&'static str> {
        vec!["inventory.updated", "inventory.low_stock"]
    }
}

struct ProductPriceChangeHandler;

impl ProductPriceChangeHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventHandler for ProductPriceChangeHandler {
    async fn handle(&self, event: &dyn Event) -> Result<()> {
        if event.event_type() == "product.price_changed" {
            // Handle price change logic
            tracing::info!("Processing product price change event");
            // Update pricing tiers, notify customers, etc.
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "product_price_change_handler"
    }

    fn event_types(&self) -> Vec<&'static str> {
        vec!["product.price_changed", "product.sale_started", "product.sale_ended"]
    }
}