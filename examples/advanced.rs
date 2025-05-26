// examples/advanced.rs

//! Advanced example demonstrating plugin-like patterns with Qorzen Core
//!
//! This example shows how to:
//! - Create custom managers
//! - Implement plugin-like behavior
//! - Use events for inter-component communication
//! - Handle complex async workflows
//!
//! Run with: cargo run --example advanced

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use uuid::Uuid;

use qorzen_oxide::{
    app::ApplicationCore,
    error::{Error, ErrorKind, Result},
    event::{Event, EventFilter},
    manager::{HealthStatus, Manager, ManagedState, ManagerStatus},
    task::{TaskBuilder, TaskCategory, TaskPriority},
    types::Metadata,
};

#[derive(Parser, Debug)]
#[command(name = "advanced")]
#[command(about = "Advanced Qorzen Example", long_about = None)]
struct Args {
    #[arg(long, default_value = "config.yaml")]
    config: String,
}

#[derive(Debug)]
struct DataProcessingService {
    state: ManagedState,
    config: DataProcessingConfig,
    stats: Arc<RwLock<ProcessingStats>>,
    processed_items: Arc<RwLock<Vec<ProcessedItem>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataProcessingConfig {
    batch_size: usize,
    processing_interval_ms: u64,
    max_items: usize,
    enable_validation: bool,
}

impl Default for DataProcessingConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            processing_interval_ms: 1000,
            max_items: 1000,
            enable_validation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessingStats {
    total_processed: u64,
    total_errors: u64,
    processing_rate_per_second: f64,
    last_batch_time: chrono::DateTime<chrono::Utc>,
}

impl Default for ProcessingStats {
    fn default() -> Self {
        Self {
            total_processed: 0,
            total_errors: 0,
            processing_rate_per_second: 0.0,
            last_batch_time: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessedItem {
    id: String,
    data: String,
    processed_at: chrono::DateTime<chrono::Utc>,
    processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataReceivedEvent {
    batch_id: String,
    item_count: usize,
    source: String,
    metadata: Metadata,
}

impl Event for DataReceivedEvent {
    fn event_type(&self) -> &'static str {
        "data.received"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataProcessedEvent {
    batch_id: String,
    processed_count: usize,
    error_count: usize,
    processing_time_ms: u64,
    source: String,
    metadata: Metadata,
}

impl Event for DataProcessedEvent {
    fn event_type(&self) -> &'static str {
        "data.processed"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DataProcessingService {
    fn new(config: DataProcessingConfig) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "data_processing_service"),
            config,
            stats: Arc::new(RwLock::new(ProcessingStats::default())),
            processed_items: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn start_processing_loop(&self) -> Result<()> {
        let stats = Arc::clone(&self.stats);
        let processed_items = Arc::clone(&self.processed_items);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.processing_interval_ms));

            loop {
                interval.tick().await;

                // Simulate receiving data
                let batch_id = Uuid::new_v4().to_string();
                let item_count = rand::random::<usize>() % config.batch_size + 1;

                // Process the batch
                let start_time = std::time::Instant::now();
                let mut processed_count = 0;
                let mut error_count = 0;

                for i in 0..item_count {
                    let item_id = format!("{}-{}", batch_id, i);

                    // Simulate processing time
                    sleep(Duration::from_millis(10)).await;

                    // Simulate occasional errors
                    if rand::random::<f64>() < 0.1 {
                        error_count += 1;
                        continue;
                    }

                    let processed_item = ProcessedItem {
                        id: item_id,
                        data: format!("processed_data_{}", i),
                        processed_at: chrono::Utc::now(),
                        processing_time_ms: 10,
                    };

                    // Store processed item
                    {
                        let mut items = processed_items.write().await;
                        if items.len() >= config.max_items {
                            items.remove(0); // Remove oldest item
                        }
                        items.push(processed_item);
                    }

                    processed_count += 1;
                }

                let processing_time = start_time.elapsed();

                // Update statistics
                {
                    let mut stats_guard = stats.write().await;
                    stats_guard.total_processed += processed_count as u64;
                    stats_guard.total_errors += error_count as u64;
                    stats_guard.last_batch_time = chrono::Utc::now();

                    // Calculate processing rate
                    if processing_time.as_secs_f64() > 0.0 {
                        stats_guard.processing_rate_per_second =
                            processed_count as f64 / processing_time.as_secs_f64();
                    }
                }

                tracing::info!(
                    "Processed batch {}: {}/{} items in {:?}",
                    batch_id, processed_count, item_count, processing_time
                );

                // In a real implementation, you'd publish events here
                // event_bus.publish(DataProcessedEvent { ... }).await?;
            }
        });

        Ok(())
    }

    async fn get_stats(&self) -> ProcessingStats {
        self.stats.read().await.clone()
    }

    async fn get_processed_items(&self, limit: Option<usize>) -> Vec<ProcessedItem> {
        let items = self.processed_items.read().await;
        let limit = limit.unwrap_or(items.len());
        items.iter().rev().take(limit).cloned().collect()
    }
}

#[async_trait]
impl Manager for DataProcessingService {
    fn name(&self) -> &str {
        "data_processing_service"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state.set_state(qorzen_oxide::manager::ManagerState::Initializing).await;

        tracing::info!("Initializing Data Processing Service");
        tracing::info!("  Batch size: {}", self.config.batch_size);
        tracing::info!("  Processing interval: {}ms", self.config.processing_interval_ms);
        tracing::info!("  Max items: {}", self.config.max_items);

        // Start the processing loop
        self.start_processing_loop().await?;

        self.state.set_state(qorzen_oxide::manager::ManagerState::Running).await;
        tracing::info!("Data Processing Service initialized successfully");

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(qorzen_oxide::manager::ManagerState::ShuttingDown).await;

        tracing::info!("Shutting down Data Processing Service");

        // In a real implementation, you'd stop the processing loop gracefully

        self.state.set_state(qorzen_oxide::manager::ManagerState::Shutdown).await;
        tracing::info!("Data Processing Service shutdown complete");

        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let stats = self.get_stats().await;
        let item_count = self.processed_items.read().await.len();

        status.add_metadata("total_processed", serde_json::Value::from(stats.total_processed));
        status.add_metadata("total_errors", serde_json::Value::from(stats.total_errors));
        status.add_metadata("processing_rate", serde_json::Value::from(stats.processing_rate_per_second));
        status.add_metadata("items_in_memory", serde_json::Value::from(item_count));
        status.add_metadata("batch_size", serde_json::Value::from(self.config.batch_size));

        status
    }

    async fn health_check(&self) -> HealthStatus {
        let stats = self.get_stats().await;
        let last_batch_age = chrono::Utc::now() - stats.last_batch_time;

        // Consider service unhealthy if no processing for more than 10 seconds
        if last_batch_age.num_seconds() > 10 {
            HealthStatus::Unhealthy
        } else if stats.total_errors > 0 && stats.total_errors as f64 / stats.total_processed as f64 > 0.2 {
            // Degraded if error rate > 20%
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

#[derive(Debug)]
struct MonitoringService {
    state: ManagedState,
    alerts: Arc<RwLock<Vec<Alert>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Alert {
    id: String,
    level: AlertLevel,
    message: String,
    service: String,
    created_at: chrono::DateTime<chrono::Utc>,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl MonitoringService {
    fn new() -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "monitoring_service"),
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn create_alert(&self, level: AlertLevel, message: String, service: String) {
        let alert = Alert {
            id: Uuid::new_v4().to_string(),
            level,
            message: message.clone(),
            service: service.clone(),
            created_at: chrono::Utc::now(),
            resolved_at: None,
        };

        self.alerts.write().await.push(alert);
        tracing::warn!("🚨 Alert created for {}: {}", service, message);
    }

    async fn get_active_alerts(&self) -> Vec<Alert> {
        self.alerts
            .read()
            .await
            .iter()
            .filter(|alert| alert.resolved_at.is_none())
            .cloned()
            .collect()
    }

    async fn start_monitoring(&self, app: Arc<ApplicationCore>) -> Result<()> {
        let alerts = Arc::clone(&self.alerts);
        let monitoring_service = self.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Check application health
                let health = app.get_health().await;

                match health.status {
                    HealthStatus::Degraded => {
                        monitoring_service.create_alert(
                            AlertLevel::Warning,
                            "Application health is degraded".to_string(),
                            "application".to_string(),
                        ).await;
                    }
                    HealthStatus::Unhealthy => {
                        monitoring_service.create_alert(
                            AlertLevel::Critical,
                            "Application health is unhealthy".to_string(),
                            "application".to_string(),
                        ).await;
                    }
                    _ => {}
                }

                // Check individual managers
                for (manager_name, manager_health) in &health.managers {
                    match manager_health {
                        HealthStatus::Degraded => {
                            monitoring_service.create_alert(
                                AlertLevel::Warning,
                                format!("Manager {} is degraded", manager_name),
                                manager_name.clone(),
                            ).await;
                        }
                        HealthStatus::Unhealthy => {
                            monitoring_service.create_alert(
                                AlertLevel::Error,
                                format!("Manager {} is unhealthy", manager_name),
                                manager_name.clone(),
                            ).await;
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(())
    }
}

impl Clone for MonitoringService {
    fn clone(&self) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "monitoring_service"),
            alerts: Arc::clone(&self.alerts),
        }
    }
}

#[async_trait]
impl Manager for MonitoringService {
    fn name(&self) -> &str {
        "monitoring_service"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4()
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state.set_state(qorzen_oxide::manager::ManagerState::Initializing).await;
        tracing::info!("Monitoring Service initialized");
        self.state.set_state(qorzen_oxide::manager::ManagerState::Running).await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state.set_state(qorzen_oxide::manager::ManagerState::ShuttingDown).await;
        tracing::info!("Monitoring Service shutdown");
        self.state.set_state(qorzen_oxide::manager::ManagerState::Shutdown).await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;
        let alert_count = self.alerts.read().await.len();
        let active_alerts = self.get_active_alerts().await.len();

        status.add_metadata("total_alerts", serde_json::Value::from(alert_count));
        status.add_metadata("active_alerts", serde_json::Value::from(active_alerts));

        status
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("🚀 Starting Qorzen Core Advanced Example");

    // Initialize the application with config file (same as basic example)
    let mut app = ApplicationCore::with_config_file(&args.config);
    app.initialize().await?;

    let app_arc = Arc::new(app);

    println!("✅ Application initialized successfully");

    // Get the configuration
    let config = app_arc.get_config().await?;
    println!("📝 Using configuration:");
    println!("   App name: {}", config.app.name);
    println!("   Environment: {}", config.app.environment);
    println!("   Debug mode: {}", config.app.debug);

    // Create and initialize custom services with proper error handling
    let mut data_service = DataProcessingService::new(DataProcessingConfig {
        batch_size: 5,
        processing_interval_ms: 2000,
        max_items: 50,
        enable_validation: true,
    });

    let mut monitoring_service = MonitoringService::new();

    // Initialize services
    println!("🔧 Initializing custom services...");

    data_service.initialize().await?;
    monitoring_service.initialize().await?;
    monitoring_service.start_monitoring(Arc::clone(&app_arc)).await?;

    println!("✅ Custom services initialized");

    // Demonstrate complex workflow
    println!("\n🔄 Running complex workflow demonstration...");

    // Simulate running for a period to see processing and monitoring
    for i in 1..=10 {
        display_monitoring_status(
            &monitoring_service,
            i,
            &data_service
        ).await?;

        if i % 3 == 0 {
            demonstrate_complex_task_workflow().await?;
        }
    }

    // Show final statistics
    println!("\n📈 Final Statistics:");

    let data_stats = data_service.get_stats().await;
    println!("  Data Processing:");
    println!("    Total processed: {}", data_stats.total_processed);
    println!("    Total errors: {}", data_stats.total_errors);
    println!("    Processing rate: {:.2}/sec", data_stats.processing_rate_per_second);

    show_final_monitoring_summary(&monitoring_service).await?;

    let app_health = app_arc.get_health().await;
    println!("  Application Health: {:?}", app_health.status);

    // Show application stats
    let app_stats = app_arc.get_stats().await;
    println!("  Application Stats:");
    println!("    Version: {}", app_stats.version);
    println!("    Uptime: {:?}", app_stats.uptime);
    println!("    Managers: {}/{}", app_stats.initialized_managers, app_stats.manager_count);

    // Shutdown services
    println!("\n🛑 Shutting down services...");
    data_service.shutdown().await?;
    monitoring_service.shutdown().await?;

    // Note: We can't shutdown app_arc directly since it's an Arc
    // In a real implementation, you'd handle this more elegantly
    println!("✅ Advanced example completed successfully");

    Ok(())
}

async fn display_monitoring_status(monitoring_service: &MonitoringService, i: u32, data_service: &DataProcessingService) -> Result<()> {
    sleep(Duration::from_secs(3)).await;

    let stats = data_service.get_stats().await;
    let alerts = monitoring_service.get_active_alerts().await;

    println!(
        "📊 Iteration {}: Processed {} items, {} errors, {} active alerts",
        i, stats.total_processed, stats.total_errors, alerts.len()
    );

    let recent_items = data_service.get_processed_items(Some(3)).await;
    for item in recent_items {
        println!(
            "   📦 Item {}: {} ({}ms)",
            item.id, item.data, item.processing_time_ms
        );
    }

    for alert in alerts {
        println!("   🚨 Alert: {:?} - {}", alert.level, alert.message);
    }

    Ok(())
}

async fn show_final_monitoring_summary(monitoring_service: &MonitoringService) -> Result<()> {
    let all_alerts = monitoring_service.alerts.read().await;
    println!("  Monitoring:");
    println!("    Total alerts: {}", all_alerts.len());
    println!(
        "    Active alerts: {}",
        monitoring_service.get_active_alerts().await.len()
    );
    Ok(())
}

async fn demonstrate_complex_task_workflow() -> Result<()> {
    println!("  🎯 Executing complex task workflow...");

    // Create a new task manager instance for this workflow
    let config = qorzen_oxide::r#mod::TaskConfig::default();
    let mut task_manager = qorzen_oxide::task::TaskManager::new(config);
    task_manager.initialize().await?;

    // Create a workflow with dependencies
    let data_fetch_task = TaskBuilder::new("fetch_data")
        .category(TaskCategory::Io)
        .priority(TaskPriority::High)
        .build(|ctx| async move {
            ctx.report_percent(0, "Starting data fetch");

            // Simulate API call
            sleep(Duration::from_millis(100)).await;
            ctx.report_percent(50, "Fetching from API");

            sleep(Duration::from_millis(100)).await;
            ctx.report_percent(100, "Data fetch complete");

            Ok(serde_json::json!({
                "records": 150,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        });

    let data_process_task = TaskBuilder::new("process_data")
        .category(TaskCategory::Core)
        .priority(TaskPriority::Normal)
        .build(|ctx| async move {
            ctx.report_percent(0, "Starting data processing");

            for i in 1..=5 {
                sleep(Duration::from_millis(50)).await;
                ctx.report_step(i, 5, format!("Processing chunk {}", i));
            }

            ctx.report_percent(100, "Data processing complete");

            Ok(serde_json::json!({
                "processed_records": 150,
                "validation_errors": 3,
                "processing_time_ms": 250
            }))
        });

    // Submit and track tasks
    let fetch_task_id = task_manager.submit_task(data_fetch_task).await?;
    let process_task_id = task_manager.submit_task(data_process_task).await?;

    // Wait for completion
    let fetch_result = task_manager.wait_for_task(fetch_task_id, Some(Duration::from_secs(2))).await?;
    let process_result = task_manager.wait_for_task(process_task_id, Some(Duration::from_secs(2))).await?;

    println!("     ✓ Fetch task: {:?}", fetch_result.status);
    println!("     ✓ Process task: {:?}", process_result.status);

    task_manager.shutdown().await?;
    Ok(())
}