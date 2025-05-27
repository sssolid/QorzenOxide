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
    Run {
        #[arg(long)]
        headless: bool,
    },
    Status,
    Health,
    ValidateConfig {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

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

fn run_ui_application(cli: &Cli) {
    tracing::info!("Starting Qorzen Oxide v{}", qorzen_oxide::VERSION);

    // Create initial app state
    let app_state = AppState {
        current_user: None,
        current_session: None,
        current_layout: Default::default(),
        current_theme: Default::default(),
        is_loading: false,
        error_message: None,
        notifications: Vec::new(),
    };

    // // Launch Dioxus application - this will handle the async runtime
    // #[cfg(target_arch = "wasm32")]
    // {
    //     #[cfg(feature = "web")]
    //     dioxus::launch(App);
    // }
    // 
    // #[cfg(not(target_arch = "wasm32"))]
    // {
    //     #[cfg(feature = "desktop")]
    //     dioxus::launch(App);
    // }
    #[cfg(target_arch = "wasm32")]
    fn main() {
        qorzen_oxide::web::launch();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn main() {
        qorzen_oxide::app::native::launch();
    }

}

fn run_headless_application(cli: &Cli) {
    // Use a new runtime for headless mode
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        if let Err(e) = run_application_async(cli, true).await {
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    });
}

fn run_headless_command<F>(command: F)
where
    F: FnOnce() -> Result<()>,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        if let Err(e) = command() {
            eprintln!("Command error: {}", e);
            process::exit(1);
        }
    });
}

async fn run_application_async(cli: &Cli, headless: bool) -> Result<()> {
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
        app.wait_for_shutdown().await?;
    }

    // Shutdown application
    app.shutdown().await?;
    Ok(())
}

fn show_status() -> Result<()> {
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

fn check_health() -> Result<()> {
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
    })
}

fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
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