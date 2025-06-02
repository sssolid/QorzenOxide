// src/plugin/search.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Result;
use crate::types::Metadata;

/// Search query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search term
    pub query: String,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Filters to apply
    pub filters: HashMap<String, SearchFilter>,
    /// Facets to include in response
    pub facets: Vec<String>,
    /// Whether to include suggestions
    pub include_suggestions: bool,
    /// Search context (user, permissions, etc.)
    pub context: SearchContext,
}

/// Search filter value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchFilter {
    Exact(serde_json::Value),
    Range {
        min: Option<serde_json::Value>,
        max: Option<serde_json::Value>,
    },
    In(Vec<serde_json::Value>),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

/// Search context providing user and permission information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    pub user_id: Option<Uuid>,
    pub permissions: Vec<String>,
    pub preferences: HashMap<String, serde_json::Value>,
    pub metadata: Metadata,
}

/// Search result from a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier for this result
    pub id: String,
    /// Result type (e.g., "product", "document", "user")
    pub result_type: String,
    /// Display title
    pub title: String,
    /// Description or snippet
    pub description: Option<String>,
    /// Relevance score (0.0 to 1.0)
    pub score: f64,
    /// URL or route to access this item
    pub url: Option<String>,
    /// Thumbnail image URL
    pub thumbnail: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Source plugin that provided this result
    pub source_plugin: String,
    /// When this result was created/updated
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Facet value for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: serde_json::Value,
    pub display_name: String,
    pub count: usize,
}

/// Search facet for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacet {
    pub field: String,
    pub name: String,
    pub values: Vec<FacetValue>,
}

/// Search suggestions for autocomplete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub completion: String,
    pub category: Option<String>,
    pub score: f64,
}

/// Complete search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: usize,
    pub facets: Vec<SearchFacet>,
    pub suggestions: Vec<SearchSuggestion>,
    pub query_time_ms: u64,
    pub sources: Vec<String>,
}

/// Plugin search provider trait
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[allow(dead_code)]
pub trait SearchProvider: Send + Sync + std::fmt::Debug {
    /// Unique identifier for this search provider
    fn provider_id(&self) -> &str;

    /// Human-readable name
    fn provider_name(&self) -> &str;

    /// Description of what this provider searches
    fn description(&self) -> &str;

    /// Priority for this provider (higher = more important)
    fn priority(&self) -> i32;

    /// Result types this provider can return
    fn supported_result_types(&self) -> Vec<String>;

    /// Whether this provider supports faceted search
    fn supports_facets(&self) -> bool;

    /// Whether this provider supports autocomplete suggestions
    fn supports_suggestions(&self) -> bool;

    /// Perform a search
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;

    /// Get facets for a query (if supported)
    async fn get_facets(&self, query: &SearchQuery) -> Result<Vec<SearchFacet>> {
        let _ = query;
        Ok(vec![])
    }

    /// Get autocomplete suggestions (if supported)
    async fn get_suggestions(&self, query: &SearchQuery) -> Result<Vec<SearchSuggestion>> {
        let _ = query;
        Ok(vec![])
    }

    /// Index new content (for indexing-based providers)
    async fn index_content(&self, content: &IndexableContent) -> Result<()> {
        let _ = content;
        Ok(())
    }

    /// Remove content from index
    async fn remove_content(&self, content_id: &str) -> Result<()> {
        let _ = content_id;
        Ok(())
    }

    /// Health check for the search provider
    async fn health_check(&self) -> Result<ProviderHealth>;
}

/// Content that can be indexed by search providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexableContent {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub body: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub permissions: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Health status of a search provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub is_healthy: bool,
    pub response_time_ms: Option<u64>,
    pub error_message: Option<String>,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Central search coordinator that manages all search providers
#[derive(Debug)]
#[allow(dead_code)]
pub struct SearchCoordinator {
    providers: Arc<RwLock<HashMap<String, Arc<dyn SearchProvider>>>>,
    provider_health: Arc<RwLock<HashMap<String, ProviderHealth>>>,
}

#[allow(dead_code)]
impl SearchCoordinator {
    /// Create a new search coordinator
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            provider_health: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a search provider from a plugin
    pub async fn register_provider(&self, provider: Arc<dyn SearchProvider>) -> Result<()> {
        let provider_id = provider.provider_id().to_string();

        // Validate the provider
        let health = provider.health_check().await.unwrap_or(ProviderHealth {
            is_healthy: false,
            response_time_ms: None,
            error_message: Some("Health check failed".to_string()),
            last_check: chrono::Utc::now(),
        });

        // Register the provider
        self.providers
            .write()
            .await
            .insert(provider_id.clone(), provider);
        self.provider_health
            .write()
            .await
            .insert(provider_id, health);

        Ok(())
    }

    /// Unregister a search provider
    pub async fn unregister_provider(&self, provider_id: &str) -> Result<()> {
        self.providers.write().await.remove(provider_id);
        self.provider_health.write().await.remove(provider_id);
        Ok(())
    }

    /// Perform a federated search across all providers
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResponse> {
        let start_time = std::time::Instant::now();
        let providers = self.providers.read().await;

        let mut all_results = Vec::new();
        let mut all_facets = Vec::new();
        let mut all_suggestions = Vec::new();
        let mut sources = Vec::new();

        // Search all providers concurrently
        let search_tasks: Vec<_> = providers
            .values()
            .map(|provider| {
                let provider = Arc::clone(provider);
                let query = query.clone();
                async move {
                    let provider_id = provider.provider_id().to_string();
                    match provider.search(&query).await {
                        Ok(results) => Some((provider_id, results)),
                        Err(e) => {
                            tracing::warn!(
                                "Search provider {} failed: {}",
                                provider.provider_id(),
                                e
                            );
                            None
                        }
                    }
                }
            })
            .collect();

        // Execute all searches
        let search_results = futures::future::join_all(search_tasks).await;

        // Collect results
        for (provider_id, mut results) in search_results.into_iter().flatten() {
            sources.push(provider_id);
            all_results.append(&mut results);
        }

        // Get facets if requested
        if !query.facets.is_empty() {
            let facet_tasks: Vec<_> = providers
                .values()
                .filter(|p| p.supports_facets())
                .map(|provider| {
                    let provider = Arc::clone(provider);
                    let query = query.clone();
                    async move { provider.get_facets(&query).await.unwrap_or_default() }
                })
                .collect();

            let facet_results = futures::future::join_all(facet_tasks).await;
            for mut facets in facet_results {
                all_facets.append(&mut facets);
            }
        }

        // Get suggestions if requested
        if query.include_suggestions {
            let suggestion_tasks: Vec<_> = providers
                .values()
                .filter(|p| p.supports_suggestions())
                .map(|provider| {
                    let provider = Arc::clone(provider);
                    let query = query.clone();
                    async move { provider.get_suggestions(&query).await.unwrap_or_default() }
                })
                .collect();

            let suggestion_results = futures::future::join_all(suggestion_tasks).await;
            for mut suggestions in suggestion_results {
                all_suggestions.append(&mut suggestions);
            }
        }

        // Sort results by score (descending)
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply pagination
        let total_count = all_results.len();
        if let (Some(offset), Some(limit)) = (query.offset, query.limit) {
            all_results = all_results.into_iter().skip(offset).take(limit).collect();
        } else if let Some(limit) = query.limit {
            all_results.truncate(limit);
        }

        // Sort suggestions by score
        all_suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_suggestions.truncate(10); // Limit suggestions

        let query_time = start_time.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            results: all_results,
            total_count,
            facets: all_facets,
            suggestions: all_suggestions,
            query_time_ms: query_time,
            sources,
        })
    }

    /// Index content across relevant providers
    pub async fn index_content(&self, content: IndexableContent) -> Result<()> {
        let providers = self.providers.read().await;

        let index_tasks: Vec<_> = providers
            .values()
            .map(|provider| {
                let provider = Arc::clone(provider);
                let content = content.clone();
                async move {
                    if let Err(e) = provider.index_content(&content).await {
                        tracing::warn!(
                            "Failed to index content in provider {}: {}",
                            provider.provider_id(),
                            e
                        );
                    }
                }
            })
            .collect();

        futures::future::join_all(index_tasks).await;
        Ok(())
    }

    /// Remove content from all provider indices
    pub async fn remove_content(&self, content_id: &str) -> Result<()> {
        let providers = self.providers.read().await;

        let remove_tasks: Vec<_> = providers
            .values()
            .map(|provider| {
                let provider = Arc::clone(provider);
                let content_id = content_id.to_string();
                async move {
                    if let Err(e) = provider.remove_content(&content_id).await {
                        tracing::warn!(
                            "Failed to remove content from provider {}: {}",
                            provider.provider_id(),
                            e
                        );
                    }
                }
            })
            .collect();

        futures::future::join_all(remove_tasks).await;
        Ok(())
    }

    /// Get health status of all providers
    pub async fn get_provider_health(&self) -> HashMap<String, ProviderHealth> {
        self.provider_health.read().await.clone()
    }

    /// Update provider health status
    pub async fn update_provider_health(&self, provider_id: &str, health: ProviderHealth) {
        self.provider_health
            .write()
            .await
            .insert(provider_id.to_string(), health);
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }
}

impl Default for SearchCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Example search provider implementation
#[derive(Debug)]
pub struct ExampleSearchProvider {
    id: String,
    name: String,
    // In a real implementation, this might connect to a database or search engine
    indexed_content: Arc<RwLock<Vec<IndexableContent>>>,
}

impl ExampleSearchProvider {
    /// Create a new example search provider
    #[allow(dead_code)]
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            indexed_content: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl SearchProvider for ExampleSearchProvider {
    fn provider_id(&self) -> &str {
        &self.id
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Example search provider for demonstration"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn supported_result_types(&self) -> Vec<String> {
        vec!["example".to_string()]
    }

    fn supports_facets(&self) -> bool {
        false
    }

    fn supports_suggestions(&self) -> bool {
        true
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let content = self.indexed_content.read().await;
        let query_lower = query.query.to_lowercase();

        let mut results = Vec::new();

        for item in content.iter() {
            let title_match = item.title.to_lowercase().contains(&query_lower);
            let body_match = item
                .body
                .as_ref()
                .map(|b| b.to_lowercase().contains(&query_lower))
                .unwrap_or(false);

            if title_match || body_match {
                let score = if title_match { 0.9 } else { 0.6 };

                results.push(SearchResult {
                    id: item.id.clone(),
                    result_type: item.content_type.clone(),
                    title: item.title.clone(),
                    description: item.body.clone(),
                    score,
                    url: Some(format!("/content/{}", item.id)),
                    thumbnail: None,
                    metadata: item.metadata.clone(),
                    source_plugin: self.id.clone(),
                    timestamp: item.updated_at,
                });
            }
        }

        Ok(results)
    }

    async fn get_suggestions(&self, query: &SearchQuery) -> Result<Vec<SearchSuggestion>> {
        let content = self.indexed_content.read().await;
        let query_lower = query.query.to_lowercase();

        let mut suggestions = Vec::new();

        for item in content.iter() {
            if item.title.to_lowercase().starts_with(&query_lower) {
                suggestions.push(SearchSuggestion {
                    text: query.query.clone(),
                    completion: item.title.clone(),
                    category: Some(item.content_type.clone()),
                    score: 0.8,
                });
            }
        }

        suggestions.truncate(5);
        Ok(suggestions)
    }

    async fn index_content(&self, content: &IndexableContent) -> Result<()> {
        let mut indexed = self.indexed_content.write().await;

        // Remove existing content with same ID
        indexed.retain(|item| item.id != content.id);

        // Add new content
        indexed.push(content.clone());

        Ok(())
    }

    async fn remove_content(&self, content_id: &str) -> Result<()> {
        let mut indexed = self.indexed_content.write().await;
        indexed.retain(|item| item.id != content_id);
        Ok(())
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        Ok(ProviderHealth {
            is_healthy: true,
            response_time_ms: Some(10),
            error_message: None,
            last_check: chrono::Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_coordinator() {
        let coordinator = SearchCoordinator::new();
        let provider = Arc::new(ExampleSearchProvider::new(
            "test".to_string(),
            "Test Provider".to_string(),
        ));

        coordinator
            .register_provider(provider.clone())
            .await
            .unwrap();

        // Index some content
        let content = IndexableContent {
            id: "1".to_string(),
            content_type: "example".to_string(),
            title: "Test Document".to_string(),
            body: Some("This is a test document".to_string()),
            metadata: HashMap::new(),
            permissions: vec![],
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        coordinator.index_content(content).await.unwrap();

        // Perform search
        let query = SearchQuery {
            query: "test".to_string(),
            limit: Some(10),
            offset: None,
            filters: HashMap::new(),
            facets: vec![],
            include_suggestions: true,
            context: SearchContext {
                user_id: None,
                permissions: vec![],
                preferences: HashMap::new(),
                metadata: HashMap::new(),
            },
        };

        let response = coordinator.search(query).await.unwrap();
        assert!(!response.results.is_empty());
        assert_eq!(response.results[0].title, "Test Document");
    }

    #[tokio::test]
    async fn test_search_provider_suggestions() {
        let provider = ExampleSearchProvider::new("test".to_string(), "Test Provider".to_string());

        let content = IndexableContent {
            id: "1".to_string(),
            content_type: "document".to_string(),
            title: "Test Document".to_string(),
            body: Some("Content".to_string()),
            metadata: HashMap::new(),
            permissions: vec![],
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        provider.index_content(&content).await.unwrap();

        let query = SearchQuery {
            query: "Test".to_string(),
            limit: Some(5),
            offset: None,
            filters: HashMap::new(),
            facets: vec![],
            include_suggestions: true,
            context: SearchContext {
                user_id: None,
                permissions: vec![],
                preferences: HashMap::new(),
                metadata: HashMap::new(),
            },
        };

        let suggestions = provider.get_suggestions(&query).await.unwrap();
        assert!(!suggestions.is_empty());
    }
}
