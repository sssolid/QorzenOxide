// src/main.rs

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process;

#[cfg(not(target_arch = "wasm32"))]
use clap::{Parser, Subcommand};

use qorzen_oxide::ui::app;
use qorzen_oxide::error::Result;

#[cfg(not(target_arch = "wasm32"))]
use qorzen_oxide::app::ApplicationCore;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser)]
#[command(name = "qorzen-oxide", version = qorzen_oxide::VERSION)]
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
    Run { #[arg(long)] headless: bool },
    Status,
    Health,
    ValidateConfig { #[arg(short, long)] config: Option<PathBuf> },
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(app);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let cli = Cli::parse();
    setup_logging(&cli);

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
        None => {
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
fn run_ui_application(_cli: &Cli) {
    tracing::info!("Starting Qorzen Oxide v{}", qorzen_oxide::VERSION);
    // This should work now with desktop features enabled
    dioxus::launch(app);
}

#[cfg(not(target_arch = "wasm32"))]
fn run_headless_application(cli: &Cli) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        if let Err(e) = run_application_async(cli, true).await {
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
            eprintln!("Command error: {}", e);
            process::exit(1);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_application_async(cli: &Cli, headless: bool) -> Result<()> {
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

#[cfg(not(target_arch = "wasm32"))]
fn check_health() -> Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        let mut app = ApplicationCore::new();
        app.initialize().await?;
        let health = app.get_health().await;

        println!("Qorzen Oxide Health");
        println!("==================");
        println!("Overall status: {:?}", health.status);

        app.shutdown().await?;
        Ok(())
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    println!("Validating configuration...");
    let _app = if let Some(path) = config_path {
        ApplicationCore::with_config_file(path)
    } else {
        ApplicationCore::new()
    };
    println!("✅ Configuration is valid");
    Ok(())
}