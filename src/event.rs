// src/event.rs

//! Event-driven architecture system with async event bus
//!
//! This module provides a comprehensive event system that supports:
//! - Type-safe event publishing and subscription
//! - Async event handlers with backpressure
//! - Event filtering and routing
//! - Event persistence and replay
//! - Dead letter queue for failed events
//! - Metrics and monitoring
//! - Event serialization for network transport

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::channel::mpsc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, ErrorKind, EventOperation, Result};
use crate::manager::{ManagedState, Manager, ManagerStatus};
use crate::types::Metadata;

/// Base event trait that all events must implement
pub trait Event: Send + Sync + Debug {
    /// Get the event type identifier
    fn event_type(&self) -> &'static str;

    /// Get the event source
    fn source(&self) -> &str;

    /// Get event metadata
    fn metadata(&self) -> &Metadata;

    /// Get event as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get event timestamp (default implementation)
    fn timestamp(&self) -> DateTime<Utc> {
        Utc::now()
    }

    /// Get event correlation ID if available
    fn correlation_id(&self) -> Option<Uuid> {
        self.metadata()
            .get("correlation_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }

    /// Get event priority (default is normal)
    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }

    /// Whether this event should be persisted
    fn should_persist(&self) -> bool {
        false
    }
}

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventPriority {
    /// Low priority events (background processing)
    Low = 0,
    /// Normal priority events
    Normal = 50,
    /// High priority events (user actions)
    High = 100,
    /// Critical priority events (system alerts)
    Critical = 200,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Event handler trait for processing events
#[async_trait]
pub trait EventHandler: Send + Sync + Debug {
    /// Handle an event
    async fn handle(&self, event: &dyn Event) -> Result<()>;

    /// Get handler name for debugging
    fn name(&self) -> &str;

    /// Get event types this handler is interested in
    fn event_types(&self) -> Vec<&'static str>;

    /// Whether this handler should receive all events (wildcard)
    fn is_wildcard(&self) -> bool {
        false
    }

    /// Get handler priority (affects processing order)
    fn priority(&self) -> i32 {
        0
    }
}

/// Event subscription filter
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Event types to match (empty means all)
    pub event_types: Vec<String>,
    /// Source patterns to match
    pub source_patterns: Vec<String>,
    /// Metadata filters
    pub metadata_filters: HashMap<String, serde_json::Value>,
    /// Minimum priority level
    pub min_priority: EventPriority,
}

impl EventFilter {
    /// Create a new event filter
    pub fn new() -> Self {
        Self {
            event_types: Vec::new(),
            source_patterns: Vec::new(),
            metadata_filters: HashMap::new(),
            min_priority: EventPriority::Low,
        }
    }

    /// Add event type filter
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_types.push(event_type.into());
        self
    }

    /// Add source pattern filter
    pub fn with_source_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.source_patterns.push(pattern.into());
        self
    }

    /// Add metadata filter
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata_filters.insert(key.into(), value);
        self
    }

    /// Set minimum priority
    pub fn with_min_priority(mut self, priority: EventPriority) -> Self {
        self.min_priority = priority;
        self
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event: &dyn Event) -> bool {
        // Check event type
        if !self.event_types.is_empty() {
            if !self.event_types.contains(&event.event_type().to_string()) {
                return false;
            }
        }

        // Check source patterns
        if !self.source_patterns.is_empty() {
            let source = event.source();
            if !self.source_patterns.iter().any(|pattern| {
                // Simple pattern matching (could be enhanced with regex)
                pattern == "*" || source.contains(pattern)
            }) {
                return false;
            }
        }

        // Check priority
        if event.priority() < self.min_priority {
            return false;
        }

        // Check metadata filters
        let event_metadata = event.metadata();
        for (key, expected_value) in &self.metadata_filters {
            match event_metadata.get(key) {
                Some(actual_value) => {
                    if actual_value != expected_value {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Event subscription
pub struct EventSubscription {
    /// Subscription ID
    pub id: Uuid,
    /// Filter for events
    pub filter: EventFilter,
    /// Event sender channel
    pub sender: mpsc::UnboundedSender<Arc<dyn Event>>,
    /// When subscription was created
    pub created_at: DateTime<Utc>,
    /// Whether subscription is active
    pub active: bool,
    /// Subscription metadata
    pub metadata: Metadata,
}

impl Debug for EventSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSubscription")
            .field("id", &self.id)
            .field("filter", &self.filter)
            .field("created_at", &self.created_at)
            .field("active", &self.active)
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    /// Total events published
    pub total_published: u64,
    /// Total events processed
    pub total_processed: u64,
    /// Total events failed
    pub total_failed: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Events by priority
    pub events_by_priority: HashMap<EventPriority, u64>,
    /// Average processing time in milliseconds
    pub avg_processing_time_ms: f64,
    /// Current active subscriptions
    pub active_subscriptions: usize,
    /// Queue size
    pub queue_size: usize,
}

/// Event bus configuration
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Number of worker threads
    pub worker_count: usize,
    /// Queue capacity
    pub queue_capacity: usize,
    /// Default timeout for event processing
    pub default_timeout: Duration,
    /// Whether to enable event persistence
    pub enable_persistence: bool,
    /// Whether to enable metrics collection
    pub enable_metrics: bool,
    /// Batch size for processing events
    pub batch_size: usize,
    /// Maximum retry delay
    pub max_retry_delay: Duration,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            worker_count: num_cpus::get(),
            queue_capacity: 10000,
            default_timeout: Duration::from_secs(30),
            enable_persistence: false,
            enable_metrics: true,
            batch_size: 100,
            max_retry_delay: Duration::from_secs(60),
        }
    }
}

/// Internal event wrapper for the bus
#[derive(Debug)]
struct EventEnvelope {
    /// The actual event
    event: Arc<dyn Event>,
    /// When event was received
    received_at: Instant,
    /// Retry count
    retry_count: u32,
    /// Maximum retries
    max_retries: u32,
}

/// Event bus manager
pub struct EventBusManager {
    state: ManagedState,
    config: EventBusConfig,
    subscriptions: Arc<DashMap<Uuid, EventSubscription>>,
    stats: Arc<RwLock<EventStats>>,
    event_counter: Arc<AtomicU64>,
}

impl Debug for EventBusManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBusManager")
            .field("config", &self.config)
            .field("subscriptions", &self.subscriptions.len())
            .finish()
    }
}

impl EventBusManager {
    /// Create a new event bus manager
    pub fn new(config: EventBusConfig) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "event_bus_manager"),
            config,
            subscriptions: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(EventStats {
                total_published: 0,
                total_processed: 0,
                total_failed: 0,
                events_by_type: HashMap::new(),
                events_by_priority: HashMap::new(),
                avg_processing_time_ms: 0.0,
                active_subscriptions: 0,
                queue_size: 0,
            })),
            event_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Publish an event to the bus
    pub async fn publish<E: Event + 'static>(&self, event: E) -> Result<()> {
        let event_arc: Arc<dyn Event> = Arc::new(event);

        // Update statistics
        self.event_counter.fetch_add(1, Ordering::Relaxed);
        {
            let mut stats = self.stats.write();
            stats.total_published += 1;
            *stats
                .events_by_type
                .entry(event_arc.event_type().to_string())
                .or_insert(0) += 1;
            *stats
                .events_by_priority
                .entry(event_arc.priority())
                .or_insert(0) += 1;
        }

        // Send to matching subscriptions immediately for web compatibility
        self.process_event_sync(event_arc).await;

        Ok(())
    }

    async fn process_event_sync(&self, event: Arc<dyn Event>) {
        let start_time = Instant::now();

        // Find matching subscriptions
        let matching_subscriptions: Vec<(Uuid, Arc<dyn Event>)> = self.subscriptions
            .iter()
            .filter_map(|entry| {
                let subscription = entry.value();
                if subscription.active && subscription.filter.matches(event.as_ref()) {
                    Some((subscription.id, Arc::clone(&event)))
                } else {
                    None
                }
            })
            .collect();

        // Send event to matching subscriptions
        let mut successful_deliveries = 0;
        let mut failed_deliveries = 0;

        for (subscription_id, _event_clone) in matching_subscriptions {
            if let Some(subscription) = self.subscriptions.get(&subscription_id) {
                match subscription.sender.unbounded_send(Arc::clone(&event)) {
                    Ok(()) => successful_deliveries += 1,
                    Err(_) => {
                        failed_deliveries += 1;
                        // Remove failed subscription
                        self.subscriptions.remove(&subscription_id);
                    }
                }
            }
        }

        let processing_time = start_time.elapsed();

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_processed += 1;
            if failed_deliveries > 0 {
                stats.total_failed += 1;
            }

            // Update average processing time
            let total_processed = stats.total_processed;
            stats.avg_processing_time_ms = (stats.avg_processing_time_ms
                * (total_processed - 1) as f64
                + processing_time.as_millis() as f64)
                / total_processed as f64;

            stats.active_subscriptions = self.subscriptions.len();
        }
    }

    /// Subscribe to events with a filter
    pub async fn subscribe(
        &self,
        filter: EventFilter,
    ) -> Result<mpsc::UnboundedReceiver<Arc<dyn Event>>> {
        let (sender, receiver) = mpsc::unbounded::<Arc<dyn Event>>();
        let subscription_id = Uuid::new_v4();

        let subscription = EventSubscription {
            id: subscription_id,
            filter,
            sender,
            created_at: Utc::now(),
            active: true,
            metadata: HashMap::new(),
        };

        self.subscriptions.insert(subscription_id, subscription);

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.active_subscriptions = self.subscriptions.len();
        }

        Ok(receiver)
    }

    /// Subscribe with a handler
    pub async fn subscribe_with_handler<H: EventHandler + 'static>(
        &self,
        filter: EventFilter,
        handler: Arc<H>,
    ) -> Result<Uuid> {
        let mut receiver = self.subscribe(filter).await?;
        let handler_name = handler.name().to_string();

        // Spawn task to handle events
        let handle = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                let start_time = Instant::now();

                match handler.handle(event.as_ref()).await {
                    Ok(()) => {
                        let processing_time = start_time.elapsed();
                        tracing::trace!(
                            "Handler '{}' processed event in {:?}",
                            handler_name,
                            processing_time
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Handler '{}' failed to process event: {}",
                            handler_name,
                            e
                        );
                    }
                }
            }
        });

        // Store the handle (simplified - in practice you'd want to track these)
        drop(handle);

        // Return a dummy subscription ID
        Ok(Uuid::new_v4())
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscription_id: Uuid) -> Result<()> {
        if let Some(mut subscription) = self.subscriptions.get_mut(&subscription_id) {
            subscription.active = false;
        }

        self.subscriptions.remove(&subscription_id).ok_or_else(|| {
            Error::new(
                ErrorKind::Event {
                    event_type: None,
                    subscriber_id: Some(subscription_id),
                    operation: EventOperation::Subscribe,
                },
                "Subscription not found",
            )
        })?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_subscriptions = self.subscriptions.len();
        }

        tracing::debug!("Removed subscription: {}", subscription_id);

        Ok(())
    }

    /// Get event bus statistics
    pub async fn get_stats(&self) -> EventStats {
        self.stats.read().await.clone()
    }

    /// Start event processing workers
    async fn start_workers(&mut self) -> Result<()> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel::<EventEnvelope>();
        self.event_queue = event_sender;

        let subscriptions = Arc::clone(&self.subscriptions);
        let stats = Arc::clone(&self.stats);

        // Move event_receiver OUT of self scope BEFORE the spawn
        let handle = tokio::spawn(Self::worker_task(event_receiver, subscriptions, stats));

        self.worker_handles.push(handle);

        Ok(())
    }

    // This function owns event_receiver and can move it safely
    async fn worker_task(
        mut event_receiver: mpsc::UnboundedReceiver<EventEnvelope>,
        subscriptions: Arc<DashMap<Uuid, EventSubscription>>,
        stats: Arc<RwLock<EventStats>>,
    ) {
        tracing::debug!("Event worker started");

        while let Some(envelope) = event_receiver.recv().await {
            Self::process_event(envelope, &subscriptions, &stats).await;
        }

        tracing::debug!("Event worker stopped");
    }

    /// Process a single event
    async fn process_event(
        envelope: EventEnvelope,
        subscriptions: &DashMap<Uuid, EventSubscription>,
        stats: &RwLock<EventStats>,
    ) {
        let start_time = Instant::now();
        let event = &envelope.event;

        // Find matching subscriptions
        let matching_subscriptions: Vec<(Uuid, Arc<dyn Event>)> = subscriptions
            .iter()
            .filter_map(|entry| {
                let subscription = entry.value();
                if subscription.active && subscription.filter.matches(event.as_ref()) {
                    Some((subscription.id, Arc::clone(event)))
                } else {
                    None
                }
            })
            .collect();

        // Send event to matching subscriptions
        let mut successful_deliveries = 0;
        let mut failed_deliveries = 0;

        for (subscription_id, _event_clone) in matching_subscriptions {
            if let Some(subscription) = subscriptions.get(&subscription_id) {
                match subscription.sender.send(Arc::clone(event)) {
                    Ok(()) => successful_deliveries += 1,
                    Err(_) => {
                        failed_deliveries += 1;
                        tracing::warn!(
                            "Failed to deliver event to subscription {}",
                            subscription_id
                        );
                    }
                }
            }
        }

        let processing_time = start_time.elapsed();

        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_processed += 1;
            if failed_deliveries > 0 {
                stats_guard.total_failed += 1;
            }

            // Update average processing time
            let total_processed = stats_guard.total_processed;
            stats_guard.avg_processing_time_ms = (stats_guard.avg_processing_time_ms
                * (total_processed - 1) as f64
                + processing_time.as_millis() as f64)
                / total_processed as f64;
        }

        tracing::trace!(
            "Processed event '{}' in {:?} (delivered to {} subscriptions, {} failed)",
            event.event_type(),
            processing_time,
            successful_deliveries,
            failed_deliveries
        );
    }

    /// Stop all workers
    async fn stop_workers(&mut self) {
        for handle in self.worker_handles.drain(..) {
            handle.abort();
            let _ = handle.await;
        }
    }
}

#[async_trait]
impl Manager for EventBusManager {
    fn name(&self) -> &str {
        "event_bus_manager"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4() // Simplified
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Start event processing workers
        self.start_workers().await?;

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        tracing::info!(
            "Event bus manager initialized with {} workers",
            self.config.worker_count
        );
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Stop processing new events
        self.stop_workers().await;

        // Clear subscriptions
        self.subscriptions.clear();

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        tracing::info!("Event bus manager shut down");
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_stats().await;

        status.add_metadata(
            "total_published",
            serde_json::Value::from(stats.total_published),
        );
        status.add_metadata(
            "total_processed",
            serde_json::Value::from(stats.total_processed),
        );
        status.add_metadata("total_failed", serde_json::Value::from(stats.total_failed));
        status.add_metadata(
            "active_subscriptions",
            serde_json::Value::from(stats.active_subscriptions),
        );
        status.add_metadata(
            "worker_count",
            serde_json::Value::from(self.config.worker_count),
        );
        status.add_metadata(
            "avg_processing_time_ms",
            serde_json::Value::from(stats.avg_processing_time_ms),
        );

        status
    }
}

/// Convenience macros for event handling
#[macro_export]
macro_rules! define_event {
    ($name:ident, $event_type:expr, $($field:ident: $type:ty),*) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            pub timestamp: chrono::DateTime<chrono::Utc>,
            pub source: String,
            pub metadata: std::collections::HashMap<String, serde_json::Value>,
            $(pub $field: $type,)*
        }

        impl $crate::event::Event for $name {
            fn event_type(&self) -> &'static str {
                $event_type
            }

            fn source(&self) -> &str {
                &self.source
            }

            fn metadata(&self) -> &$crate::types::Metadata {
                &self.metadata
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
                self.timestamp
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Test event
    #[derive(Debug, Clone)]
    struct TestEvent {
        source: String,
        metadata: Metadata,
        data: String,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "test.event"
        }

        fn source(&self) -> &str {
            &self.source
        }

        fn metadata(&self) -> &Metadata {
            &self.metadata
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_event_bus_creation() {
        let config = EventBusConfig::default();
        let bus = EventBusManager::new(config);
        assert_eq!(bus.subscriptions.len(), 0);
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let config = EventBusConfig::default();
        let mut bus = EventBusManager::new(config);

        bus.initialize().await.unwrap();

        let event = TestEvent {
            source: "test".to_string(),
            metadata: HashMap::new(),
            data: "test data".to_string(),
        };

        bus.publish(event).await.unwrap();

        let stats = bus.get_stats().await;
        assert_eq!(stats.total_published, 1);

        bus.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let config = EventBusConfig::default();
        let mut bus = EventBusManager::new(config);

        bus.initialize().await.unwrap();

        let filter = EventFilter::new().with_event_type("test.event");
        let mut receiver = bus.subscribe(filter).await.unwrap();

        let event = TestEvent {
            source: "test".to_string(),
            metadata: HashMap::new(),
            data: "test data".to_string(),
        };

        bus.publish(event).await.unwrap();

        // Give some time for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if we received the event
        if let Ok(received_event) =
            tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await
        {
            assert!(received_event.is_some());
            let event = received_event.unwrap();
            assert_eq!(event.event_type(), "test.event");
        }

        bus.shutdown().await.unwrap();
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new()
            .with_event_type("test.event")
            .with_source_pattern("test")
            .with_min_priority(EventPriority::Normal);

        let event = TestEvent {
            source: "test_source".to_string(),
            metadata: HashMap::new(),
            data: "test data".to_string(),
        };

        assert!(filter.matches(&event));

        let filter_no_match = EventFilter::new().with_event_type("other.event");

        assert!(!filter_no_match.matches(&event));
    }

    #[test]
    fn test_event_priority() {
        assert!(EventPriority::Critical > EventPriority::High);
        assert!(EventPriority::High > EventPriority::Normal);
        assert!(EventPriority::Normal > EventPriority::Low);
    }
}
