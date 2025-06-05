// scripts/build_plugins.rs - Plugin Build Helper Script
#![cfg(not(target_arch = "wasm32"))]
/*!
Plugin Build Helper

This script helps build and manage plugins for the Qorzen framework.

Usage:
    cargo run --bin build_plugins [command] [options]

Commands:
    build [plugin_name]     - Build specific plugin or all plugins
    clean [plugin_name]     - Clean build artifacts
    list                    - List all discovered plugins
    validate [plugin_name]  - Validate plugin manifest
    init <plugin_name>      - Create new plugin template

Examples:
    cargo run --bin build_plugins build product_catalog
    cargo run --bin build_plugins build  # Build all plugins
    cargo run --bin build_plugins init my_plugin
*/

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        exit(1);
    }

    match args[1].as_str() {
        "build" => {
            let plugin_name = args.get(2);
            build_plugins(plugin_name.map(|s| s.as_str()));
        }
        "clean" => {
            let plugin_name = args.get(2);
            clean_plugins(plugin_name.map(|s| s.as_str()));
        }
        "list" => {
            list_plugins();
        }
        "validate" => {
            let plugin_name = args.get(2);
            validate_plugins(plugin_name.map(|s| s.as_str()));
        }
        "init" => {
            if let Some(plugin_name) = args.get(2) {
                init_plugin(plugin_name);
            } else {
                eprintln!("Error: Plugin name required for init command");
                exit(1);
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", args[1]);
            print_help();
            exit(1);
        }
    }
}

fn print_help() {
    println!(r#"
Plugin Build Helper for Qorzen Framework

USAGE:
    cargo run --bin build_plugins [COMMAND] [OPTIONS]

COMMANDS:
    build [PLUGIN]     Build specific plugin or all plugins
    clean [PLUGIN]     Clean build artifacts for plugin(s)
    list               List all discovered plugins
    validate [PLUGIN]  Validate plugin manifest(s)
    init <PLUGIN>      Create new plugin from template
    help               Show this help message

EXAMPLES:
    cargo run --bin build_plugins build product_catalog
    cargo run --bin build_plugins build
    cargo run --bin build_plugins clean
    cargo run --bin build_plugins init my_new_plugin
    cargo run --bin build_plugins list
"#);
}

fn get_target_dir() -> PathBuf {
    // First try CARGO_TARGET_DIR env var
    if let Ok(env_path) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(env_path);
    }

    // Then try to load .cargo/config.toml manually
    let config_path = Path::new(".cargo").join("config.toml");
    if let Ok(contents) = std::fs::read_to_string(config_path) {
        if let Ok(toml_val) = contents.parse::<toml::Value>() {
            if let Some(target_dir) = toml_val
                .get("build")
                .and_then(|b| b.get("target-dir"))
                .and_then(|v| v.as_str())
            {
                return PathBuf::from(target_dir);
            }
        }
    }

    // Fallback
    PathBuf::from("target")
}

fn build_plugins(plugin_name: Option<&str>) {
    let plugins_dir = Path::new("plugins");

    if !plugins_dir.exists() {
        eprintln!("Error: plugins directory does not exist");
        exit(1);
    }

    match plugin_name {
        Some(name) => build_single_plugin(name),
        None => build_all_plugins(),
    }
}

fn build_single_plugin(plugin_name: &str) {
    let plugin_path = Path::new("plugins").join(plugin_name);

    if !plugin_path.exists() {
        eprintln!("Error: Plugin '{}' not found", plugin_name);
        exit(1);
    }

    println!("Building plugin: {}", plugin_name);

    if let Err(e) = build_plugin_at_path(&plugin_path) {
        eprintln!("Error building plugin '{}': {}", plugin_name, e);
        exit(1);
    }

    println!("Successfully built plugin: {}", plugin_name);
}

fn build_all_plugins() {
    let plugins_dir = Path::new("plugins");
    let mut built_count = 0;
    let mut failed_count = 0;

    for entry in fs::read_dir(plugins_dir).unwrap() {
        let entry = entry.unwrap();
        let plugin_path = entry.path();

        if plugin_path.is_dir() {
            let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
            println!("Building plugin: {}", plugin_name);

            match build_plugin_at_path(&plugin_path) {
                Ok(()) => {
                    println!("✓ Successfully built: {}", plugin_name);
                    built_count += 1;
                }
                Err(e) => {
                    eprintln!("✗ Failed to build {}: {}", plugin_name, e);
                    failed_count += 1;
                }
            }
        }
    }

    println!("\nBuild Summary:");
    println!("  Built: {}", built_count);
    println!("  Failed: {}", failed_count);

    if failed_count > 0 {
        exit(1);
    }
}

fn build_plugin_at_path(plugin_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_toml = plugin_path.join("Cargo.toml");

    if !cargo_toml.exists() {
        return Err("No Cargo.toml found in plugin directory".into());
    }

    let mut cmd = Command::new("cargo");
    cmd.args(&["build", "--release"])
        .current_dir(plugin_path);

    // Manually set the target dir if defined
    cmd.env("CARGO_TARGET_DIR", get_target_dir());

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Cargo build failed:\n{}", stderr).into());
    }

    // Copy built library to plugin directory
    copy_built_library(plugin_path)?;

    Ok(())
}

fn copy_built_library(plugin_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();

    let target_base = get_target_dir();

    let target_release_dir = target_base.join("release");

    let (prefix, extension) = if cfg!(target_os = "windows") {
        ("", "dll")
    } else if cfg!(target_os = "macos") {
        ("lib", "dylib")
    } else {
        ("lib", "so")
    };

    let lib_name = format!("{}{}.{}", prefix, plugin_name, extension);
    let source_path = target_release_dir.join(&lib_name);
    let dest_path = plugin_path.join(&lib_name);

    if source_path.exists() {
        std::fs::copy(&source_path, &dest_path)?;
        println!("  Copied library: {}", lib_name);
    } else {
        return Err(format!("Built library not found: {}", source_path.display()).into());
    }

    Ok(())
}

fn clean_plugins(plugin_name: Option<&str>) {
    match plugin_name {
        Some(name) => clean_single_plugin(name),
        None => clean_all_plugins(),
    }
}

fn clean_single_plugin(plugin_name: &str) {
    let plugin_path = Path::new("plugins").join(plugin_name);

    if !plugin_path.exists() {
        eprintln!("Error: Plugin '{}' not found", plugin_name);
        exit(1);
    }

    clean_plugin_at_path(&plugin_path, plugin_name);
}

fn clean_all_plugins() {
    let plugins_dir = Path::new("plugins");

    for entry in fs::read_dir(plugins_dir).unwrap() {
        let entry = entry.unwrap();
        let plugin_path = entry.path();

        if plugin_path.is_dir() {
            let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
            clean_plugin_at_path(&plugin_path, &plugin_name);
        }
    }
}

fn clean_plugin_at_path(plugin_path: &Path, plugin_name: &str) {
    println!("Cleaning plugin: {}", plugin_name);

    // Clean cargo target directory
    let target_base = get_target_dir();

    if target_base.exists() {
        if let Err(e) = fs::remove_dir_all(&target_base) {
            eprintln!("Warning: Failed to remove target directory: {}", e);
        }
    }

    // Remove built libraries
    let extensions = ["dll", "dylib", "so"];
    for ext in &extensions {
        let lib_patterns = [
            format!("lib{}.{}", plugin_name, ext),
            format!("{}.{}", plugin_name, ext),
        ];

        for pattern in &lib_patterns {
            let lib_path = plugin_path.join(pattern);
            if lib_path.exists() {
                if let Err(e) = fs::remove_file(&lib_path) {
                    eprintln!("Warning: Failed to remove {}: {}", pattern, e);
                } else {
                    println!("  Removed: {}", pattern);
                }
            }
        }
    }
}

fn list_plugins() {
    let plugins_dir = Path::new("plugins");

    if !plugins_dir.exists() {
        println!("No plugins directory found");
        return;
    }

    println!("Discovered plugins:");

    for entry in fs::read_dir(plugins_dir).unwrap() {
        let entry = entry.unwrap();
        let plugin_path = entry.path();

        if plugin_path.is_dir() {
            let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
            let manifest_path = plugin_path.join("plugin.toml");
            let cargo_path = plugin_path.join("Cargo.toml");

            print!("  {} ", plugin_name);

            if manifest_path.exists() {
                print!("✓ manifest ");
            } else {
                print!("✗ manifest ");
            }

            if cargo_path.exists() {
                print!("✓ cargo ");
            } else {
                print!("✗ cargo ");
            }

            // Check for built library
            let has_library = check_for_built_library(&plugin_path);
            if has_library {
                print!("✓ built");
            } else {
                print!("✗ built");
            }

            println!();
        }
    }
}

fn check_for_built_library(plugin_path: &Path) -> bool {
    let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
    let extensions = ["dll", "dylib", "so"];

    for ext in &extensions {
        let lib_patterns = [
            format!("lib{}.{}", plugin_name, ext),
            format!("{}.{}", plugin_name, ext),
        ];

        for pattern in &lib_patterns {
            if plugin_path.join(pattern).exists() {
                return true;
            }
        }
    }

    false
}

fn validate_plugins(plugin_name: Option<&str>) {
    match plugin_name {
        Some(name) => validate_single_plugin(name),
        None => validate_all_plugins(),
    }
}

fn validate_single_plugin(plugin_name: &str) {
    let plugin_path = Path::new("plugins").join(plugin_name);

    if !plugin_path.exists() {
        eprintln!("Error: Plugin '{}' not found", plugin_name);
        exit(1);
    }

    validate_plugin_at_path(&plugin_path, plugin_name);
}

fn validate_all_plugins() {
    let plugins_dir = Path::new("plugins");

    for entry in fs::read_dir(plugins_dir).unwrap() {
        let entry = entry.unwrap();
        let plugin_path = entry.path();

        if plugin_path.is_dir() {
            let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
            validate_plugin_at_path(&plugin_path, &plugin_name);
        }
    }
}

fn validate_plugin_at_path(plugin_path: &Path, plugin_name: &str) {
    println!("Validating plugin: {}", plugin_name);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check for required files
    let manifest_path = plugin_path.join("plugin.toml");
    let cargo_path = plugin_path.join("Cargo.toml");
    let src_path = plugin_path.join("src").join("lib.rs");

    if !manifest_path.exists() {
        errors.push("Missing plugin.toml manifest file");
    }

    if !cargo_path.exists() {
        errors.push("Missing Cargo.toml build file");
    }

    if !src_path.exists() {
        errors.push("Missing src/lib.rs source file");
    }

    let mut errors: Vec<String> = Vec::new();

    // Validate manifest content if it exists
    if manifest_path.exists() {
        if let Err(e) = validate_manifest(&manifest_path) {
            errors.push(format!("Invalid manifest: {}", e));
        }
    }

    // Check for built library
    if !check_for_built_library(plugin_path) {
        warnings.push("No built library found - run build command");
    }

    // Print results
    if errors.is_empty() && warnings.is_empty() {
        println!("  ✓ Plugin is valid");
    } else {
        if !errors.is_empty() {
            for error in errors {
                println!("  ✗ Error: {}", error);
            }
        }

        if !warnings.is_empty() {
            for warning in warnings {
                println!("  ⚠ Warning: {}", warning);
            }
        }
    }
}

fn validate_manifest(manifest_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    // Basic validation - check for required fields
    let required_fields = ["id", "name", "version"];

    for field in &required_fields {
        if !content.contains(&format!("{} =", field)) {
            return Err(format!("Missing required field: {}", field));
        }
    }

    Ok(())
}

fn init_plugin(plugin_name: &str) {
    let plugin_path = Path::new("plugins").join(plugin_name);

    if plugin_path.exists() {
        eprintln!("Error: Plugin '{}' already exists", plugin_name);
        exit(1);
    }

    println!("Creating plugin: {}", plugin_name);

    // Create plugin directory
    fs::create_dir_all(&plugin_path).unwrap();
    fs::create_dir_all(plugin_path.join("src")).unwrap();

    // Create Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
name = "{}"
crate-type = ["cdylib", "rlib"]

[dependencies]
qorzen_oxide = {{ path = "../../", features = ["default"] }}
dioxus = {{ version = "0.6", features = ["macro", "html", "desktop", "router"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
async-trait = "0.1"
tokio = {{ version = "1.0", features = ["macros", "rt", "sync"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
uuid = {{ version = "1.0", features = ["v4", "serde"] }}
tracing = "0.1"
"#, plugin_name, plugin_name);

    fs::write(plugin_path.join("Cargo.toml"), cargo_toml).unwrap();

    // Create plugin.toml
    let plugin_toml = format!(r#"[plugin]
id = "{}"
name = "{}"
version = "0.1.0"
description = "A new plugin for the Qorzen framework"
author = "Your Name"
license = "MIT"
api_version = "0.1.0"

[build]
entry = "src/lib.rs"
library_name = "{}"

[permissions]
required = [
    {{ resource = "ui", action = "render", scope = "Global" }}
]
"#, plugin_name, plugin_name.replace('_', " ").replace('-', " "), plugin_name);

    fs::write(plugin_path.join("plugin.toml"), plugin_toml).unwrap();

    // Create src/lib.rs
    let lib_rs = format!(r#"use qorzen_oxide::plugin::*;
use qorzen_oxide::{{plugin, export_plugin}};

plugin! {{
    id: "{}",
    name: "{}",
    version: "0.1.0",
    author: "Your Name",
    description: "A new plugin for the Qorzen framework",
    permissions: ["ui.render"],

    impl {{
        async fn initialize(&mut self, context: PluginContext) -> qorzen_oxide::error::Result<()> {{
            self.context = Some(context);
            tracing::info!("Plugin '{}' initialized successfully");
            Ok(())
        }}

        async fn shutdown(&mut self) -> qorzen_oxide::error::Result<()> {{
            tracing::info!("Plugin '{}' shutting down");
            Ok(())
        }}

        fn ui_components(&self) -> Vec<UIComponent> {{
            vec![]
        }}

        fn menu_items(&self) -> Vec<MenuItem> {{
            vec![]
        }}

        fn settings_schema(&self) -> Option<qorzen_oxide::config::SettingsSchema> {{
            None
        }}

        fn api_routes(&self) -> Vec<ApiRoute> {{
            vec![]
        }}

        fn event_handlers(&self) -> Vec<EventHandler> {{
            vec![]
        }}

        fn render_component(&self, component_id: &str, props: serde_json::Value)
            -> qorzen_oxide::error::Result<dioxus::prelude::VNode> {{
            Err(qorzen_oxide::error::Error::plugin(&self.info().id, "No components implemented"))
        }}

        async fn handle_api_request(&self, route_id: &str, request: ApiRequest)
            -> qorzen_oxide::error::Result<ApiResponse> {{
            Err(qorzen_oxide::error::Error::plugin(&self.info().id, "No API routes implemented"))
        }}

        async fn handle_event(&self, handler_id: &str, event: &dyn qorzen_oxide::event::Event)
            -> qorzen_oxide::error::Result<()> {{
            Ok(())
        }}
    }}
}}
"#, plugin_name, plugin_name.replace('_', " ").replace('-', " "), plugin_name, plugin_name);

    fs::write(plugin_path.join("src").join("lib.rs"), lib_rs).unwrap();

    println!("✓ Plugin '{}' created successfully", plugin_name);
    println!("  Next steps:");
    println!("    1. cd plugins/{}", plugin_name);
    println!("    2. Implement your plugin logic in src/lib.rs");
    println!("    3. cargo run --bin build_plugins build {}", plugin_name);
}