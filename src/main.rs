// src/main.rs

//! Qorzen Core CLI application entry point
//!
//! This is the main entry point for the Qorzen Core application.
//! It provides a command-line interface for running the application
//! in various modes and configurations.

mod types;
mod error;
mod utils;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{Context, Result};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use qorzen_core::{app::ApplicationCore, config::{AppConfig, ConfigManager}, logging::LoggingManager, Manager};

/// Qorzen Core - A modular plugin-based system
#[derive(Parser)]
#[command(
    name = "qorzen",
    version = qorzen_core::VERSION,
    about = "A modular plugin-based system with async core managers",
    long_about = None
)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Log level override
    #[arg(short, long, value_enum)]
    log_level: Option<LogLevel>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,

    /// Run in headless mode (no UI components)
    #[arg(long)]
    headless: bool,

    /// Validate configuration and exit
    #[arg(long)]
    validate_config: bool,

    /// Print configuration and exit
    #[arg(long)]
    print_config: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the application
    Run {
        /// Override headless mode
        #[arg(long)]
        headless: bool,
    },
    /// Validate configuration
    ValidateConfig {
        /// Configuration file to validate
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Print current configuration
    PrintConfig {
        /// Configuration file to print
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Output format
        #[arg(short, long, value_enum, default_value = "yaml")]
        format: ConfigFormat,
    },
    /// Show application status
    Status {
        /// Configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Output format
        #[arg(short, long, value_enum, default_value = "human")]
        format: OutputFormat,
    },
    /// Show application health
    Health {
        /// Configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Output format
        #[arg(short, long, value_enum, default_value = "human")]
        format: OutputFormat,
    },
    /// Manager operations
    Manager {
        #[command(subcommand)]
        action: ManagerAction,
    },
}

#[derive(Subcommand)]
enum ManagerAction {
    /// List all managers
    List {
        /// Output format
        #[arg(short, long, value_enum, default_value = "human")]
        format: OutputFormat,
    },
    /// Show manager status
    Status {
        /// Manager name
        name: String,
        /// Output format
        #[arg(short, long, value_enum, default_value = "human")]
        format: OutputFormat,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
        }
    }
}

#[derive(clap::ValueEnum, Clone)]
enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Human,
    Json,
    Yaml,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup error handling
    color_eyre::install()?;

    let cli = Cli::parse();

    // Setup basic logging for CLI
    setup_cli_logging(&cli)?;

    // Handle early exit commands
    if cli.validate_config {
        return validate_config_command(cli.config).await;
    }

    if cli.print_config {
        return print_config_command(cli.config, ConfigFormat::Yaml).await;
    }

    // Execute commands
    match &cli.command {
        Some(Commands::Run { headless }) => {
            run_application(&cli, *headless || cli.headless).await
        }
        Some(Commands::ValidateConfig { config }) => {
            validate_config_command(config.clone().or(cli.config)).await
        }
        Some(Commands::PrintConfig { config, format }) => {
            print_config_command(config.clone().or(cli.config), format.clone()).await
        }
        Some(Commands::Status { config, format }) => {
            show_status_command(config.clone().or(cli.config), format.clone()).await
        }
        Some(Commands::Health { config, format }) => {
            show_health_command(config.clone().or(cli.config), format.clone()).await
        }
        Some(Commands::Manager { action }) => {
            handle_manager_command(action, &cli).await
        }
        None => {
            // Default to run command
            run_application(&cli, cli.headless).await
        }
    }
}

/// Setup CLI logging before application initialization
fn setup_cli_logging(cli: &Cli) -> Result<()> {
    let log_level = cli.log_level.clone().unwrap_or_else(|| {
        if cli.debug {
            LogLevel::Debug
        } else if cli.verbose {
            LogLevel::Info
        } else {
            LogLevel::Warn
        }
    });

    let env_filter = EnvFilter::from_default_env()
        .add_directive(LevelFilter::from(log_level).into());

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(env_filter);

    Ok(())
}

/// Run the main application
async fn run_application(cli: &Cli, headless: bool) -> Result<()> {
    tracing::info!("Starting Qorzen Core v{}", qorzen_core::VERSION);

    // Create application core
    let mut app = if let Some(config_path) = &cli.config {
        ApplicationCore::with_config_file(config_path)
    } else {
        ApplicationCore::new()
    };

    // Initialize application
    app.initialize().await
        .context("Failed to initialize application")?;

    tracing::info!("Application initialized successfully");

    if headless {
        tracing::info!("Running in headless mode");
    } else {
        tracing::info!("Running with full system");
    }

    // Setup graceful shutdown
    let shutdown_future = async {
        #[cfg(unix)]
        {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to register SIGTERM handler");
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM");
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT");
                }
            }
        }

        #[cfg(windows)]
        {
            let mut ctrl_c = signal::windows::ctrl_c()
                .expect("Failed to register Ctrl+C handler");

            ctrl_c.recv().await;
            tracing::info!("Received Ctrl+C");
        }
    };

    // Wait for shutdown signal or application termination
    tokio::select! {
        _ = shutdown_future => {
            tracing::info!("Shutdown signal received, terminating application");
        }
        _ = app.wait_for_shutdown() => {
            tracing::info!("Application requested shutdown");
        }
    }

    // Shutdown application
    tracing::info!("Shutting down application");
    app.shutdown().await
        .context("Failed to shutdown application gracefully")?;

    tracing::info!("Application shutdown complete");
    Ok(())
}

/// Validate configuration file
async fn validate_config_command(config_path: Option<PathBuf>) -> Result<()> {
    tracing::info!("Validating configuration");

    let config_manager = if let Some(path) = config_path {
        ConfigManager::with_config_file(path)
    } else {
        ConfigManager::new()
    };

    // Try to load and validate configuration
    let mut config_manager = config_manager;
    config_manager.initialize().await
        .context("Configuration validation failed")?;

    let config = config_manager.get_config().await;
    tracing::info!("Configuration validation successful");

    println!("✅ Configuration is valid");
    println!("   App name: {}", config.app.name);
    println!("   Version: {}", config.app.version);
    println!("   Environment: {}", config.app.environment);

    Ok(())
}

/// Print configuration to stdout
async fn print_config_command(config_path: Option<PathBuf>, format: ConfigFormat) -> Result<()> {
    let config_manager = if let Some(path) = config_path {
        ConfigManager::with_config_file(path)
    } else {
        ConfigManager::new()
    };

    let mut config_manager = config_manager;
    config_manager.initialize().await
        .context("Failed to load configuration")?;

    let config = config_manager.get_config().await;

    match format {
        ConfigFormat::Yaml => {
            let yaml = serde_yaml::to_string(&config)
                .context("Failed to serialize configuration to YAML")?;
            print!("{}", yaml);
        }
        ConfigFormat::Json => {
            let json = serde_json::to_string_pretty(&config)
                .context("Failed to serialize configuration to JSON")?;
            print!("{}", json);
        }
        ConfigFormat::Toml => {
            let toml = toml::to_string(&config)
                .context("Failed to serialize configuration to TOML")?;
            print!("{}", toml);
        }
    }

    Ok(())
}

/// Show application status
async fn show_status_command(config_path: Option<PathBuf>, format: OutputFormat) -> Result<()> {
    // For this example, we'll create a temporary application to get status
    // In a real implementation, you might connect to a running instance

    let mut app = if let Some(path) = config_path {
        ApplicationCore::with_config_file(path)
    } else {
        ApplicationCore::new()
    };

    app.initialize().await
        .context("Failed to initialize application for status check")?;

    let stats = app.get_stats().await;

    match format {
        OutputFormat::Human => {
            println!("Qorzen Core Status");
            println!("=================");
            println!("Version: {}", stats.version);
            println!("State: {:?}", stats.state);
            println!("Uptime: {:?}", stats.uptime);
            println!("Started: {}", stats.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("Managers: {}/{} initialized", stats.initialized_managers, stats.manager_count);
            if stats.failed_managers > 0 {
                println!("Failed managers: {}", stats.failed_managers);
            }
            println!("System: {} {} ({})", stats.system_info.os_name, stats.system_info.os_version, stats.system_info.arch);
            println!("CPU cores: {}", stats.system_info.cpu_cores);
            println!("Hostname: {}", stats.system_info.hostname);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&stats)
                .context("Failed to serialize status to JSON")?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&stats)
                .context("Failed to serialize status to YAML")?;
            print!("{}", yaml);
        }
    }

    app.shutdown().await.ok();
    Ok(())
}

/// Show application health
async fn show_health_command(config_path: Option<PathBuf>, format: OutputFormat) -> Result<()> {
    let mut app = if let Some(path) = config_path {
        ApplicationCore::with_config_file(path)
    } else {
        ApplicationCore::new()
    };

    app.initialize().await
        .context("Failed to initialize application for health check")?;

    let health = app.get_health().await;

    match format {
        OutputFormat::Human => {
            println!("Qorzen Core Health");
            println!("==================");
            println!("Overall status: {:?}", health.status);
            println!("Uptime: {:?}", health.uptime);
            println!("Last check: {}", health.last_check.format("%Y-%m-%d %H:%M:%S UTC"));
            println!();
            println!("Manager Health:");
            for (name, status) in &health.managers {
                let status_icon = match status {
                    qorzen_core::manager::HealthStatus::Healthy => "✅",
                    qorzen_core::manager::HealthStatus::Degraded => "⚠️",
                    qorzen_core::manager::HealthStatus::Unhealthy => "❌",
                    qorzen_core::manager::HealthStatus::Unknown => "❓",
                };
                println!("  {} {}: {:?}", status_icon, name, status);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&health)
                .context("Failed to serialize health to JSON")?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&health)
                .context("Failed to serialize health to YAML")?;
            print!("{}", yaml);
        }
    }

    // Set exit code based on health
    let exit_code = match health.status {
        qorzen_core::manager::HealthStatus::Healthy => 0,
        qorzen_core::manager::HealthStatus::Degraded => 1,
        qorzen_core::manager::HealthStatus::Unhealthy => 2,
        qorzen_core::manager::HealthStatus::Unknown => 3,
    };

    app.shutdown().await.ok();

    if exit_code != 0 {
        process::exit(exit_code);
    }

    Ok(())
}

/// Handle manager-related commands
async fn handle_manager_command(action: &ManagerAction, cli: &Cli) -> Result<()> {
    let mut app = if let Some(config_path) = &cli.config {
        ApplicationCore::with_config_file(config_path)
    } else {
        ApplicationCore::new()
    };

    app.initialize().await
        .context("Failed to initialize application for manager operation")?;

    match action {
        ManagerAction::List { format } => {
            let status = app.status().await;

            match format {
                OutputFormat::Human => {
                    println!("Registered Managers");
                    println!("==================");
                    println!("Application Core: {:?}", status.state);
                    // In a real implementation, you'd list all registered managers
                    println!("  - config_manager");
                    println!("  - logging_manager");
                    println!("  - event_bus_manager");
                    println!("  - file_manager");
                    println!("  - concurrency_manager");
                    println!("  - task_manager");
                }
                OutputFormat::Json => {
                    let managers_info = serde_json::json!({
                        "managers": [
                            "config_manager",
                            "logging_manager", 
                            "event_bus_manager",
                            "file_manager",
                            "concurrency_manager",
                            "task_manager"
                        ]
                    });
                    println!("{}", serde_json::to_string_pretty(&managers_info)?);
                }
                OutputFormat::Yaml => {
                    println!("managers:");
                    println!("  - config_manager");
                    println!("  - logging_manager");
                    println!("  - event_bus_manager");
                    println!("  - file_manager");
                    println!("  - concurrency_manager");
                    println!("  - task_manager");
                }
            }
        }
        ManagerAction::Status { name, format } => {
            match name.as_str() {
                "application_core" => {
                    let status = app.status().await;
                    match format {
                        OutputFormat::Human => {
                            println!("Manager: {}", status.name);
                            println!("State: {:?}", status.state);
                            println!("Health: {:?}", status.health);
                            println!("Created: {}", status.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                            if let Some(init_at) = status.started_at {
                                println!("Initialized: {}", init_at.format("%Y-%m-%d %H:%M:%S UTC"));
                            }
                        }
                        OutputFormat::Json => {
                            println!("{}", serde_json::to_string_pretty(&status)?);
                        }
                        OutputFormat::Yaml => {
                            println!("{}", serde_yaml::to_string(&status)?);
                        }
                    }
                }
                _ => {
                    eprintln!("Manager '{}' not found or status not available", name);
                    process::exit(1);
                }
            }
        }
    }

    app.shutdown().await.ok();
    Ok(())
}

/// Print version information
fn print_version() {
    println!("Qorzen Core v{}", qorzen_core::VERSION);
    println!("Built with Rust {}", env!("RUSTC_VERSION"));
    println!("Target: {}", env!("TARGET"));
}

/// Print help with examples
fn print_help_with_examples() {
    println!("Qorzen Core v{}", qorzen_core::VERSION);
    println!("A modular plugin-based system with async core managers");
    println!();
    println!("EXAMPLES:");
    println!("  # Run with default configuration");
    println!("  qorzen run");
    println!();
    println!("  # Run with custom configuration");
    println!("  qorzen --config config.yaml run");
    println!();
    println!("  # Run in headless mode");
    println!("  qorzen run --headless");
    println!();
    println!("  # Validate configuration");
    println!("  qorzen validate-config --config config.yaml");
    println!();
    println!("  # Show application status");
    println!("  qorzen status --format json");
    println!();
    println!("  # Check application health");
    println!("  qorzen health");
    println!();
    println!("  # List managers");
    println!("  qorzen manager list");
    println!();
    println!("  # Show manager status");
    println!("  qorzen manager status application_core");
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert()
    }

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LevelFilter::from(LogLevel::Debug), LevelFilter::DEBUG);
        assert_eq!(LevelFilter::from(LogLevel::Info), LevelFilter::INFO);
        assert_eq!(LevelFilter::from(LogLevel::Error), LevelFilter::ERROR);
    }

    #[tokio::test]
    async fn test_validate_config_with_default() {
        // This should not fail with default configuration
        let result = validate_config_command(None).await;
        assert!(result.is_ok());
    }
}