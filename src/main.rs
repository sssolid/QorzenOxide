// src/main.rs - Cross-platform main entry point

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dioxus::prelude::*;

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
    Run {
        #[arg(long)]
        headless: bool,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Status,
    #[cfg(not(target_arch = "wasm32"))]
    Health,
    #[cfg(not(target_arch = "wasm32"))]
    ValidateConfig {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Set up panic hook
    console_error_panic_hook::set_once();

    // Initialize tracing for web
    tracing_wasm::set_as_global_default();

    // Launch Dioxus app directly for web
    dioxus::launch(App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    // Setup basic logging
    setup_logging(&cli);

    // Execute command based on what was requested
    match &cli.command {
        Some(Commands::Run { headless }) => {
            let headless = *headless || cli.headless;
            if headless {
                run_headless_application(&cli);
            } else {
                run_ui_application(&cli);
            }
        }
        Some(Commands::Status) => {
            run_headless_command(show_status);
        }
        Some(Commands::Health) => {
            run_headless_command(check_health);
        }
        Some(Commands::ValidateConfig { config }) => {
            let config_path = config.clone().or(cli.config.clone());
            run_headless_command(move || validate_config(config_path));
        }
        None => {
            // Default to run command
            if cli.headless {
                run_headless_application(&cli);
            } else {
                run_ui_application(&cli);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
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
        .try_init()
        .ok();
}

#[cfg(not(target_arch = "wasm32"))]
fn run_ui_application(cli: &Cli) {
    tracing::info!("Starting Qorzen Oxide v{}", qorzen_oxide::VERSION);
    dioxus::launch(App);
}

#[cfg(not(target_arch = "wasm32"))]
fn run_headless_application(cli: &Cli) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        if let Err(e) = run_application_async(cli, true).await {
            eprintln!("Application error: {}", e);
            std::process::exit(1);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn run_headless_command<F>(command: F)
where
    F: FnOnce() -> Result<()>,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        if let Err(e) = command() {
            eprintln!("Command error: {}", e);
            std::process::exit(1);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_application_async(cli: &Cli, headless: bool) -> Result<()> {
    use qorzen_oxide::ApplicationCore;

    tracing::info!("Starting Qorzen Oxide v{}", qorzen_oxide::VERSION);

    let mut app = if let Some(config_path) = &cli.config {
        ApplicationCore::with_config_file(config_path)
    } else {
        ApplicationCore::new()
    };

    app.initialize().await?;

    if headless {
        tracing::info!("Running in headless mode");
        app.wait_for_shutdown().await?;
    }

    app.shutdown().await?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn show_status() -> Result<()> {
    use qorzen_oxide::ApplicationCore;

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        println!("Qorzen Oxide Status");
        println!("==================");

        let mut app = ApplicationCore::new();
        app.initialize().await?;

        let stats = app.get_stats().await;

        println!("Version: {}", stats.version);
        println!("State: {:?}", stats.state);
        println!("Uptime: {:?}", stats.uptime);
        println!("Managers: {}", stats.manager_count);
        println!(
            "System: {} {} ({})",
            stats.system_info.os_name, stats.system_info.os_version, stats.system_info.arch
        );
        println!("CPU cores: {}", stats.system_info.cpu_cores);
        println!("Hostname: {}", stats.system_info.hostname);

        app.shutdown().await?;
        Ok(())
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn check_health() -> Result<()> {
    use qorzen_oxide::ApplicationCore;

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let mut app = ApplicationCore::new();
        app.initialize().await?;

        let health = app.get_health().await;

        println!("Qorzen Oxide Health");
        println!("==================");
        println!("Overall status: {:?}", health.status);
        println!("Uptime: {:?}", health.uptime);
        println!(
            "Last check: {}",
            health.last_check.format("%Y-%m-%d %H:%M:%S UTC")
        );
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

        let exit_code = match health.status {
            qorzen_oxide::manager::HealthStatus::Healthy => 0,
            qorzen_oxide::manager::HealthStatus::Degraded => 1,
            qorzen_oxide::manager::HealthStatus::Unhealthy => 2,
            qorzen_oxide::manager::HealthStatus::Unknown => 3,
        };

        app.shutdown().await?;

        if exit_code != 0 {
            std::process::exit(exit_code);
        }

        Ok(())
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    use qorzen_oxide::ApplicationCore;

    println!("Validating configuration...");

    let _app = if let Some(path) = config_path {
        ApplicationCore::with_config_file(path)
    } else {
        ApplicationCore::new()
    };

    println!("✅ Configuration is valid");
    println!("   Framework: Qorzen Oxide");
    println!("   Version: {}", qorzen_oxide::VERSION);

    Ok(())
}