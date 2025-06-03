// // example_plugin/src/lib.rs
//
// //! Todo List Plugin
// //!
// //! This plugin provides todo list functionality with CRUD operations,
// //! search capabilities, and a simple UI. It demonstrates all major
// //! plugin system features including database access, API routes,
// //! search providers, and UI components.
//
// use std::collections::HashMap;
// use std::sync::Arc;
// use async_trait::async_trait;
// use serde::{Deserialize, Serialize};
// use tokio::sync::RwLock;
// use chrono::{DateTime, Utc};
//
// use qorzen_oxide::{
//     plugin::*,
//     plugin::search::*,
//     auth::{Permission, PermissionScope},
//     error::{Error, Result},
//     event::Event,
//     config::SettingsSchema,
// };
//
// /// Todo item data structure
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TodoItem {
//     pub id: String,
//     pub title: String,
//     pub description: Option<String>,
//     pub completed: bool,
//     pub priority: Priority,
//     pub category: String,
//     pub tags: Vec<String>,
//     pub created_at: DateTime<Utc>,
//     pub updated_at: DateTime<Utc>,
//     pub due_date: Option<DateTime<Utc>>,
// }
//
// /// Priority levels for todo items
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum Priority {
//     Low,
//     Medium,
//     High,
//     Critical,
// }
//
// impl std::fmt::Display for Priority {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Priority::Low => write!(f, "Low"),
//             Priority::Medium => write!(f, "Medium"),
//             Priority::High => write!(f, "High"),
//             Priority::Critical => write!(f, "Critical"),
//         }
//     }
// }
//
// /// Plugin configuration
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TodoConfig {
//     pub max_items: usize,
//     pub enable_notifications: bool,
//     pub default_category: String,
//     pub auto_archive_completed: bool,
//     pub search_enabled: bool,
// }
//
// impl Default for TodoConfig {
//     fn default() -> Self {
//         Self {
//             max_items: 1000,
//             enable_notifications: true,
//             default_category: "General".to_string(),
//             auto_archive_completed: false,
//             search_enabled: true,
//         }
//     }
// }
//
// /// Todo service for data operations
// #[derive(Debug)]
// pub struct TodoService {
//     items: Arc<RwLock<HashMap<String, TodoItem>>>,
//     config: TodoConfig,
//     database: Option<PluginDatabase>,
// }
//
// impl TodoService {
//     /// Create a new todo service
//     pub fn new(config: TodoConfig, database: Option<PluginDatabase>) -> Self {
//         Self {
//             items: Arc::new(RwLock::new(HashMap::new())),
//             config,
//             database,
//         }
//     }
//
//     /// Create a new todo item
//     pub async fn create_item(&self, title: String, description: Option<String>) -> Result<TodoItem> {
//         let items = self.items.read().await;
//         if items.len() >= self.config.max_items {
//             return Err(Error::plugin("todo_plugin", "Maximum number of items reached"));
//         }
//         drop(items);
//
//         let item = TodoItem {
//             id: uuid::Uuid::new_v4().to_string(),
//             title,
//             description,
//             completed: false,
//             priority: Priority::Medium,
//             category: self.config.default_category.clone(),
//             tags: Vec::new(),
//             created_at: chrono::Utc::now(),
//             updated_at: chrono::Utc::now(),
//             due_date: None,
//         };
//
//         // Store in database if available
//         if let Some(ref db) = self.database {
//             let query = "INSERT INTO todos (id, title, description, completed, priority, category, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
//             let params = vec![
//                 serde_json::Value::String(item.id.clone()),
//                 serde_json::Value::String(item.title.clone()),
//                 serde_json::Value::String(item.description.clone().unwrap_or_default()),
//                 serde_json::Value::Bool(item.completed),
//                 serde_json::Value::String(item.priority.to_string()),
//                 serde_json::Value::String(item.category.clone()),
//                 serde_json::Value::String(item.created_at.to_rfc3339()),
//                 serde_json::Value::String(item.updated_at.to_rfc3339()),
//             ];
//             db.execute(query, &params).await?;
//         }
//
//         // Store in memory
//         let mut items = self.items.write().await;
//         items.insert(item.id.clone(), item.clone());
//
//         Ok(item)
//     }
//
//     /// Get all todo items
//     pub async fn get_items(&self) -> Vec<TodoItem> {
//         self.items.read().await.values().cloned().collect()
//     }
//
//     /// Get a specific todo item by ID
//     pub async fn get_item(&self, id: &str) -> Option<TodoItem> {
//         self.items.read().await.get(id).cloned()
//     }
//
//     /// Update a todo item
//     pub async fn update_item(&self, id: &str, updates: TodoUpdate) -> Result<TodoItem> {
//         let mut items = self.items.write().await;
//
//         if let Some(item) = items.get_mut(id) {
//             if let Some(title) = updates.title {
//                 item.title = title;
//             }
//             if let Some(description) = updates.description {
//                 item.description = description;
//             }
//             if let Some(completed) = updates.completed {
//                 item.completed = completed;
//             }
//             if let Some(priority) = updates.priority {
//                 item.priority = priority;
//             }
//             if let Some(category) = updates.category {
//                 item.category = category;
//             }
//             if let Some(tags) = updates.tags {
//                 item.tags = tags;
//             }
//             if let Some(due_date) = updates.due_date {
//                 item.due_date = due_date;
//             }
//             item.updated_at = chrono::Utc::now();
//
//             Ok(item.clone())
//         } else {
//             Err(Error::plugin("todo_plugin", "Todo item not found"))
//         }
//     }
//
//     /// Delete a todo item
//     pub async fn delete_item(&self, id: &str) -> Result<()> {
//         let mut items = self.items.write().await;
//
//         if items.remove(id).is_some() {
//             // Remove from database if available
//             if let Some(ref db) = self.database {
//                 let query = "DELETE FROM todos WHERE id = ?";
//                 let params = vec![serde_json::Value::String(id.to_string())];
//                 db.execute(query, &params).await?;
//             }
//             Ok(())
//         } else {
//             Err(Error::plugin("todo_plugin", "Todo item not found"))
//         }
//     }
//
//     /// Search todo items
//     pub async fn search_items(&self, query: &str) -> Vec<TodoItem> {
//         let items = self.items.read().await;
//         let query_lower = query.to_lowercase();
//
//         items.values()
//             .filter(|item| {
//                 item.title.to_lowercase().contains(&query_lower) ||
//                     item.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query_lower)) ||
//                     item.category.to_lowercase().contains(&query_lower) ||
//                     item.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
//             })
//             .cloned()
//             .collect()
//     }
//
//     /// Get items by category
//     pub async fn get_items_by_category(&self, category: &str) -> Vec<TodoItem> {
//         let items = self.items.read().await;
//         items.values()
//             .filter(|item| item.category == category)
//             .cloned()
//             .collect()
//     }
//
//     /// Get categories
//     pub async fn get_categories(&self) -> Vec<String> {
//         let items = self.items.read().await;
//         let mut categories: Vec<String> = items.values()
//             .map(|item| item.category.clone())
//             .collect::<std::collections::HashSet<_>>()
//             .into_iter()
//             .collect();
//         categories.sort();
//         categories
//     }
// }
//
// /// Todo update structure
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TodoUpdate {
//     pub title: Option<String>,
//     pub description: Option<Option<String>>,
//     pub completed: Option<bool>,
//     pub priority: Option<Priority>,
//     pub category: Option<String>,
//     pub tags: Option<Vec<String>>,
//     pub due_date: Option<Option<DateTime<Utc>>>,
// }
//
// /// Todo search provider
// #[derive(Debug)]
// pub struct TodoSearchProvider {
//     service: Arc<TodoService>,
// }
//
// impl TodoSearchProvider {
//     /// Create a new todo search provider
//     pub fn new(service: Arc<TodoService>) -> Self {
//         Self { service }
//     }
// }
//
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// impl SearchProvider for TodoSearchProvider {
//     fn provider_id(&self) -> &str {
//         "todo_search"
//     }
//
//     fn provider_name(&self) -> &str {
//         "Todo Search"
//     }
//
//     fn description(&self) -> &str {
//         "Search todo items by title, description, category, and tags"
//     }
//
//     fn priority(&self) -> i32 {
//         150 // Medium-high priority
//     }
//
//     fn supported_result_types(&self) -> Vec<String> {
//         vec!["todo".to_string()]
//     }
//
//     fn supports_facets(&self) -> bool {
//         true
//     }
//
//     fn supports_suggestions(&self) -> bool {
//         true
//     }
//
//     async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
//         let items = self.service.search_items(&query.query).await;
//         let mut results = Vec::new();
//
//         for item in items {
//             let score = if item.title.to_lowercase().contains(&query.query.to_lowercase()) {
//                 0.9
//             } else if item.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query.query.to_lowercase())) {
//                 0.7
//             } else if item.category.to_lowercase().contains(&query.query.to_lowercase()) {
//                 0.6
//             } else {
//                 0.5
//             };
//
//             let mut metadata = HashMap::new();
//             metadata.insert("priority".to_string(), serde_json::json!(item.priority.to_string()));
//             metadata.insert("category".to_string(), serde_json::json!(item.category));
//             metadata.insert("completed".to_string(), serde_json::json!(item.completed));
//             metadata.insert("tags".to_string(), serde_json::json!(item.tags));
//
//             results.push(SearchResult {
//                 id: item.id.clone(),
//                 result_type: "todo".to_string(),
//                 title: item.title.clone(),
//                 description: item.description.clone(),
//                 score,
//                 url: Some(format!("/plugins/todo/items/{}", item.id)),
//                 thumbnail: None,
//                 metadata,
//                 source_plugin: "todo_plugin".to_string(),
//                 timestamp: item.updated_at,
//             });
//         }
//
//         // Apply limit if specified
//         if let Some(limit) = query.limit {
//             results.truncate(limit);
//         }
//
//         Ok(results)
//     }
//
//     async fn get_facets(&self, _query: &SearchQuery) -> Result<Vec<SearchFacet>> {
//         let categories = self.service.get_categories().await;
//
//         let category_facet = SearchFacet {
//             field: "category".to_string(),
//             name: "Category".to_string(),
//             values: categories.into_iter().map(|cat| FacetValue {
//                 value: serde_json::Value::String(cat.clone()),
//                 display_name: cat,
//                 count: 0, // Would be calculated from actual data
//             }).collect(),
//         };
//
//         let priority_facet = SearchFacet {
//             field: "priority".to_string(),
//             name: "Priority".to_string(),
//             values: vec![
//                 FacetValue {
//                     value: serde_json::Value::String("Low".to_string()),
//                     display_name: "Low".to_string(),
//                     count: 0,
//                 },
//                 FacetValue {
//                     value: serde_json::Value::String("Medium".to_string()),
//                     display_name: "Medium".to_string(),
//                     count: 0,
//                 },
//                 FacetValue {
//                     value: serde_json::Value::String("High".to_string()),
//                     display_name: "High".to_string(),
//                     count: 0,
//                 },
//                 FacetValue {
//                     value: serde_json::Value::String("Critical".to_string()),
//                     display_name: "Critical".to_string(),
//                     count: 0,
//                 },
//             ],
//         };
//
//         Ok(vec![category_facet, priority_facet])
//     }
//
//     async fn get_suggestions(&self, query: &SearchQuery) -> Result<Vec<SearchSuggestion>> {
//         let items = self.service.search_items(&query.query).await;
//         let mut suggestions = Vec::new();
//
//         for item in items.into_iter().take(5) {
//             suggestions.push(SearchSuggestion {
//                 text: query.query.clone(),
//                 completion: item.title,
//                 category: Some("Todo".to_string()),
//                 score: 0.8,
//             });
//         }
//
//         Ok(suggestions)
//     }
//
//     async fn health_check(&self) -> Result<ProviderHealth> {
//         let start = std::time::Instant::now();
//
//         // Simple health check - try to get items
//         let _items = self.service.get_items().await;
//
//         Ok(ProviderHealth {
//             is_healthy: true,
//             response_time_ms: Some(start.elapsed().as_millis() as u64),
//             error_message: None,
//             last_check: chrono::Utc::now(),
//         })
//     }
// }
//
// /// Main todo plugin implementation
// #[derive(Debug)]
// pub struct TodoPlugin {
//     service: Option<Arc<TodoService>>,
//     search_provider: Option<Arc<TodoSearchProvider>>,
//     context: Option<PluginContext>,
// }
//
// impl TodoPlugin {
//     /// Create a new todo plugin
//     pub fn new() -> Self {
//         Self {
//             service: None,
//             search_provider: None,
//             context: None,
//         }
//     }
//
//     /// Get the todo service
//     fn get_service(&self) -> Result<&Arc<TodoService>> {
//         self.service.as_ref()
//             .ok_or_else(|| Error::plugin("todo_plugin", "Service not initialized"))
//     }
// }
//
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// impl Plugin for TodoPlugin {
//     fn info(&self) -> PluginInfo {
//         PluginInfo {
//             id: "todo_plugin".to_string(),
//             name: "Todo List".to_string(),
//             version: "1.0.0".to_string(),
//             description: "A comprehensive todo list plugin with search and categorization".to_string(),
//             author: "Qorzen Team".to_string(),
//             license: "MIT".to_string(),
//             homepage: Some("https://github.com/sssolid/plugins/todo".to_string()),
//             repository: Some("https://github.com/sssolid/plugins".to_string()),
//             minimum_core_version: "0.1.0".to_string(),
//             supported_platforms: vec![Platform::All],
//         }
//     }
//
//     fn required_dependencies(&self) -> Vec<PluginDependency> {
//         vec![]
//     }
//
//     fn required_permissions(&self) -> Vec<Permission> {
//         vec![
//             Permission {
//                 resource: "todos".to_string(),
//                 action: "read".to_string(),
//                 scope: PermissionScope::Global,
//             },
//             Permission {
//                 resource: "todos".to_string(),
//                 action: "write".to_string(),
//                 scope: PermissionScope::Global,
//             },
//             Permission {
//                 resource: "search".to_string(),
//                 action: "provide".to_string(),
//                 scope: PermissionScope::Global,
//             },
//             Permission {
//                 resource: "ui".to_string(),
//                 action: "render".to_string(),
//                 scope: PermissionScope::Global,
//             },
//         ]
//     }
//
//     async fn initialize(&mut self, context: PluginContext) -> Result<()> {
//         // Load configuration from context
//         let config = if let Ok(Some(config_value)) = context.api_client.get_config("todo_plugin").await {
//             serde_json::from_value(config_value).unwrap_or_default()
//         } else {
//             TodoConfig::default()
//         };
//
//         // Initialize service
//         let service = Arc::new(TodoService::new(config, context.database.clone()));
//
//         // Initialize search provider if search is enabled
//         let search_provider = if config.search_enabled {
//             Some(Arc::new(TodoSearchProvider::new(Arc::clone(&service))))
//         } else {
//             None
//         };
//
//         self.service = Some(service);
//         self.search_provider = search_provider;
//         self.context = Some(context);
//
//         tracing::info!("Todo plugin initialized successfully");
//         Ok(())
//     }
//
//     async fn shutdown(&mut self) -> Result<()> {
//         tracing::info!("Todo plugin shutting down");
//         Ok(())
//     }
//
//     fn ui_components(&self) -> Vec<UIComponent> {
//         vec![
//             UIComponent {
//                 id: "todo_list".to_string(),
//                 name: "Todo List".to_string(),
//                 component_type: ComponentType::Page,
//                 props: serde_json::json!({
//                     "title": "Todo List",
//                     "searchable": true,
//                     "categorizable": true
//                 }),
//                 required_permissions: vec![
//                     Permission {
//                         resource: "todos".to_string(),
//                         action: "read".to_string(),
//                         scope: PermissionScope::Global,
//                     }
//                 ],
//             },
//             UIComponent {
//                 id: "todo_item".to_string(),
//                 name: "Todo Item".to_string(),
//                 component_type: ComponentType::Widget,
//                 props: serde_json::json!({
//                     "editable": true,
//                     "deletable": true
//                 }),
//                 required_permissions: vec![
//                     Permission {
//                         resource: "todos".to_string(),
//                         action: "read".to_string(),
//                         scope: PermissionScope::Global,
//                     }
//                 ],
//             },
//         ]
//     }
//
//     fn menu_items(&self) -> Vec<MenuItem> {
//         vec![
//             MenuItem {
//                 id: "todos".to_string(),
//                 label: "Todo List".to_string(),
//                 icon: Some("📝".to_string()),
//                 route: Some("/plugins/todo".to_string()),
//                 action: None,
//                 required_permissions: vec![
//                     Permission {
//                         resource: "todos".to_string(),
//                         action: "read".to_string(),
//                         scope: PermissionScope::Global,
//                     }
//                 ],
//                 order: 200,
//                 children: vec![
//                     MenuItem {
//                         id: "todo_list".to_string(),
//                         label: "All Items".to_string(),
//                         icon: Some("📋".to_string()),
//                         route: Some("/plugins/todo/items".to_string()),
//                         action: None,
//                         required_permissions: vec![],
//                         order: 0,
//                         children: vec![],
//                     },
//                     MenuItem {
//                         id: "todo_categories".to_string(),
//                         label: "Categories".to_string(),
//                         icon: Some("🏷️".to_string()),
//                         route: Some("/plugins/todo/categories".to_string()),
//                         action: None,
//                         required_permissions: vec![],
//                         order: 1,
//                         children: vec![],
//                     },
//                 ],
//             }
//         ]
//     }
//
//     fn settings_schema(&self) -> Option<SettingsSchema> {
//         Some(SettingsSchema {
//             version: "1.0".to_string(),
//             schema: serde_json::json!({
//                 "type": "object",
//                 "properties": {
//                     "max_items": {
//                         "type": "integer",
//                         "title": "Maximum Items",
//                         "description": "Maximum number of todo items allowed",
//                         "default": 1000,
//                         "minimum": 1,
//                         "maximum": 10000
//                     },
//                     "enable_notifications": {
//                         "type": "boolean",
//                         "title": "Enable Notifications",
//                         "description": "Enable notifications for due dates and reminders",
//                         "default": true
//                     },
//                     "default_category": {
//                         "type": "string",
//                         "title": "Default Category",
//                         "description": "Default category for new todo items",
//                         "default": "General"
//                     },
//                     "auto_archive_completed": {
//                         "type": "boolean",
//                         "title": "Auto Archive Completed",
//                         "description": "Automatically archive completed items after a period",
//                         "default": false
//                     },
//                     "search_enabled": {
//                         "type": "boolean",
//                         "title": "Enable Search",
//                         "description": "Enable search provider functionality",
//                         "default": true
//                     }
//                 }
//             }),
//             defaults: serde_json::to_value(TodoConfig::default()).unwrap(),
//         })
//     }
//
//     fn api_routes(&self) -> Vec<ApiRoute> {
//         vec![
//             ApiRoute {
//                 path: "/api/plugins/todo/items".to_string(),
//                 method: HttpMethod::GET,
//                 handler_id: "list_items".to_string(),
//                 required_permissions: vec![
//                     Permission {
//                         resource: "todos".to_string(),
//                         action: "read".to_string(),
//                         scope: PermissionScope::Global,
//                     }
//                 ],
//                 rate_limit: Some(RateLimit {
//                     requests_per_minute: 60,
//                     burst_limit: 10,
//                 }),
//                 documentation: ApiDocumentation {
//                     summary: "List todo items".to_string(),
//                     description: "Get a list of todo items with optional filtering".to_string(),
//                     parameters: vec![
//                         ApiParameter {
//                             name: "category".to_string(),
//                             parameter_type: ParameterType::Query,
//                             required: false,
//                             description: "Filter by category".to_string(),
//                             example: Some(serde_json::json!("Work")),
//                         },
//                         ApiParameter {
//                             name: "completed".to_string(),
//                             parameter_type: ParameterType::Query,
//                             required: false,
//                             description: "Filter by completion status".to_string(),
//                             example: Some(serde_json::json!(false)),
//                         },
//                     ],
//                     responses: vec![
//                         ApiResponse {
//                             status_code: 200,
//                             description: "List of todo items".to_string(),
//                             schema: Some(serde_json::json!({
//                                 "type": "array",
//                                 "items": {
//                                     "type": "object",
//                                     "properties": {
//                                         "id": {"type": "string"},
//                                         "title": {"type": "string"},
//                                         "completed": {"type": "boolean"}
//                                     }
//                                 }
//                             })),
//                         }
//                     ],
//                     examples: vec![],
//                 },
//             },
//             ApiRoute {
//                 path: "/api/plugins/todo/items".to_string(),
//                 method: HttpMethod::POST,
//                 handler_id: "create_item".to_string(),
//                 required_permissions: vec![
//                     Permission {
//                         resource: "todos".to_string(),
//                         action: "write".to_string(),
//                         scope: PermissionScope::Global,
//                     }
//                 ],
//                 rate_limit: Some(RateLimit {
//                     requests_per_minute: 30,
//                     burst_limit: 5,
//                 }),
//                 documentation: ApiDocumentation {
//                     summary: "Create todo item".to_string(),
//                     description: "Create a new todo item".to_string(),
//                     parameters: vec![],
//                     responses: vec![],
//                     examples: vec![],
//                 },
//             },
//         ]
//     }
//
//     fn event_handlers(&self) -> Vec<EventHandler> {
//         vec![
//             EventHandler {
//                 event_type: "todo.created".to_string(),
//                 handler_id: "handle_todo_created".to_string(),
//                 priority: 100,
//             },
//             EventHandler {
//                 event_type: "todo.completed".to_string(),
//                 handler_id: "handle_todo_completed".to_string(),
//                 priority: 100,
//             },
//         ]
//     }
//
//     fn render_component(&self, component_id: &str, _props: serde_json::Value) -> Result<dioxus::prelude::VNode> {
//         match component_id {
//             "todo_list" => {
//                 // In a real implementation, this would render the todo list component
//                 Err(Error::plugin("todo_plugin", "Component rendering requires Dioxus runtime"))
//             }
//             "todo_item" => {
//                 // In a real implementation, this would render the todo item component
//                 Err(Error::plugin("todo_plugin", "Component rendering requires Dioxus runtime"))
//             }
//             _ => Err(Error::plugin("todo_plugin", "Unknown component"))
//         }
//     }
//
//     async fn handle_api_request(&self, route_id: &str, request: ApiRequest) -> Result<ApiResponse> {
//         let service = self.get_service()?;
//
//         match route_id {
//             "list_items" => {
//                 let items = if let Some(category) = request.query_params.get("category") {
//                     service.get_items_by_category(category).await
//                 } else {
//                     service.get_items().await
//                 };
//
//                 // Filter by completion status if specified
//                 let filtered_items = if let Some(completed_str) = request.query_params.get("completed") {
//                     let completed = completed_str.parse::<bool>().unwrap_or(false);
//                     items.into_iter().filter(|item| item.completed == completed).collect()
//                 } else {
//                     items
//                 };
//
//                 Ok(ApiResponse {
//                     status_code: 200,
//                     description: "Success".to_string(),
//                     schema: Some(serde_json::to_value(&filtered_items)?),
//                 })
//             }
//             "create_item" => {
//                 if let Some(body) = request.body {
//                     let title = body.get("title")
//                         .and_then(|v| v.as_str())
//                         .ok_or_else(|| Error::plugin("todo_plugin", "Title is required"))?;
//
//                     let description = body.get("description")
//                         .and_then(|v| v.as_str())
//                         .map(|s| s.to_string());
//
//                     let item = service.create_item(title.to_string(), description).await?;
//
//                     Ok(ApiResponse {
//                         status_code: 201,
//                         description: "Created".to_string(),
//                         schema: Some(serde_json::to_value(&item)?),
//                     })
//                 } else {
//                     Err(Error::plugin("todo_plugin", "Request body is required"))
//                 }
//             }
//             _ => Err(Error::plugin("todo_plugin", "Unknown API route"))
//         }
//     }
//
//     async fn handle_event(&self, handler_id: &str, event: &dyn Event) -> Result<()> {
//         match handler_id {
//             "handle_todo_created" => {
//                 tracing::info!("Todo created: {}", event.event_type());
//                 // Could send notifications, update statistics, etc.
//                 Ok(())
//             }
//             "handle_todo_completed" => {
//                 tracing::info!("Todo completed: {}", event.event_type());
//                 // Could trigger auto-archive, send notifications, etc.
//                 Ok(())
//             }
//             _ => Err(Error::plugin("todo_plugin", "Unknown event handler"))
//         }
//     }
// }
//
// // Export the plugin using the SDK macro
// qorzen_oxide::export_plugin!(TodoPlugin);
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_plugin_info() {
//         let plugin = TodoPlugin::new();
//         let info = plugin.info();
//
//         assert_eq!(info.id, "todo_plugin");
//         assert_eq!(info.name, "Todo List");
//         assert!(!info.supported_platforms.is_empty());
//     }
//
//     #[test]
//     fn test_plugin_permissions() {
//         let plugin = TodoPlugin::new();
//         let permissions = plugin.required_permissions();
//
//         assert!(!permissions.is_empty());
//         assert!(permissions.iter().any(|p| p.resource == "todos" && p.action == "read"));
//         assert!(permissions.iter().any(|p| p.resource == "todos" && p.action == "write"));
//     }
//
//     #[tokio::test]
//     async fn test_todo_service() {
//         let config = TodoConfig::default();
//         let service = TodoService::new(config, None);
//
//         // Test creating an item
//         let item = service.create_item("Test Todo".to_string(), Some("Test description".to_string())).await.unwrap();
//         assert_eq!(item.title, "Test Todo");
//         assert!(!item.completed);
//
//         // Test getting items
//         let items = service.get_items().await;
//         assert_eq!(items.len(), 1);
//
//         // Test updating an item
//         let update = TodoUpdate {
//             completed: Some(true),
//             ..Default::default()
//         };
//         let updated_item = service.update_item(&item.id, update).await.unwrap();
//         assert!(updated_item.completed);
//
//         // Test searching
//         let search_results = service.search_items("Test").await;
//         assert_eq!(search_results.len(), 1);
//
//         // Test deleting
//         service.delete_item(&item.id).await.unwrap();
//         let items = service.get_items().await;
//         assert!(items.is_empty());
//     }
//
//     #[tokio::test]
//     async fn test_search_provider() {
//         let config = TodoConfig::default();
//         let service = Arc::new(TodoService::new(config, None));
//         let provider = TodoSearchProvider::new(service.clone());
//
//         // Create some test items
//         service.create_item("Test Todo 1".to_string(), Some("Description 1".to_string())).await.unwrap();
//         service.create_item("Another Task".to_string(), Some("Description 2".to_string())).await.unwrap();
//
//         // Test search
//         let query = SearchQuery {
//             query: "Test".to_string(),
//             limit: Some(10),
//             offset: None,
//             filters: HashMap::new(),
//             facets: vec![],
//             include_suggestions: false,
//             context: SearchContext {
//                 user_id: None,
//                 permissions: vec![],
//                 preferences: HashMap::new(),
//                 metadata: HashMap::new(),
//             },
//         };
//
//         let results = provider.search(&query).await.unwrap();
//         assert_eq!(results.len(), 1);
//         assert_eq!(results[0].title, "Test Todo 1");
//
//         // Test health check
//         let health = provider.health_check().await.unwrap();
//         assert!(health.is_healthy);
//     }
// }
//
// // Default implementations for TodoUpdate
// impl Default for TodoUpdate {
//     fn default() -> Self {
//         Self {
//             title: None,
//             description: None,
//             completed: None,
//             priority: None,
//             category: None,
//             tags: None,
//             due_date: None,
//         }
//     }
// }