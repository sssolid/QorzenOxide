// examples/basic.rs

//! Basic example demonstrating Qorzen Core usage
//!
//! This example shows how to:
//! - Initialize the application
//! - Use various managers
//! - Handle graceful shutdown
//!
//! Run with: cargo run --example basic

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use qorzen_oxide::{app::ApplicationCore, error::Result, event::{Event}, file::{FileOperationOptions}, task::{TaskBuilder, TaskCategory, TaskPriority}, types::Metadata, Error, ErrorKind, Manager};

use clap::Parser;
use qorzen_oxide::r#mod::AppConfig;

#[derive(Parser, Debug)]
#[command(name = "basic")]
#[command(about = "Basic Qorzen Example", long_about = None)]
struct Args {
    #[arg(long, default_value = "config.yaml")]
    config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderProcessedEvent {
    order_id: String,
    customer_id: String,
    amount: f64,
    source: String,
    metadata: Metadata,
}

impl Event for OrderProcessedEvent {
    fn event_type(&self) -> &'static str {
        "order.processed"
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
struct UserLoginEvent {
    user_id: String,
    username: String,
    login_time: chrono::DateTime<chrono::Utc>,
    source: String,
    metadata: Metadata,
}

impl Event for UserLoginEvent {
    fn event_type(&self) -> &'static str {
        "user.login"
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    // Initialize the application
    println!("🚀 Starting Qorzen Core Basic Example");

    let mut app = ApplicationCore::with_config_file(&args.config);
    app.initialize().await?;
    let config = app.get_config().await?;

    println!("✅ Application initialized successfully");

    // Show application stats
    let stats = app.get_stats().await;
    println!(
        "📊 Application Stats:\n   Version: {}\n   Uptime: {:?}\n   Managers: {}/{}",
        stats.version,
        stats.uptime,
        stats.initialized_managers,
        stats.manager_count
    );

    // Demonstrate various functionalities
    println!("\n🔧 Testing core functionality...");

    // Test configuration access
    demonstrate_configuration(&config).await?;

    // Test file operations
    demonstrate_file_operations().await?;

    // Test task management
    demonstrate_task_management().await?;

    // Test concurrency
    demonstrate_concurrency().await?;

    // Test error handling
    demonstrate_error_handling().await?;

    // Show final health status
    let health = app.get_health().await;
    println!(
        "\n💚 Final Health Status: {:?}",
        health.status
    );

    // Graceful shutdown
    println!("\n🛑 Shutting down application...");
    app.shutdown().await?;
    println!("✅ Application shutdown complete");

    Ok(())
}

async fn demonstrate_configuration(config: &AppConfig) -> Result<()> {
    println!("  📝 Configuration Management:");

    println!("     App Name: {}", config.app.name);
    println!("     Environment: {}", config.app.environment);
    println!("     Debug Mode: {}", config.app.debug);

    Ok(())
}

async fn demonstrate_file_operations() -> Result<()> {
    println!("  📁 File Management:");

    // Create a temporary file manager for demonstration
    let config = qorzen_oxide::r#mod::FileConfig::default();
    let mut file_manager = qorzen_oxide::file::FileManager::new(config);
    file_manager.initialize().await?;

    // Write a sample configuration file
    let sample_config = serde_json::json!({
        "service_name": "example_service",
        "version": "1.0.0",
        "features": {
            "logging": true,
            "metrics": true
        }
    });

    let json = serde_json::to_string_pretty(&sample_config)
        .map_err(|e| Error::new(ErrorKind::Serialization(e.to_string()), "Failed to serialize sample config"))?;

    file_manager.write_file(
        "config/service.json",
        json.as_bytes(),
        Option::from(FileOperationOptions {
            create_parents: true,
            ..Default::default()
        }),
    ).await?;

    // Read it back
    let content = file_manager.read_file("config/service.json").await?;
    println!("     ✓ Written and read configuration file");
    println!("     ✓ Content length: {} bytes", content.len());

    // Get file info
    let file_info = file_manager.get_metadata("config/service.json").await?;
    println!("     ✓ File size: {}", file_info.size);

    // Create a copy
    let copy_path = file_manager.copy_file(
        "config/service.json",
        "config/service_backup.json",
        Option::from(FileOperationOptions {
            create_parents: true,
            ..Default::default()
        }),
    ).await?;
    println!("     ✓ Copied bytes: {}", copy_path);

    // Clean up config files
    file_manager.delete_file("config/service.json").await?;
    file_manager.delete_file("config/service_backup.json").await?;

    file_manager.shutdown().await?;
    Ok(())
}

async fn demonstrate_task_management() -> Result<()> {
    println!("  ⚡ Task Management:");

    let config = qorzen_oxide::r#mod::TaskConfig::default();
    let mut task_manager = qorzen_oxide::task::TaskManager::new(config);
    task_manager.initialize().await?;

    // Create multiple tasks with different priorities
    let mut task_ids = Vec::new();

    let counter = Arc::new(AtomicU32::new(0));

    // High priority task
    let high_counter = Arc::clone(&counter);
    let high_priority_task = TaskBuilder::new("high_priority_calculation")
        .category(TaskCategory::Core)
        .priority(TaskPriority::High)
        .timeout(Duration::from_secs(30)) // Increased timeout
        .build(move |ctx| {
            let counter = Arc::clone(&high_counter);
            async move {
                ctx.report_percent(0, "Starting high priority calculation");

                for i in 1..=5 {
                    counter.fetch_add(1, Ordering::SeqCst);
                    ctx.report_percent(i * 20, format!("Processing step {}/5", i));
                    sleep(Duration::from_millis(50)).await;
                }

                ctx.report_percent(100, "High priority calculation complete");
                Ok(serde_json::Value::String("High priority result".to_string()))
            }
        });

    task_ids.push(task_manager.submit_task(high_priority_task).await?);

    // Background processing task
    let background_counter = Arc::clone(&counter);
    let background_task = TaskBuilder::new("background_processing")
        .category(TaskCategory::Background)
        .priority(TaskPriority::Low)
        .timeout(Duration::from_secs(30)) // Increased timeout
        .build(move |ctx| {
            let counter = Arc::clone(&background_counter);
            async move {
                ctx.report_percent(0, "Starting background processing");

                for i in 1..=10 {
                    if ctx.is_cancelled() {
                        return Err(qorzen_oxide::error::Error::task(
                            Some(ctx.task_id),
                            "Task was cancelled",
                        ));
                    }

                    counter.fetch_add(1, Ordering::SeqCst);
                    ctx.report_step(i, 10, format!("Processing item {}", i));
                    sleep(Duration::from_millis(20)).await;
                }

                ctx.report_percent(100, "Background processing complete");
                Ok(serde_json::Value::Number(counter.load(Ordering::SeqCst).into()))
            }
        });

    task_ids.push(task_manager.submit_task(background_task).await?);

    // Wait for all tasks to complete
    for task_id in task_ids {
        let task_info = task_manager.wait_for_task(task_id, Some(Duration::from_secs(15))).await?;
        println!(
            "     ✓ Task '{}' completed with status: {:?}",
            task_info.name, task_info.status
        );

        if let Some(duration) = task_info.duration() {
            println!("       Duration: {:?}", duration);
        }

        if let Some(result) = &task_info.result {
            if result.success {
                println!("       Result: {:?}", result.data);
            } else {
                println!("       Error: {:?}", result.error);
            }
        }
    }

    // Show task statistics
    let stats = task_manager.get_stats().await;
    println!("     ✓ Total tasks created: {}", stats.total_created);
    println!("     ✓ Total tasks completed: {}", stats.total_completed);
    println!("     ✓ Final counter value: {}", counter.load(Ordering::SeqCst));

    task_manager.shutdown().await?;
    Ok(())
}

async fn demonstrate_concurrency() -> Result<()> {
    println!("  🔄 Concurrency Management:");

    let config = qorzen_oxide::r#mod::ConcurrencyConfig::default();
    let mut concurrency_manager = qorzen_oxide::concurrency::ConcurrencyManager::new(config)?;
    concurrency_manager.initialize().await?;

    // Test different types of concurrent operations
    let start_time = std::time::Instant::now();

    // CPU-intensive task
    let compute_future = concurrency_manager.execute_compute(|| {
        (0..100_000).fold(0u64, |acc, x| acc + x)
    });

    // I/O task
    let io_future = concurrency_manager.execute_io(|| {
        std::thread::sleep(Duration::from_millis(100));
        "I/O operation completed"
    });

    // Blocking task
    let blocking_future = concurrency_manager.execute_blocking(|| {
        std::thread::sleep(Duration::from_millis(50));
        42
    });

    // Execute all concurrently
    let (compute_result, io_result, blocking_result) =
        tokio::try_join!(compute_future, io_future, blocking_future)?;

    let elapsed = start_time.elapsed();

    println!("     ✓ Compute result: {}", compute_result);
    println!("     ✓ I/O result: {}", io_result);
    println!("     ✓ Blocking result: {}", blocking_result);
    println!("     ✓ Total time: {:?}", elapsed);

    // Show thread pool statistics
    let stats = concurrency_manager.get_all_thread_pool_stats();
    for (pool_type, pool_stats) in stats {
        println!(
            "     ✓ {:?} pool: {} active threads, {} tasks executed",
            pool_type, pool_stats.active_threads, pool_stats.total_executed
        );
    }

    concurrency_manager.shutdown().await?;
    Ok(())
}

async fn demonstrate_error_handling() -> Result<()> {
    println!("  ⚠️  Error Handling:");

    // Demonstrate different error types and handling
    use qorzen_oxide::error::{Error, ErrorSeverity};

    // Configuration error
    let config_error = Error::config("Missing required configuration value")
        .source("example_component")
        .metadata("config_key", serde_json::Value::String("database.url".to_string()));

    println!("     ✓ Config error severity: {:?}", config_error.severity);
    println!("     ✓ Should handle automatically: {}", config_error.should_handle());

    // Task error with context
    let task_error = Error::task(Some(uuid::Uuid::new_v4()), "Task execution failed")
        .caused_by(config_error);

    println!("     ✓ Task error has {} causes", task_error.causes.len());

    // File error
    let file_error = Error::file(
        "/nonexistent/path/file.txt",
        qorzen_oxide::error::FileOperation::Read,
        "File not found",
    ).severity(ErrorSeverity::Medium);

    println!("     ✓ File error is critical: {}", file_error.is_critical());

    // Demonstrate error result extensions
    use qorzen_oxide::error::ResultExt;

    let _result: Result<()> = Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "Access denied",
    ))
        .with_context(|| "Failed to access system resource".to_string())
        .with_source("example_service");

    println!("     ✓ Error context and chaining demonstrated");

    Ok(())
}