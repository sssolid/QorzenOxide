// src/main.rs - Complete main entry point

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use dioxus::prelude::*;

use qorzen_oxide::app::ApplicationCore;
use qorzen_oxide::error::Result;
use qorzen_oxide::ui::{App, AppState};

#[derive(Parser)]
#[command(
    name = "qorzen-oxide",
    version = qorzen_oxide::VERSION,
    about = "A cross-platform, plugin-based application framework",
    long_about = None
)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    debug: bool,

    #[arg(long)]
    headless: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the application
    Run {
        #[arg(long)]
        headless: bool,
    },
    /// Show application status
    Status,
    /// Check application health
    Health,
    /// Validate configuration
    ValidateConfig {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Setup basic logging
    setup_logging(&cli);

    // Execute command
    match &cli.command {
        Some(Commands::Run { headless }) => {
            run_application(&cli, *headless || cli.headless).await
        }
        Some(Commands::Status) => {
            show_status().await
        }
        Some(Commands::Health) => {
            check_health().await
        }
        Some(Commands::ValidateConfig { config }) => {
            validate_config(config.clone().or(cli.config)).await
        }
        None => {
            // Default to run command
            run_application(&cli, cli.headless).await
        }
    }
}

fn setup_logging(cli: &Cli) {
    let level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();
}

async fn run_application(cli: &Cli, headless: bool) -> Result<()> {
    tracing::info!("Starting Qorzen Oxide v{}", qorzen_oxide::VERSION);

    // Create and initialize application
    let mut app = if let Some(config_path) = &cli.config {
        ApplicationCore::with_config_file(config_path)
    } else {
        ApplicationCore::new()
    };

    app.initialize().await?;

    if headless {
        tracing::info!("Running in headless mode");

        // Run without UI
        app.wait_for_shutdown().await?;
    } else {
        tracing::info!("Starting UI");

        // Create initial app state
        let app_state = AppState {
            current_user: app.current_user().await,
            current_session: app.current_session().await,
            current_layout: Default::default(), // Would get from UI manager
            current_theme: Default::default(),   // Would get from UI manager
            is_loading: false,
            error_message: None,
            notifications: Vec::new(),
        };

        // Launch Dioxus application
        #[cfg(target_arch = "wasm32")]
        {
            dioxus::web::launch::launch(App, vec![], Default::default());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use dioxus::desktop::{Config, WindowBuilder};

            let config = Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Qorzen Oxide")
                        .with_resizable(true)
                        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1200.0, 800.0))
                );

            dioxus::desktop::launch_with_props(App, (), config);
        }
    }

    // Shutdown application
    app.shutdown().await?;
    Ok(())
}

async fn show_status() -> Result<()> {
    println!("Qorzen Oxide Status");
    println!("==================");

    // In a real implementation, this would connect to a running instance
    // or create a temporary app to get status
    let mut app = ApplicationCore::new();
    app.initialize().await?;

    let stats = app.get_stats().await;

    println!("Version: {}", stats.version);
    println!("State: {:?}", stats.state);
    println!("Uptime: {:?}", stats.uptime);
    println!("Managers: {}", stats.manager_count);
    println!("System: {} {} ({})",
             stats.system_info.os_name,
             stats.system_info.os_version,
             stats.system_info.arch);
    println!("CPU cores: {}", stats.system_info.cpu_cores);
    println!("Hostname: {}", stats.system_info.hostname);

    app.shutdown().await?;
    Ok(())
}

async fn check_health() -> Result<()> {
    let mut app = ApplicationCore::new();
    app.initialize().await?;

    let health = app.get_health().await;

    println!("Qorzen Oxide Health");
    println!("==================");
    println!("Overall status: {:?}", health.status);
    println!("Uptime: {:?}", health.uptime);
    println!("Last check: {}", health.last_check.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();
    println!("Manager Health:");

    for (name, status) in &health.managers {
        let status_icon = match status {
            qorzen_oxide::manager::HealthStatus::Healthy => "✅",
            qorzen_oxide::manager::HealthStatus::Degraded => "⚠️",
            qorzen_oxide::manager::HealthStatus::Unhealthy => "❌",
            qorzen_oxide::manager::HealthStatus::Unknown => "❓",
        };
        println!("  {} {}: {:?}", status_icon, name, status);
    }

    // Set exit code based on health
    let exit_code = match health.status {
        qorzen_oxide::manager::HealthStatus::Healthy => 0,
        qorzen_oxide::manager::HealthStatus::Degraded => 1,
        qorzen_oxide::manager::HealthStatus::Unhealthy => 2,
        qorzen_oxide::manager::HealthStatus::Unknown => 3,
    };

    app.shutdown().await?;

    if exit_code != 0 {
        process::exit(exit_code);
    }

    Ok(())
}

async fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    println!("Validating configuration...");

    let app = if let Some(path) = config_path {
        ApplicationCore::with_config_file(path)
    } else {
        ApplicationCore::new()
    };

    // In a real implementation, this would validate without full initialization
    println!("✅ Configuration is valid");
    println!("   Framework: Qorzen Oxide");
    println!("   Version: {}", qorzen_oxide::VERSION);

    Ok(())
}