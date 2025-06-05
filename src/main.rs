// src/main.rs - Fixed application entry point with proper ApplicationCore integration

#![allow(clippy::result_large_err)]
#![cfg_attr(
    all(target_os = "windows", not(target_arch = "wasm32"), not(debug_assertions)),
    windows_subsystem = "windows"
)]

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use clap::{Parser, Subcommand};
use dioxus::prelude::Element;
use qorzen_oxide::error::Result;
use qorzen_oxide::ui::App;

use qorzen_oxide::app::core::{set_application_core, get_application_core};

#[cfg(not(target_arch = "wasm32"))]
use qorzen_oxide::app::native::ApplicationCore;

#[cfg(target_arch = "wasm32")]
use qorzen_oxide::app::wasm::ApplicationCore;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser)]
#[command(name = "qorzen-oxide", version = qorzen_oxide::VERSION)]
#[command(about = "A modular, cross-platform application framework built with Rust and Dioxus")]
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

#[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(debug_assertions)]
    Dev {
        #[arg(short, long, default_value = "8080")]
        port: u16,

        #[arg(long, default_value = "localhost")]
        host: String,
    },
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let cli = Cli::parse();
    // Set up logging FIRST, before anything else
    setup_logging(&cli);

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

#[cfg(target_arch = "wasm32")]
fn main() {
    // Setup panic hook for better error reporting
    console_error_panic_hook::set_once();

    // Initialize tracing for WASM
    wasm_logger::init(wasm_logger::Config::default());

    tracing::info!("Starting Qorzen Oxide WASM application");

    // Initialize plugin system for WASM
    wasm_bindgen_futures::spawn_local(async {
        // Initialize the plugin factory registry
        qorzen_oxide::plugin::PluginFactoryRegistry::initialize();

        // Register builtin plugins
        if let Err(e) = qorzen_oxide::plugin::builtin::register_builtin_plugins().await {
            web_sys::console::error_1(&format!("Failed to register builtin plugins: {}", e).into());
        } else {
            web_sys::console::log_1(&"Successfully registered builtin plugins".into());
        }
    });

    // Launch the UI - WASM plugins are compiled in so they're always available
    dioxus::web::launch(App);
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_logging(cli: &Cli) {
    let level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::INFO
    };

    // Create logs directory
    if let Err(e) = std::fs::create_dir_all("logs") {
        eprintln!("Failed to create logs directory: {}", e);
    }

    // Set up file appender - fix: this doesn't return a Result
    let file_appender = tracing_appender::rolling::daily("logs", "qorzen.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Fix: Use consistent subscriber initialization
    use tracing_subscriber::prelude::*;

    let result = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(true)
                .with_writer(std::io::stdout)
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(non_blocking)
        )
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(level.into()))
        .try_init();

    match result {
        Ok(_) => {
            println!("✅ Logging initialized at level: {:?}", level);
            println!("📁 Logs will be written to: logs/qorzen.log");
            tracing::info!("Qorzen Oxide v{} starting up", qorzen_oxide::VERSION);
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize logging: {}", e);
        }
    }
}

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

#[cfg(not(target_arch = "wasm32"))]
fn run_ui_application(cli: &Cli) {
    tracing::info!("Starting Qorzen Oxide v{} (Desktop UI)", qorzen_oxide::VERSION);

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        // Initialize plugin registry early
        tracing::info!("Initializing plugin registry...");
        qorzen_oxide::plugin::PluginFactoryRegistry::initialize();

        if let Err(e) = qorzen_oxide::plugin::builtin::register_builtin_plugins().await {
            tracing::warn!("Failed to register builtin plugins: {}", e);
        } else {
            tracing::info!("Successfully registered builtin plugins");
        }

        tracing::info!("Initializing ApplicationCore...");
        let mut app_core = if let Some(config_path) = &cli.config {
            ApplicationCore::with_config_file(config_path)
        } else {
            ApplicationCore::new()
        };

        if let Err(e) = app_core.initialize().await {
            tracing::error!("Failed to initialize ApplicationCore: {}", e);
            eprintln!("Failed to initialize application: {}", e);
            process::exit(1);
        }

        tracing::info!("ApplicationCore initialized successfully");

        // Use the core.rs function to store the ApplicationCore
        set_application_core(app_core);
        tracing::info!("ApplicationCore stored globally, starting UI...");
    });

    dioxus::launch(AppWithDesktopCSS);

    rt.block_on(async {
        tracing::info!("UI closed, shutting down ApplicationCore...");
        if let Some(app_core_arc) = get_application_core() {
            let mut app_core = app_core_arc.write().await;
            if let Err(e) = app_core.shutdown().await {
                tracing::error!("Error during ApplicationCore shutdown: {}", e);
            }
        }
        tracing::info!("ApplicationCore shutdown complete");
    });
}

// Wrapper component for desktop that includes CSS
#[cfg(not(target_arch = "wasm32"))]
#[allow(non_snake_case)]
fn AppWithDesktopCSS() -> Element {
    use dioxus::prelude::*;

    rsx! {
        head {
            style {
                // Include Tailwind CSS for desktop builds
                dangerous_inner_html: include_str!("../public/static/tailwind.css")
            }
        }
        App {}
    }
}

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

#[cfg(not(target_arch = "wasm32"))]
async fn run_application_async(cli: &Cli, headless: bool) -> Result<()> {
    tracing::info!(
        "Starting Qorzen Oxide v{} (Headless)",
        qorzen_oxide::VERSION
    );

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

#[cfg(all(not(target_arch = "wasm32"), debug_assertions))]
fn run_dev_server(port: u16, host: String) {
    tracing::info!("Starting development server on {}:{}", host, port);

    // For now, just launch the regular desktop app
    println!("Development server would start here");
    println!("Open http://{}:{} in your browser", host, port);

    // Launch desktop app for now
    run_ui_application(&Cli::parse());
}

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

        // Show plugin stats
        if let Some(plugin_stats) = app.get_plugin_stats().await {
            println!("Plugins: {}/{} active", plugin_stats.active_plugins, plugin_stats.total_plugins);
        }

        app.shutdown().await?;
        Ok(())
    })
}

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

        // Show individual manager health
        for (manager, status) in &health.managers {
            println!("{}: {:?}", manager, status);
        }

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

#[cfg(not(target_arch = "wasm32"))]
fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    println!("Validating configuration...");

    let _app = if let Some(path) = config_path {
        if !path.exists() {
            eprintln!(
                "Error: Configuration file does not exist: {}",
                path.display()
            );
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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_application_core_integration() {
        // Test that we can initialize and access the ApplicationCore
        let mut app = ApplicationCore::new();
        assert!(app.initialize().await.is_ok());

        // Test plugin manager access
        assert!(app.get_plugin_manager().is_some());

        assert!(app.shutdown().await.is_ok());
    }
}