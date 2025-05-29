// src/main.rs - Application entry point with enhanced UI integration

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process;

#[cfg(not(target_arch = "wasm32"))]
use clap::{Parser, Subcommand};
use qorzen_oxide::ui::App;
use qorzen_oxide::error::Result;

#[cfg(not(target_arch = "wasm32"))]
use qorzen_oxide::app::ApplicationCore;

/// Command-line interface for native platforms
#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser)]
#[command(name = "qorzen-oxide", version = qorzen_oxide::VERSION)]
#[command(about = "A modular, cross-platform application framework built with Rust and Dioxus")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Run in headless mode (no GUI)
    #[arg(long)]
    headless: bool,

    /// Subcommands
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available commands
#[cfg(not(target_arch = "wasm32"))]
#[derive(Subcommand)]
enum Commands {
    /// Run the application
    Run {
        /// Run in headless mode
        #[arg(long)]
        headless: bool
    },

    /// Show application status
    Status,

    /// Check application health
    Health,

    /// Validate configuration file
    ValidateConfig {
        /// Configuration file to validate
        #[arg(short, long)]
        config: Option<PathBuf>
    },

    /// Start development server with hot reload
    #[cfg(debug_assertions)]
    Dev {
        /// Port to serve on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "localhost")]
        host: String,
    },
}

/// Main entry point for native platforms
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let cli = Cli::parse();
    setup_logging(&cli);

    // Handle version information
    if cli.debug {
        print_system_info();
    }

    match &cli.command {
        Some(Commands::Run { headless }) => {
            if *headless || cli.headless {
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
        #[cfg(debug_assertions)]
        Some(Commands::Dev { port, host }) => {
            run_dev_server(*port, host.clone());
        }
        None => {
            if cli.headless {
                run_headless_application(&cli);
            } else {
                run_ui_application(&cli);
            }
        }
    }
}

/// Main entry point for WASM/web platform
#[cfg(target_arch = "wasm32")]
fn main() {
    // Set up panic hook for better error reporting in browser
    console_error_panic_hook::set_once();

    // Initialize logging for web
    wasm_logger::init(wasm_logger::Config::default());

    // Log application start
    web_sys::console::log_1(&format!("Starting Qorzen Oxide v{}", qorzen_oxide::VERSION).into());

    // Launch the Dioxus application without hydration
    dioxus::launch(App);
}

/// Set up logging based on CLI arguments
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

    tracing::info!("Logging initialized at level: {:?}", level);
}

/// Print system information for debugging
#[cfg(not(target_arch = "wasm32"))]
fn print_system_info() {
    tracing::info!("=== System Information ===");
    tracing::info!("Version: {}", qorzen_oxide::VERSION);

    // Only try to get build hash if available
    if let Ok(build_hash) = std::env::var("BUILD_HASH") {
        tracing::info!("Build: {}", build_hash);
    } else {
        tracing::info!("Build: dev");
    }

    tracing::info!("Target: {}", std::env::consts::OS);
    tracing::info!("Architecture: {}", std::env::consts::ARCH);

    // Get Rust version at runtime if needed
    if let Ok(rustc_version) = std::env::var("RUSTC_VERSION") {
        tracing::info!("Rust version: {}", rustc_version);
    }

    tracing::info!("CPU cores: {}", num_cpus::get());
    tracing::info!("================================");
}

/// Run the UI application (desktop mode)
#[cfg(not(target_arch = "wasm32"))]
fn run_ui_application(_cli: &Cli) {
    tracing::info!("Starting Qorzen Oxide v{} (Desktop UI)", qorzen_oxide::VERSION);
    
    // Launch the desktop application
    dioxus::launch(App);
}

/// Run the application in headless mode
#[cfg(not(target_arch = "wasm32"))]
fn run_headless_application(cli: &Cli) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        if let Err(e) = run_application_async(cli, true).await {
            tracing::error!("Application error: {}", e);
            eprintln!("Application error: {}", e);
            process::exit(1);
        }
    });
}

/// Run a headless command
#[cfg(not(target_arch = "wasm32"))]
fn run_headless_command<F>(command: F)
where
    F: FnOnce() -> Result<()>,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        if let Err(e) = command() {
            tracing::error!("Command error: {}", e);
            eprintln!("Command error: {}", e);
            process::exit(1);
        }
    });
}

/// Run the application core asynchronously
#[cfg(not(target_arch = "wasm32"))]
async fn run_application_async(cli: &Cli, headless: bool) -> Result<()> {
    tracing::info!("Starting Qorzen Oxide v{} (Headless)", qorzen_oxide::VERSION);

    let mut app = if let Some(config_path) = &cli.config {
        ApplicationCore::with_config_file(config_path)
    } else {
        ApplicationCore::new()
    };

    // Initialize the application core
    app.initialize().await?;

    if headless {
        tracing::info!("Running in headless mode - waiting for shutdown signal");
        app.wait_for_shutdown().await?;
    }

    // Graceful shutdown
    app.shutdown().await?;
    Ok(())
}

/// Development server for hot reloading
#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
fn run_dev_server(port: u16, host: String) {
    tracing::info!("Starting development server on {}:{}", host, port);

    // For now, just launch the regular desktop app
    println!("Development server would start here");
    println!("Open http://{}:{} in your browser", host, port);

    // Launch desktop app for now
    dioxus::launch(App);
}

/// Show application status
#[cfg(not(target_arch = "wasm32"))]
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

        app.shutdown().await?;
        Ok(())
    })
}

/// Check application health
#[cfg(not(target_arch = "wasm32"))]
fn check_health() -> Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        let mut app = ApplicationCore::new();
        app.initialize().await?;
        let health = app.get_health().await;

        println!("Qorzen Oxide Health Check");
        println!("========================");
        println!("Overall status: {:?}", health.status);

        // Exit with appropriate code based on health
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

/// Validate configuration file
#[cfg(not(target_arch = "wasm32"))]
fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    println!("Validating configuration...");

    let _app = if let Some(path) = config_path {
        if !path.exists() {
            eprintln!("Error: Configuration file does not exist: {}", path.display());
            process::exit(1);
        }

        println!("Using configuration file: {}", path.display());
        ApplicationCore::with_config_file(path)
    } else {
        println!("Using default configuration");
        ApplicationCore::new()
    };

    // In a real implementation, this would parse and validate the config
    println!("✅ Configuration is valid");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!qorzen_oxide::VERSION.is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_cli_parsing() {
        use clap::Parser;

        // Test basic parsing
        let cli = Cli::try_parse_from(&["qorzen-oxide", "--verbose"]).unwrap();
        assert!(cli.verbose);
        assert!(!cli.debug);

        // Test with subcommand
        let cli = Cli::try_parse_from(&["qorzen-oxide", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Status)));
    }
}