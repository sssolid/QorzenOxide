# Qorzen Oxide: Comprehensive Plugin Design & Implementation Guide

## Table of Contents
1. [Plugin Development Guide](#plugin-development-guide)
2. [Example Plugin Implementation](#example-plugin-implementation)

---

## Plugin Development Guide

### Plugin Architecture

Plugins in Qorzen Oxide are self-contained modules that extend the application's functionality. They follow a standardized interface and lifecycle, making them safe and predictable.

### Plugin Structure

```
my-plugin/
├── Cargo.toml
├── plugin.json          # Plugin metadata
├── src/
│   ├── lib.rs          # Main plugin implementation
│   ├── ui/             # Dioxus UI components
│   │   ├── mod.rs
│   │   └── components.rs
│   ├── api/            # API endpoints
│   │   ├── mod.rs
│   │   └── handlers.rs
│   ├── config/         # Configuration schema
│   │   ├── mod.rs
│   │   └── schema.rs
│   └── models/         # Data models
│       ├── mod.rs
│       └── entities.rs
├── migrations/         # Database migrations
│   └── 001_initial.sql
├── assets/            # Static assets
│   ├── icons/
│   └── styles/
└── docs/              # Plugin documentation
    └── README.md
```

### Plugin Metadata (plugin.json)

```json
{
  "id": "com.company.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "A sample plugin for demonstration",
  "author": "Your Name",
  "license": "MIT",
  "homepage": "https://github.com/yourname/my-plugin",
  "minimum_core_version": "0.1.0",
  "supported_platforms": ["desktop", "mobile", "wasm"],
  "dependencies": [
    {
      "plugin_id": "com.qorzen.database",
      "version": ">=1.0.0",
      "optional": false
    }
  ],
  "permissions": [
    "database.read",
    "database.write",
    "api.expose",
    "ui.menu",
    "ui.sidebar"
  ],
  "configuration_schema": "./config/schema.json",
  "database_migrations": "./migrations/",
  "assets": "./assets/"
}
```

### Plugin Implementation Steps

1. **Define Plugin Structure**
2. **Implement Core Trait**
3. **Create UI Components**
4. **Define API Endpoints**
5. **Setup Configuration**
6. **Add Database Migrations**
7. **Test and Package**

### Plugin Development Workflow

1. **Setup Development Environment**
   ```bash
   cargo new --lib my-plugin
   cd my-plugin
   # Add qorzen-plugin-sdk dependency
   ```

2. **Implement Plugin Trait**
3. **Develop UI Components with Dioxus**
4. **Create API Endpoints**
5. **Test with Qorzen Oxide**
6. **Package and Distribute**

---

## Example Plugin Implementation

Let's create a complete example: a "Product Catalog" plugin for an e-commerce application.

### Plugin Metadata

```json
{
  "id": "com.example.product-catalog",
  "name": "Product Catalog",
  "version": "1.0.0",
  "description": "Product catalog management with inventory tracking",
  "author": "Example Company",
  "license": "MIT",
  "minimum_core_version": "0.1.0",
  "supported_platforms": ["desktop", "mobile", "wasm"],
  "dependencies": [
    {
      "plugin_id": "com.qorzen.database",
      "version": ">=1.0.0",
      "optional": false
    }
  ],
  "permissions": [
    "database.read",
    "database.write",
    "api.expose",
    "ui.menu",
    "ui.sidebar",
    "file.upload"
  ]
}
```

### Main Plugin Implementation

```rust
// src/lib.rs
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
```

### UI Components

```rust
// src/ui/mod.rs
use dioxus::prelude::*;
use qorzen_oxide::{
    ui::{UIComponent, UIComponentType},
    types::Permission,
};

pub struct ProductCatalogUI;

impl ProductCatalogUI {
    pub fn render_main_page(cx: Scope) -> Element {
        let products = use_state(cx, || Vec::<Product>::new());
        let loading = use_state(cx, || true);
        let search_term = use_state(cx, String::new);
        let selected_category = use_state(cx, || None::<String>);
        
        // Load products on component mount
        use_effect(cx, (), |_| {
            to_owned![products, loading];
            async move {
                if let Ok(product_list) = fetch_products().await {
                    products.set(product_list);
                    loading.set(false);
                }
            }
        });
        
        render! {
            div { class: "product-catalog-page",
                div { class: "page-header",
                    h1 { "Product Catalog" }
                    div { class: "header-actions",
                        button { 
                            class: "btn btn-primary",
                            onclick: |_| {
                                // Handle add product
                            },
                            "Add Product"
                        }
                        button { 
                            class: "btn btn-secondary",
                            onclick: |_| {
                                // Handle import products
                            },
                            "Import Products"
                        }
                    }
                }
                
                div { class: "product-filters",
                    div { class: "search-bar",
                        input {
                            r#type: "text",
                            placeholder: "Search products...",
                            value: "{search_term}",
                            oninput: move |evt| search_term.set(evt.value.clone()),
                        }
                    }
                    
                    div { class: "category-filter",
                        select {
                            value: "{selected_category:?}",
                            onchange: move |evt| {
                                selected_category.set(
                                    if evt.value.is_empty() { 
                                        None 
                                    } else { 
                                        Some(evt.value.clone()) 
                                    }
                                );
                            },
                            option { value: "", "All Categories" }
                            // Render category options dynamically
                        }
                    }
                }
                
                if **loading {
                    div { class: "loading-spinner",
                        "Loading products..."
                    }
                } else {
                    div { class: "product-grid",
                        products.iter().map(|product| rsx! {
                            ProductCard { 
                                key: "{product.id}",
                                product: product.clone(),
                                on_edit: |product_id| {
                                    // Handle edit product
                                },
                                on_delete: |product_id| {
                                    // Handle delete product
                                }
                            }
                        })
                    }
                }
                
                // Pagination component
                div { class: "pagination",
                    // Pagination controls
                }
            }
        }
    }
    
    pub fn render_quick_add_panel(cx: Scope) -> Element {
        let product_name = use_state(cx, String::new);
        let product_price = use_state(cx, String::new);
        let product_category = use_state(cx, String::new);
        let is_submitting = use_state(cx, || false);
        
        render! {
            div { class: "quick-add-panel",
                h3 { "Quick Add Product" }
                
                form {
                    onsubmit: move |evt| {
                        evt.prevent_default();
                        if !**is_submitting {
                            is_submitting.set(true);
                            // Handle form submission
                        }
                    },
                    
                    div { class: "form-group",
                        label { "Product Name" }
                        input {
                            r#type: "text",
                            value: "{product_name}",
                            oninput: move |evt| product_name.set(evt.value.clone()),
                            required: true,
                        }
                    }
                    
                    div { class: "form-group",
                        label { "Price" }
                        input {
                            r#type: "number",
                            step: "0.01",
                            value: "{product_price}",
                            oninput: move |evt| product_price.set(evt.value.clone()),
                            required: true,
                        }
                    }
                    
                    div { class: "form-group",
                        label { "Category" }
                        select {
                            value: "{product_category}",
                            onchange: move |evt| product_category.set(evt.value.clone()),
                            required: true,
                            // Category options
                        }
                    }
                    
                    div { class: "form-actions",
                        button {
                            r#type: "submit",
                            class: "btn btn-primary",
                            disabled: **is_submitting,
                            if **is_submitting { "Adding..." } else { "Add Product" }
                        }
                        button {
                            r#type: "button",
                            class: "btn btn-secondary",
                            onclick: |_| {
                                // Reset form
                            },
                            "Reset"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProductCard(cx: Scope, product: Product, on_edit: EventHandler<String>, on_delete: EventHandler<String>) -> Element {
    render! {
        div { class: "product-card",
            div { class: "product-image",
                if let Some(image_url) = &product.image_url {
                    img { src: "{image_url}", alt: "{product.name}" }
                } else {
                    div { class: "no-image", "No Image" }
                }
            }
            
            div { class: "product-info",
                h3 { class: "product-name", "{product.name}" }
                p { class: "product-description", "{product.description}" }
                div { class: "product-price", "${product.price}" }
                
                if let Some(category) = &product.category {
                    div { class: "product-category", 
                        span { class: "category-badge", "{category.name}" }
                    }
                }
            }
            
            div { class: "product-actions",
                button {
                    class: "btn btn-sm btn-primary",
                    onclick: move |_| on_edit.call(product.id.clone()),
                    "Edit"
                }
                button {
                    class: "btn btn-sm btn-danger",
                    onclick: move |_| on_delete.call(product.id.clone()),
                    "Delete"
                }
            }
            
            if product.inventory.quantity <= product.inventory.low_stock_threshold {
                div { class: "stock-warning",
                    "⚠️ Low Stock: {product.inventory.quantity} remaining"
                }
            }
        }
    }
}

async fn fetch_products() -> Result<Vec<Product>, Error> {
    // Implementation to fetch products from API
    // This would use the plugin's API client
    todo!()
}
```

### API Handlers

```rust
// src/api/handlers.rs
use qorzen_oxide::{
    api::{ApiHandler, ApiRequest, ApiResponse, HttpMethod},
    error::{Result, Error},
    types::Permission,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct ProductApiHandlers {
    // Database connection or service layer
}

impl ProductApiHandlers {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn list_products(&self) -> ListProductsHandler {
        ListProductsHandler::new()
    }
    
    pub fn get_product(&self) -> GetProductHandler {
        GetProductHandler::new()
    }
    
    pub fn create_product(&self) -> CreateProductHandler {
        CreateProductHandler::new()
    }
    
    pub async fn shutdown(&self) -> Result<()> {
        // Cleanup resources
        Ok(())
    }
}

pub struct ListProductsHandler;

impl ListProductsHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiHandler for ListProductsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse> {
        // Extract query parameters
        let page = request.query_params.get("page")
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(1);
        let limit = request.query_params.get("limit")
            .and_then(|l| l.parse::<u32>().ok())
            .unwrap_or(20);
        let category_filter = request.query_params.get("category");
        
        // Validate permissions
        if let Some(user) = &request.user {
            if !user.has_permission("products", "read") {
                return Ok(ApiResponse {
                    status_code: 403,
                    headers: HashMap::new(),
                    body: Some(b"Insufficient permissions".to_vec()),
                    content_type: "text/plain".to_string(),
                });
            }
        } else {
            return Ok(ApiResponse {
                status_code: 401,
                headers: HashMap::new(),
                body: Some(b"Authentication required".to_vec()),
                content_type: "text/plain".to_string(),
            });
        }
        
        // Fetch products from database
        let products = fetch_products_from_db(page, limit, category_filter).await?;
        let total_count = get_products_total_count(category_filter).await?;
        
        let response_data = ProductListResponse {
            products,
            pagination: PaginationInfo {
                page,
                limit,
                total_count,
                total_pages: (total_count + limit - 1) / limit,
            },
        };
        
        let json_body = serde_json::to_vec(&response_data).map_err(|e| {
            Error::api("serialization", format!("Failed to serialize response: {}", e))
        })?;
        
        Ok(ApiResponse {
            status_code: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            },
            body: Some(json_body),
            content_type: "application/json".to_string(),
        })
    }
}

pub struct GetProductHandler;

impl GetProductHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiHandler for GetProductHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse> {
        // Extract product ID from path
        let product_id = extract_path_param(&request.path, "id")
            .ok_or_else(|| Error::api("validation", "Product ID not provided"))?;
        
        // Validate permissions
        if let Some(user) = &request.user {
            if !user.has_permission("products", "read") {
                return Ok(ApiResponse {
                    status_code: 403,
                    headers: HashMap::new(),
                    body: Some(b"Insufficient permissions".to_vec()),
                    content_type: "text/plain".to_string(),
                });
            }
        } else {
            return Ok(ApiResponse {
                status_code: 401,
                headers: HashMap::new(),
                body: Some(b"Authentication required".to_vec()),
                content_type: "text/plain".to_string(),
            });
        }
        
        // Fetch product from database
        match fetch_product_by_id(&product_id).await {
            Ok(Some(product)) => {
                let json_body = serde_json::to_vec(&product).map_err(|e| {
                    Error::api("serialization", format!("Failed to serialize product: {}", e))
                })?;
                
                Ok(ApiResponse {
                    status_code: 200,
                    headers: {
                        let mut headers = HashMap::new();
                        headers.insert("Content-Type".to_string(), "application/json".to_string());
                        headers
                    },
                    body: Some(json_body),
                    content_type: "application/json".to_string(),
                })
            }
            Ok(None) => {
                Ok(ApiResponse {
                    status_code: 404,
                    headers: HashMap::new(),
                    body: Some(b"Product not found".to_vec()),
                    content_type: "text/plain".to_string(),
                })
            }
            Err(e) => {
                tracing::error!("Failed to fetch product {}: {}", product_id, e);
                Ok(ApiResponse {
                    status_code: 500,
                    headers: HashMap::new(),
                    body: Some(b"Internal server error".to_vec()),
                    content_type: "text/plain".to_string(),
                })
            }
        }
    }
}

pub struct CreateProductHandler;

impl CreateProductHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiHandler for CreateProductHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse> {
        // Validate permissions
        if let Some(user) = &request.user {
            if !user.has_permission("products", "write") {
                return Ok(ApiResponse {
                    status_code: 403,
                    headers: HashMap::new(),
                    body: Some(b"Insufficient permissions".to_vec()),
                    content_type: "text/plain".to_string(),
                });
            }
        } else {
            return Ok(ApiResponse {
                status_code: 401,
                headers: HashMap::new(),
                body: Some(b"Authentication required".to_vec()),
                content_type: "text/plain".to_string(),
            });
        }
        
        // Parse request body
        let body = request.body.ok_or_else(|| {
            Error::api("validation", "Request body is required")
        })?;
        
        let create_request: CreateProductRequest = serde_json::from_slice(&body)
            .map_err(|e| Error::api("validation", format!("Invalid JSON: {}", e)))?;
        
        // Validate product data
        let validation_errors = validate_product_data(&create_request);
        if !validation_errors.is_empty() {
            let error_response = ValidationErrorResponse {
                message: "Validation failed".to_string(),
                errors: validation_errors,
            };
            
            let json_body = serde_json::to_vec(&error_response).map_err(|e| {
                Error::api("serialization", format!("Failed to serialize error response: {}", e))
            })?;
            
            return Ok(ApiResponse {
                status_code: 400,
                headers: {
                    let mut headers = HashMap::new();
                    headers.insert("Content-Type".to_string(), "application/json".to_string());
                    headers
                },
                body: Some(json_body),
                content_type: "application/json".to_string(),
            });
        }
        
        // Create product in database
        let new_product = create_product_in_db(create_request).await?;
        
        let json_body = serde_json::to_vec(&new_product).map_err(|e| {
            Error::api("serialization", format!("Failed to serialize product: {}", e))
        })?;
        
        Ok(ApiResponse {
            status_code: 201,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers.insert("Location".to_string(), format!("/api/products/{}", new_product.id));
                headers
            },
            body: Some(json_body),
            content_type: "application/json".to_string(),
        })
    }
}

// Helper functions and types

#[derive(Debug, Serialize, Deserialize)]
struct ProductListResponse {
    products: Vec<Product>,
    pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaginationInfo {
    page: u32,
    limit: u32,
    total_count: u32,
    total_pages: u32,
}

#[derive(Debug, Deserialize)]
struct CreateProductRequest {
    name: String,
    description: String,
    price: f64,
    category_id: Option<String>,
    sku: Option<String>,
    barcode: Option<String>,
    initial_quantity: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ValidationErrorResponse {
    message: String,
    errors: Vec<ValidationError>,
}

#[derive(Debug, Serialize)]
struct ValidationError {
    field: String,
    message: String,
}

fn extract_path_param(path: &str, param_name: &str) -> Option<String> {
    // Simple path parameter extraction
    // In a real implementation, you'd use a proper router
    if let Some(id_part) = path.split('/').last() {
        Some(id_part.to_string())
    } else {
        None
    }
}

async fn fetch_products_from_db(
    page: u32,
    limit: u32,
    category_filter: Option<&String>,
) -> Result<Vec<Product>> {
    // Database query implementation
    // This would use the plugin's database context
    todo!()
}

async fn get_products_total_count(category_filter: Option<&String>) -> Result<u32> {
    // Database count query implementation
    todo!()
}

async fn fetch_product_by_id(product_id: &str) -> Result<Option<Product>> {
    // Database query implementation
    todo!()
}

async fn create_product_in_db(request: CreateProductRequest) -> Result<Product> {
    // Database insert implementation
    todo!()
}

fn validate_product_data(request: &CreateProductRequest) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    
    if request.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name".to_string(),
            message: "Product name is required".to_string(),
        });
    }
    
    if request.name.len() > 255 {
        errors.push(ValidationError {
            field: "name".to_string(),
            message: "Product name must be 255 characters or less".to_string(),
        });
    }
    
    if request.price < 0.0 {
        errors.push(ValidationError {
            field: "price".to_string(),
            message: "Price must be non-negative".to_string(),
        });
    }
    
    if let Some(sku) = &request.sku {
        if sku.trim().is_empty() {
            errors.push(ValidationError {
                field: "sku".to_string(),
                message: "SKU cannot be empty if provided".to_string(),
            });
        }
    }
    
    errors
}
```

### Database Migrations

```sql
-- migrations/001_create_products.sql
CREATE TABLE IF NOT EXISTS products (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    price DECIMAL(10,2) NOT NULL,
    sku TEXT UNIQUE,
    barcode TEXT UNIQUE,
    category_id TEXT,
    image_url TEXT,
    weight DECIMAL(10,3),
    dimensions_length DECIMAL(10,2),
    dimensions_width DECIMAL(10,2),
    dimensions_height DECIMAL(10,2),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT,
    updated_by TEXT,
    
    FOREIGN KEY (category_id) REFERENCES categories(id)
);

CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_sku ON products(sku);
CREATE INDEX idx_products_active ON products(is_active);
```

```sql
-- migrations/002_create_categories.sql
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    parent_id TEXT,
    sort_order INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (parent_id) REFERENCES categories(id)
);

CREATE INDEX idx_categories_parent ON categories(parent_id);
CREATE INDEX idx_categories_active ON categories(is_active);
```

```sql
-- migrations/003_create_inventory.sql
CREATE TABLE IF NOT EXISTS inventory (
    product_id TEXT PRIMARY KEY,
    quantity INTEGER NOT NULL DEFAULT 0,
    reserved_quantity INTEGER NOT NULL DEFAULT 0,
    low_stock_threshold INTEGER DEFAULT 10,
    reorder_point INTEGER DEFAULT 5,
    reorder_quantity INTEGER DEFAULT 50,
    cost DECIMAL(10,2),
    last_restocked_at TIMESTAMP,
    last_sold_at TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

CREATE INDEX idx_inventory_low_stock ON inventory(quantity, low_stock_threshold);
```

### Plugin Configuration

```rust
// src/config/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCatalogConfig {
    pub default_currency: String,
    pub enable_inventory_tracking: bool,
    pub low_stock_threshold: u32,
    pub enable_barcode_scanning: bool,
    pub image_upload_max_size_mb: u32,
    pub supported_image_formats: Vec<String>,
    pub enable_product_reviews: bool,
    pub enable_product_variants: bool,
    pub tax_calculation_mode: TaxCalculationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaxCalculationMode {
    Inclusive,
    Exclusive,
    None,
}

impl Default for ProductCatalogConfig {
    fn default() -> Self {
        Self {
            default_currency: "USD".to_string(),
            enable_inventory_tracking: true,
            low_stock_threshold: 10,
            enable_barcode_scanning: false,
            image_upload_max_size_mb: 5,
            supported_image_formats: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/webp".to_string(),
            ],
            enable_product_reviews: false,
            enable_product_variants: false,
            tax_calculation_mode: TaxCalculationMode::Exclusive,
        }
    }
}
```

### Data Models

```rust
// src/models/mod.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub category: Option<Category>,
    pub image_url: Option<String>,
    pub weight: Option<f64>,
    pub dimensions: Option<Dimensions>,
    pub inventory: Inventory,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub product_id: String,
    pub quantity: u32,
    pub reserved_quantity: u32,
    pub low_stock_threshold: u32,
    pub reorder_point: u32,
    pub reorder_quantity: u32,
    pub cost: Option<f64>,
    pub last_restocked_at: Option<DateTime<Utc>>,
    pub last_sold_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub unit: String, // "cm", "in", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub name: String,
    pub sku: Option<String>,
    pub price_adjustment: f64, // Amount to add/subtract from base price
    pub weight_adjustment: Option<f64>,
    pub attributes: std::collections::HashMap<String, String>, // color: "red", size: "large"
    pub inventory: Inventory,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductReview {
    pub id: String,
    pub product_id: String,
    pub user_id: String,
    pub rating: u8, // 1-5 stars
    pub title: String,
    pub comment: String,
    pub is_verified_purchase: bool,
    pub is_approved: bool,
    pub helpful_votes: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Plugin Packaging

```toml
# Cargo.toml
[package]
name = "product-catalog-plugin"
version = "1.0.0"
edition = "2021"
authors = ["Example Company <dev@example.com>"]
description = "Product catalog management plugin for Qorzen Oxide"
license = "MIT"
repository = "https://github.com/example/product-catalog-plugin"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
qorzen-core = { path = "../qorzen-core" }
qorzen-plugin-sdk = { path = "../qorzen-plugin-sdk" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
dioxus = "0.4"
tracing = "0.1"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = "0.3"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
sqlx = { version = "0.7", features = ["sqlite", "chrono", "uuid"] }
```

---

## Plugin Loading and Integration Process

### How Plugins are Discovered and Loaded

#### 1. Plugin Discovery Process

```rust
pub struct PluginDiscovery {
    plugin_directories: Vec<PathBuf>,
    plugin_registry: PluginRegistry,
    security_validator: SecurityValidator,
}

impl PluginDiscovery {
    pub async fn discover_plugins(&self) -> Result<Vec<PluginManifest>> {
        let mut discovered_plugins = Vec::new();
        
        for directory in &self.plugin_directories {
            match self.scan_directory(directory).await {
                Ok(mut plugins) => discovered_plugins.append(&mut plugins),
                Err(e) => tracing::warn!("Failed to scan plugin directory {}: {}", directory.display(), e),
            }
        }
        
        // Validate and filter plugins
        let mut valid_plugins = Vec::new();
        for plugin in discovered_plugins {
            match self.validate_plugin(&plugin).await {
                Ok(()) => valid_plugins.push(plugin),
                Err(e) => tracing::warn!("Plugin validation failed for {}: {}", plugin.id, e),
            }
        }
        
        Ok(valid_plugins)
    }
    
    async fn scan_directory(&self, directory: &Path) -> Result<Vec<PluginManifest>> {
        let mut plugins = Vec::new();
        let mut entries = fs::read_dir(directory).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                // Look for plugin.json in subdirectory
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    match self.load_manifest(&manifest_path).await {
                        Ok(manifest) => plugins.push(manifest),
                        Err(e) => tracing::warn!("Failed to load manifest from {}: {}", manifest_path.display(), e),
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("plugin") {
                // Plugin archive file
                match self.extract_and_load_archive(&path).await {
                    Ok(manifest) => plugins.push(manifest),
                    Err(e) => tracing::warn!("Failed to load plugin archive {}: {}", path.display(), e),
                }
            }
        }
        
        Ok(plugins)
    }
}
```

#### 2. Plugin Loading Pipeline

```rust
pub struct PluginLoadingPipeline {
    stages: Vec<Box<dyn PluginLoadingStage>>,
}

#[async_trait]
pub trait PluginLoadingStage: Send + Sync {
    async fn process(&self, context: &mut PluginLoadingContext) -> Result<()>;
    fn stage_name(&self) -> &str;
}

pub struct PluginLoadingContext {
    pub manifest: PluginManifest,
    pub plugin_path: PathBuf,
    pub security_context: SecurityContext,
    pub dependencies: Vec<String>,
    pub loaded_plugin: Option<Box<dyn Plugin>>,
}

// Loading stages
pub struct SecurityValidationStage;
pub struct DependencyResolutionStage;
pub struct ConfigurationLoadingStage;
pub struct DatabaseMigrationStage;
pub struct PluginInstantiationStage;
pub struct UIRegistrationStage;
pub struct APIRegistrationStage;

impl PluginLoadingPipeline {
    pub fn new() -> Self {
        Self {
            stages: vec![
                Box::new(SecurityValidationStage),
                Box::new(DependencyResolutionStage),
                Box::new(ConfigurationLoadingStage),
                Box::new(DatabaseMigrationStage),
                Box::new(PluginInstantiationStage),
                Box::new(UIRegistrationStage),
                Box::new(APIRegistrationStage),
            ],
        }
    }
    
    pub async fn load_plugin(&self, manifest: PluginManifest, plugin_path: PathBuf) -> Result<Box<dyn Plugin>> {
        let mut context = PluginLoadingContext {
            manifest,
            plugin_path,
            security_context: SecurityContext::new(),
            dependencies: Vec::new(),
            loaded_plugin: None,
        };
        
        for stage in &self.stages {
            tracing::debug!("Processing plugin loading stage: {}", stage.stage_name());
            stage.process(&mut context).await.with_context(|| {
                format!("Plugin loading failed at stage: {}", stage.stage_name())
            })?;
        }
        
        context.loaded_plugin.ok_or_else(|| {
            Error::plugin(&context.manifest.id, "Plugin instantiation failed")
        })
    }
}
```

#### 3. Runtime Plugin Management

```rust
pub struct RuntimePluginManager {
    loaded_plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    plugin_lifecycle: PluginLifecycleManager,
    hot_reload_watcher: Option<HotReloadWatcher>,
    dependency_graph: DependencyGraph,
}

#[derive(Debug)]
pub struct LoadedPlugin {
    pub plugin: Box<dyn Plugin>,
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub load_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub resource_usage: ResourceUsage,
    pub health_status: PluginHealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    Loading,
    Active,
    Paused,
    Error,
    Unloading,
    Unloaded,
}

impl RuntimePluginManager {
    pub async fn load_plugin(&self, plugin_id: &str) -> Result<()> {
        // Load plugin through pipeline
        let plugin = self.plugin_lifecycle.load_plugin(plugin_id).await?;
        
        // Add to loaded plugins registry
        let loaded_plugin = LoadedPlugin {
            plugin,
            manifest: self.get_manifest(plugin_id)?,
            state: PluginState::Active,
            load_time: Utc::now(),
            last_activity: Utc::now(),
            resource_usage: ResourceUsage::default(),
            health_status: PluginHealthStatus::Healthy,
        };
        
        self.loaded_plugins.write().await.insert(plugin_id.to_string(), loaded_plugin);
        
        // Register plugin components
        self.register_plugin_components(plugin_id).await?;
        
        // Publish plugin loaded event
        self.publish_plugin_event(PluginEvent::Loaded { plugin_id: plugin_id.to_string() }).await?;
        
        Ok(())
    }
    
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        // Check if plugin can be safely unloaded
        if self.has_dependent_plugins(plugin_id).await? {
            return Err(Error::plugin(plugin_id, "Cannot unload plugin with active dependents"));
        }
        
        // Gracefully shutdown plugin
        if let Some(mut loaded_plugin) = self.loaded_plugins.write().await.remove(plugin_id) {
            loaded_plugin.state = PluginState::Unloading;
            
            // Unregister components
            self.unregister_plugin_components(plugin_id).await?;
            
            // Shutdown plugin
            loaded_plugin.plugin.shutdown().await?;
            
            loaded_plugin.state = PluginState::Unloaded;
            
            // Publish plugin unloaded event
            self.publish_plugin_event(PluginEvent::Unloaded { plugin_id: plugin_id.to_string() }).await?;
        }
        
        Ok(())
    }
    
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<()> {
        // Hot reload support
        self.unload_plugin(plugin_id).await?;
        tokio::time::sleep(Duration::from_millis(100)).await; // Brief pause
        self.load_plugin(plugin_id).await?;
        Ok(())
    }
    
    async fn register_plugin_components(&self, plugin_id: &str) -> Result<()> {
        let plugins = self.loaded_plugins.read().await;
        if let Some(loaded_plugin) = plugins.get(plugin_id) {
            // Register UI components
            for ui_component in loaded_plugin.plugin.ui_components() {
                self.ui_manager.register_component(ui_component).await?;
            }
            
            // Register API routes
            for api_route in loaded_plugin.plugin.api_routes() {
                self.api_manager.register_route(plugin_id, api_route).await?;
            }
            
            // Register event handlers
            for event_handler in loaded_plugin.plugin.event_handlers() {
                self.event_manager.register_handler(event_handler).await?;
            }
            
            // Register menu items
            for menu_item in loaded_plugin.plugin.menu_items() {
                self.ui_manager.register_menu_item(menu_item).await?;
            }
        }
        
        Ok(())
    }
}
```

### How End Users Interact with Plugins

#### 1. User Interface Integration

**Menu Integration Example:**
When the Product Catalog plugin is loaded, users will see:

- **Main Menu**: A "Products" menu item appears in the navigation bar
- **Sidebar**: Quick actions panel for adding products appears in the sidebar
- **Pages**: New routes like `/products`, `/products/categories` become available
- **Widgets**: Dashboard widgets showing product statistics appear on relevant pages

**Real-Time Integration:**
```rust
// The UI automatically updates when plugins are loaded/unloaded
#[component]
fn MainNavigation(cx: Scope) -> Element {
    let loaded_plugins = use_shared_state::<LoadedPlugins>(cx)?;
    let current_user = use_shared_state::<User>(cx)?;
    
    // Generate menu items from loaded plugins
    let menu_items = loaded_plugins.read()
        .iter()
        .flat_map(|(_, plugin)| plugin.menu_items())
        .filter(|item| current_user.read().has_permissions(&item.required_permissions))
        .collect::<Vec<_>>();
    
    render! {
        nav { class: "main-navigation",
            // Core menu items
            NavigationItem { 
                label: "Dashboard", 
                route: "/", 
                icon: "home" 
            }
            
            // Plugin-contributed menu items
            for menu_item in menu_items {
                NavigationItem {
                    key: "{menu_item.id}",
                    label: "{menu_item.label}",
                    route: "{menu_item.route}",
                    icon: "{menu_item.icon:?}",
                    children: menu_item.children.clone()
                }
            }
        }
    }
}
```

#### 2. API Integration Example

**How External Systems Use Plugin APIs:**

```javascript
// External JavaScript application using the Product Catalog API
class ProductCatalogClient {
    constructor(baseUrl, apiKey) {
        this.baseUrl = baseUrl;
        this.apiKey = apiKey;
    }
    
    async getProducts(filters = {}) {
        const queryParams = new URLSearchParams(filters);
        const response = await fetch(`${this.baseUrl}/api/products?${queryParams}`, {
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            }
        });
        
        if (!response.ok) {
            throw new Error(`API request failed: ${response.status}`);
        }
        
        return response.json();
    }
    
    async createProduct(productData) {
        const response = await fetch(`${this.baseUrl}/api/products`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(productData)
        });
        
        if (!response.ok) {
            throw new Error(`Failed to create product: ${response.status}`);
        }
        
        return response.json();
    }
}

// Usage in external application
const catalog = new ProductCatalogClient('https://api.mycompany.com', 'user-api-key');

// List products
const products = await catalog.getProducts({
    category: 'electronics',
    page: 1,
    limit: 20
});

// Create new product
const newProduct = await catalog.createProduct({
    name: 'Smartphone XYZ',
    description: 'Latest smartphone with advanced features',
    price: 599.99,
    category_id: 'electronics-123',
    sku: 'PHONE-XYZ-001'
});
```

#### 3. User Permission-Based Experience

**Different User Experiences:**

```rust
// Administrator sees full product management interface
#[component]
fn ProductManagementPage(cx: Scope) -> Element {
    let current_user = use_shared_state::<User>(cx)?;
    let user = current_user.read();
    
    render! {
        div { class: "product-management",
            if user.has_permission("products", "write") {
                div { class: "admin-actions",
                    button { 
                        class: "btn btn-primary",
                        onclick: |_| { /* Create product */ },
                        "Add New Product"
                    }
                    button { 
                        class: "btn btn-secondary",
                        onclick: |_| { /* Bulk import */ },
                        "Bulk Import"
                    }
                    button { 
                        class: "btn btn-danger",
                        onclick: |_| { /* Delete selected */ },
                        "Delete Selected"
                    }
                }
            }
            
            if user.has_permission("products", "read") {
                ProductGrid { editable: user.has_permission("products", "write") }
            } else {
                div { class: "no-permission",
                    "You don't have permission to view products."
                }
            }
            
            if user.has_permission("inventory", "read") {
                InventoryPanel { }
            }
        }
    }
}

// Customer sees product browsing interface
#[component]
fn ProductBrowsingPage(cx: Scope) -> Element {
    render! {
        div { class: "product-browsing",
            SearchBar { }
            CategoryFilter { }
            ProductGrid { 
                editable: false,
                show_prices: true,
                show_add_to_cart: true
            }
            ShoppingCart { }
        }
    }
}
```

#### 4. Plugin Configuration by Users

**Settings Interface Integration:**
```rust
#[component]
fn PluginSettingsPage(cx: Scope, plugin_id: String) -> Element {
    let plugin_manager = use_shared_state::<PluginManager>(cx)?;
    let settings_manager = use_shared_state::<SettingsManager>(cx)?;
    let current_user = use_shared_state::<User>(cx)?;
    
    let plugin = plugin_manager.read().get_plugin(&plugin_id)?;
    let settings_schema = plugin.settings_schema()?;
    
    // Check if user has permission to modify plugin settings
    let can_edit = current_user.read().has_permissions(&settings_schema.required_permissions);
    
    render! {
        div { class: "plugin-settings",
            h2 { "Settings for {plugin.info().name}" }
            
            if can_edit {
                SettingsForm { 
                    schema: settings_schema,
                    on_save: move |values| {
                        // Save plugin settings
                    }
                }
            } else {
                SettingsViewer { 
                    schema: settings_schema,
                    read_only: true
                }
            }
        }
    }
}
```

### Cross-Platform Plugin Interaction

#### Desktop Platform
- **Full Feature Access**: Plugins can access filesystem, native APIs, system notifications
- **Performance**: Direct database access, full threading support
- **Integration**: System tray, file associations, native menus

#### Mobile Platform
- **Touch Optimized**: Plugin UIs adapt to touch interfaces
- **Background Limitations**: Reduced background processing capabilities
- **Platform APIs**: Camera, GPS, push notifications through platform abstraction

#### Web Platform (WASM)
- **Browser Sandbox**: Limited file access, no direct database connections
- **Fallback Services**: Server-side APIs for heavy operations
- **Progressive Enhancement**: Core features work offline, advanced features require connectivity

---

## Complete System Interaction Flow

### Example: Adding a Product Through the System

Let's trace how a complete operation flows through the system:

#### 1. User Interaction (UI Layer)
```rust
// User clicks "Add Product" button in the UI
#[component]
fn AddProductButton(cx: Scope) -> Element {
    let plugin_manager = use_shared_state::<PluginManager>(cx)?;
    
    render! {
        button {
            class: "btn btn-primary",
            onclick: move |_| {
                // Navigate to add product form
                navigator.push("/products/add");
            },
            "Add Product"
        }
    }
}
```

#### 2. Route Handling (Plugin UI Component)
```rust
// Plugin-registered route handles the request
#[component]
fn AddProductForm(cx: Scope) -> Element {
    let form_data = use_state(cx, || CreateProductRequest::default());
    
    render! {
        form {
            onsubmit: move |evt| {
                evt.prevent_default();
                // Submit to plugin API
                submit_product_form(form_data.get());
            },
            // Form fields...
        }
    }
}

async fn submit_product_form(data: CreateProductRequest) {
    // Call plugin API endpoint
    let response = api_client.post("/api/products")
        .json(&data)
        .send()
        .await?;
        
    if response.status().is_success() {
        // Show success message, redirect to product list
    } else {
        // Handle validation errors
    }
}
```

#### 3. API Processing (Plugin API Handler)
```rust
// Plugin's API handler processes the request
#[async_trait]
impl ApiHandler for CreateProductHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse> {
        // 1. Authentication & Authorization
        let user = authenticate_request(&request)?;
        authorize_action(&user, "products", "write")?;
        
        // 2. Parse and validate input
        let product_data: CreateProductRequest = parse_request_body(&request)?;
        validate_product_data(&product_data)?;
        
        // 3. Business logic
        let new_product = create_product(product_data, &user).await?;
        
        // 4. Publish events
        publish_event(ProductCreatedEvent {
            product_id: new_product.id.clone(),
            created_by: user.id.clone(),
        }).await?;
        
        // 5. Return response
        Ok(ApiResponse::success(new_product))
    }
}
```

#### 4. Database Operations (Database Manager)
```rust
async fn create_product(data: CreateProductRequest, user: &User) -> Result<Product> {
    // Use platform-appropriate database
    let db = get_database_connection().await?;
    
    db.transaction(|tx| async move {
        // 1. Insert product
        let product = Product {
            id: Uuid::new_v4().to_string(),
            name: data.name,
            price: data.price,
            created_by: Some(user.id.clone()),
            created_at: Utc::now(),
            // ... other fields
        };
        
        tx.execute(
            "INSERT INTO products (id, name, price, created_by, created_at) VALUES (?, ?, ?, ?, ?)",
            &[&product.id, &product.name, &product.price, &product.created_by, &product.created_at]
        ).await?;
        
        // 2. Initialize inventory if tracking enabled
        if is_inventory_tracking_enabled().await? {
            tx.execute(
                "INSERT INTO inventory (product_id, quantity, low_stock_threshold) VALUES (?, ?, ?)",
                &[&product.id, &data.initial_quantity.unwrap_or(0), &10]
            ).await?;
        }
        
        Ok(product)
    }).await
}
```

#### 5. Event Processing (Event System)
```rust
// Event is published and processed by interested handlers
pub struct InventoryAlertHandler;

#[async_trait]
impl EventHandler for InventoryAlertHandler {
    async fn handle(&self, event: &dyn Event) -> Result<()> {
        if event.event_type() == "product.created" {
            if let Some(product_event) = event.as_any().downcast_ref::<ProductCreatedEvent>() {
                // Check if this triggers any inventory alerts
                check_inventory_alerts(&product_event.product_id).await?;
                
                // Update dashboard statistics
                update_product_statistics().await?;
                
                // Send notifications to relevant users
                notify_product_managers(&product_event).await?;
            }
        }
        Ok(())
    }
}
```

#### 6. UI Updates (Real-time Updates)
```rust
// UI components automatically update via shared state
#[component]
fn ProductList(cx: Scope) -> Element {
    let products = use_shared_state::<Vec<Product>>(cx)?;
    
    // Listen for product events
    use_effect(cx, (), |_| {
        let event_subscription = event_bus.subscribe(EventFilter::new()
            .with_event_type("product.created")
            .with_event_type("product.updated")
        );
        
        spawn(async move {
            while let Ok(event) = event_subscription.recv().await {
                // Refresh product list
                refresh_products().await;
            }
        });
    });
    
    render! {
        div { class: "product-list",
            for product in products.read().iter() {
                ProductCard { product: product.clone() }
            }
        }
    }
}
```

#### 7. Platform-Specific Adaptations

**Desktop**: Direct database access, native file operations
```rust
#[cfg(not(target_arch = "wasm32"))]
async fn save_product_image(image_data: &[u8]) -> Result<String> {
    let file_path = format!("./uploads/products/{}.jpg", Uuid::new_v4());
    fs::write(&file_path, image_data).await?;
    Ok(file_path)
}
```

**WASM**: Server-side processing via API calls
```rust
#[cfg(target_arch = "wasm32")]
async fn save_product_image(image_data: &[u8]) -> Result<String> {
    let form_data = FormData::new()?;
    form_data.append_with_blob("image", &Blob::new_with_u8_array_sequence(&[image_data])?)?;
    
    let response = web_sys::window()
        .unwrap()
        .fetch_with_request(&Request::new_with_str_and_init(
            "/api/upload/image",
            RequestInit::new().method("POST").body(Some(&form_data))
        )?);
        
    let response_text = JsFuture::from(response).await?.as_string().unwrap();
    Ok(response_text)
}
```

---

## Summary and Conclusion

This comprehensive design guide outlines a sophisticated, cross-platform application framework that achieves the following goals:

### Key Achievements

1. **Universal Platform Support**:
    - Single codebase runs on desktop, mobile, and web
    - Platform-specific optimizations and fallbacks
    - Consistent user experience across all platforms

2. **Extensible Plugin Architecture**:
    - Safe, sandboxed plugin execution
    - Rich plugin APIs for UI, data, and business logic
    - Hot-reloading and dependency management

3. **Role-Based Access Control**:
    - Granular permissions system
    - Dynamic UI adaptation based on user roles
    - Secure API access with proper authorization

4. **Enterprise-Ready Features**:
    - Comprehensive configuration management
    - Real-time monitoring and health checks
    - Scalable architecture with proper error handling

5. **Developer-Friendly**:
    - Well-defined plugin development SDK
    - Comprehensive documentation and examples
    - Type-safe APIs with excellent tooling support

### Technical Strengths

- **Manager-Based Architecture**: Provides consistent lifecycle management
- **Event-Driven Communication**: Enables loose coupling and extensibility
- **Platform Abstraction**: Clean separation between business logic and platform code
- **Configuration-Driven Behavior**: Highly customizable without code changes
- **Modern Rust Ecosystem**: Leverages the best of async Rust and web technologies

### Development Recommendations

1. **Start Small**: Begin with core managers and a simple plugin
2. **Iterate Frequently**: Build, test, and refine each component
3. **Platform Testing**: Test early and often on all target platforms
4. **Security First**: Implement security measures from the beginning
5. **Documentation**: Maintain comprehensive documentation throughout development

### Real-World Applications

This framework is ideal for:
- **Business Management Systems**: CRM, ERP, project management
- **E-commerce Platforms**: Multi-tenant online stores
- **Content Management**: Publishing and media management
- **IoT Dashboards**: Device monitoring and control
- **Educational Platforms**: Learning management systems

### Future Extensibility

The architecture supports future enhancements such as:
- **Microservices Integration**: Plugin-based service decomposition
- **Machine Learning**: AI/ML plugins for intelligent features
- **Real-time Collaboration**: WebRTC plugins for live collaboration
- **Blockchain Integration**: Cryptocurrency and smart contract plugins
- **Advanced Analytics**: Business intelligence and reporting plugins