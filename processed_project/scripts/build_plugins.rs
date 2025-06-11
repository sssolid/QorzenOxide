#![cfg(not(target_arch="wasm32"))]use std::env;use std::fs;use std::path::{Path,PathBuf};use std::process::{Command,exit};fn main(){let args:Vec<String>=env::args().collect();if args.len()<2{print_help();exit(1);}match args[1].as_str(){"build"=>{let plugin_name=args.get(2);build_plugins(plugin_name.map(|s|s.as_str()));}"clean"=>{let plugin_name=args.get(2);clean_plugins(plugin_name.map(|s|s.as_str()));}"list"=>{list_plugins();}"validate"=>{let plugin_name=args.get(2);validate_plugins(plugin_name.map(|s|s.as_str()));}"init"=>{if let Some(plugin_name)=args.get(2){init_plugin(plugin_name);}else{eprintln!("Error: Plugin name required for init command");exit(1);}}"install"=>{let plugin_name=args.get(2);install_plugins(plugin_name.map(|s|s.as_str()));}"help"|"--help"|"-h"=>{print_help();}_=>{eprintln!("Error: Unknown command '{}'",args[1]);print_help();exit(1);}}}fn print_help(){println!(r#"
Plugin Build Helper for Qorzen Framework

USAGE:
    cargo run --bin build_plugins [COMMAND] [OPTIONS]

COMMANDS:
    build [PLUGIN]     Build specific plugin or all plugins
    clean [PLUGIN]     Clean build artifacts for plugin(s)
    list               List all discovered plugins
    validate [PLUGIN]  Validate plugin manifest(s)
    init <PLUGIN>      Create new plugin from template
    install [PLUGIN]   Install plugin(s) to system directory
    help               Show this help message

EXAMPLES:
    cargo run --bin build_plugins build product_catalog
    cargo run --bin build_plugins build
    cargo run --bin build_plugins install product_catalog
    cargo run --bin build_plugins clean
    cargo run --bin build_plugins init my_new_plugin
    cargo run --bin build_plugins list
"#);}fn get_plugin_data_dir()->PathBuf{if let Ok(env_dir)=std::env::var("QORZEN_PLUGINS_DIR"){return PathBuf::from(env_dir);}#[cfg(target_os="linux")]{if let Some(home)=dirs::home_dir(){return home.join(".local/share/qorzen/plugins");}}#[cfg(target_os="macos")]{if let Some(data_dir)=dirs::data_dir(){return data_dir.join("qorzen/plugins");}}#[cfg(target_os="windows")]{if let Some(data_dir)=dirs::data_dir(){return data_dir.join("Qorzen/plugins");}}PathBuf::from("./target/plugins")}fn get_target_dir()->PathBuf{if let Ok(env_path)=std::env::var("CARGO_TARGET_DIR"){return PathBuf::from(env_path);}let config_path=Path::new(".cargo").join("config.toml");if let Ok(contents)=std::fs::read_to_string(config_path){if let Ok(toml_val)=contents.parse::<toml::Value>(){if let Some(target_dir)=toml_val.get("build").and_then(|b|b.get("target-di__STRING_LITERAL_1__target")}fn get_library_patterns(plugin_name:&str)->Vec<String>{let mut patterns=Vec::new();#[cfg(target_os="windows")]{patterns.push(format!("{}.dll",plugin_name));patterns.push(format!("lib{}.dll",plugin_name));}#[cfg(target_os="macos")]{patterns.push(format!("lib{}.dylib",plugin_name));}#[cfg(target_os="linux")]{patterns.push(format!("lib{}.so",plugin_name));}patterns.push(format!("lib{}.so",plugin_name));patterns.push(format!("lib{}.dylib",plugin_name));patterns.push(format!("{}.dll",plugin_name));patterns}fn build_plugins(plugin_name:Option<&str>){let plugins_dir=Path::new("plugins");if!plugins_dir.exists(){eprintln!("Error: plugins directory does not exist");exit(1);}match plugin_name{Some(name)=>build_single_plugin(name),None=>build_all_plugins(),}}fn build_single_plugin(plugin_name:&str){let plugin_path=Path::new("plugins").join(plugin_name);if!plugin_path.exists(){eprintln!("Error: Plugin '{}' not found",plugin_name);exit(1);}println!("Building plugin: {}",plugin_name);if let Err(e)=build_plugin_at_path(&plugin_path){eprintln!("Error building plugin '{}': {}",plugin_name,e);exit(1);}println!("Successfully built plugin: {}",plugin_name);}fn build_all_plugins(){let plugins_dir=Path::new("plugins");let mut built_count=0;let mut failed_count=0;for entry in fs::read_dir(plugins_dir).unwrap(){let entry=entry.unwrap();let plugin_path=entry.path();if plugin_path.is_dir(){let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();println!("Building plugin: {}",plugin_name);match build_plugin_at_path(&plugin_path){Ok(())=>{println!("✓ Successfully built: {}",plugin_name);built_count +=1;}Err(e)=>{eprintln!("✗ Failed to build {}: {}",plugin_name,e);failed_count +=1;}}}}println!("\nBuild Summary:");println!("  Built: {}",built_count);println!("  Failed: {}",failed_count);if failed_count>0{exit(1);}}fn build_plugin_at_path(plugin_path:&Path)->Result<(),Box<dyn std::error::Error>>{let cargo_toml=plugin_path.join("Cargo.toml");if!cargo_toml.exists(){return Err("No Cargo.toml found in plugin directory".into());}let manifest_path=plugin_path.join("plugin.toml");if manifest_path.exists(){validate_manifest(&manifest_path)?;}let mut cmd=Command::new("cargo");cmd.args(&["build","--release"]).current_dir(plugin_path);let output=cmd.output()?;if!output.status.success(){let stderr=String::from_utf8_lossy(&output.stderr);return Err(format!("Cargo build failed:\n{}",stderr).into());}Ok(())}fn install_plugins(plugin_name:Option<&str>){match plugin_name{Some(name)=>install_single_plugin(name),None=>install_all_plugins(),}}fn install_single_plugin(plugin_name:&str){let plugin_path=Path::new("plugins").join(plugin_name);if!plugin_path.exists(){eprintln!("Error: Plugin '{}' not found",plugin_name);exit(1);}println!("Installing plugin: {}",plugin_name);if let Err(e)=install_plugin_at_path(&plugin_path){eprintln!("Error installing plugin '{}': {}",plugin_name,e);exit(1);}println!("Successfully installed plugin: {}",plugin_name);}fn install_all_plugins(){let plugins_dir=Path::new("plugins");let mut installed_count=0;let mut failed_count=0;for entry in fs::read_dir(plugins_dir).unwrap(){let entry=entry.unwrap();let plugin_path=entry.path();if plugin_path.is_dir(){let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();println!("Installing plugin: {}",plugin_name);match install_plugin_at_path(&plugin_path){Ok(())=>{println!("✓ Successfully installed: {}",plugin_name);installed_count +=1;}Err(e)=>{eprintln!("✗ Failed to install {}: {}",plugin_name,e);failed_count +=1;}}}}println!("\nInstall Summary:");println!("  Installed: {}",installed_count);println!("  Failed: {}",failed_count);if failed_count>0{exit(1);}}fn install_plugin_at_path(plugin_path:&Path)->Result<(),Box<dyn std::error::Error>>{let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();let data_dir=get_plugin_data_dir();let plugin_data_dir=data_dir.join(&*plugin_name);fs::create_dir_all(&plugin_data_dir)?;let manifest_src=plugin_path.join("plugin.toml");let manifest_dst=plugin_data_dir.join("plugin.toml");if manifest_src.exists(){fs::copy(&manifest_src,&manifest_dst)?;println!("  Copied manifest: plugin.toml");}copy_built_library_to_data_dir(&plugin_name,&plugin_data_dir)?;for entry in["README.md","LICENSE","assets/"]{let src_path=plugin_path.join(entry);let dst_path=plugin_data_dir.join(entry);if src_path.exists(){if src_path.is_dir(){copy_dir_all(&src_path,&dst_path)?;}else{if let Some(parent)=dst_path.parent(){fs::create_dir_all(parent)?;}fs::copy(&src_path,&dst_path)?;}println!("  Copied: {}",entry);}}println!("  Plugin installed to: {}",plugin_data_dir.display());Ok(())}fn copy_built_library_to_data_dir(plugin_name:&str,plugin_data_dir:&Path)->Result<(),Box<dyn std::error::Error>>{let patterns=get_library_patterns(plugin_name);let mut tried_paths=Vec::new();let plugin_target_dir=Path::new("plugins").join(plugin_name).join("target").join("release");println!("  Looking for library in: {}",plugin_target_dir.display());for pattern in&patterns{let source_path=plugin_target_dir.join(pattern);tried_paths.push(source_path.clone());println!("  Checking: {}",source_path.display());if source_path.exists(){let dest_path=plugin_data_dir.join(pattern);fs::copy(&source_path,&dest_path)?;println!("  Copied library: {} (from plugin target dir)",pattern);return Ok(());}}let main_target_base=get_target_dir();let main_target_release_dir=main_target_base.join("release");println!("  Looking for library in: {}",main_target_release_dir.display());for pattern in&patterns{let source_path=main_target_release_dir.join(pattern);tried_paths.push(source_path.clone());println!("  Checking: {}",source_path.display());if source_path.exists(){let dest_path=plugin_data_dir.join(pattern);fs::copy(&source_path,&dest_path)?;println!("  Copied library: {} (from main target dir)",pattern);return Ok(());}}return Err(format!("Built library not found for plugin '{}'. Tried paths: {:?}",plugin_name,tried_paths).into());}fn copy_dir_all(src:&Path,dst:&Path)->Result<(),Box<dyn std::error::Error>>{fs::create_dir_all(dst)?;for entry in fs::read_dir(src)?{let entry=entry?;let src_path=entry.path();let dst_path=dst.join(entry.file_name());if src_path.is_dir(){copy_dir_all(&src_path,&dst_path)?;}else{fs::copy(&src_path,&dst_path)?;}}Ok(())}fn clean_plugins(plugin_name:Option<&str>){match plugin_name{Some(name)=>clean_single_plugin(name),None=>clean_all_plugins(),}}fn clean_single_plugin(plugin_name:&str){let plugin_path=Path::new("plugins").join(plugin_name);if!plugin_path.exists(){eprintln!("Error: Plugin '{}' not found",plugin_name);exit(1);}clean_plugin_at_path(&plugin_path,plugin_name);}fn clean_all_plugins(){let plugins_dir=Path::new("plugins");for entry in fs::read_dir(plugins_dir).unwrap(){let entry=entry.unwrap();let plugin_path=entry.path();if plugin_path.is_dir(){let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();clean_plugin_at_path(&plugin_path,&plugin_name);}}let data_dir=get_plugin_data_dir();if data_dir.exists(){println!("Cleaning plugin data directory: {}",data_dir.display());if let Err(e)=fs::remove_dir_all(&data_dir){eprintln!("Warning: Failed to remove plugin data directory: {}",e);}}}fn clean_plugin_at_path(plugin_path:&Path,plugin_name:&str){println!("Cleaning plugin: {}",plugin_name);let data_dir=get_plugin_data_dir();let plugin_data_dir=data_dir.join(plugin_name);if plugin_data_dir.exists(){if let Err(e)=fs::remove_dir_all(&plugin_data_dir){eprintln!("Warning: Failed to remove plugin data directory: {}",e);}else{println!("  Removed data directory");}}let plugin_target_dir=plugin_path.join("target");if plugin_target_dir.exists(){if let Err(e)=fs::remove_dir_all(&plugin_target_dir){eprintln!("Warning: Failed to remove plugin target directory: {}",e);}else{println!("  Removed plugin target directory");}}let main_target_base=get_target_dir();if main_target_base.exists(){let main_target_release_dir=main_target_base.join("release");let patterns=get_library_patterns(plugin_name);for pattern in&patterns{let lib_path=main_target_release_dir.join(pattern);if lib_path.exists(){if let Err(e)=fs::remove_file(&lib_path){eprintln!("Warning: Failed to remove {}: {}",pattern,e);}else{println!("  Removed: {}",pattern);}}}}}fn list_plugins(){let plugins_dir=Path::new("plugins");if!plugins_dir.exists(){println!("No plugins directory found");return;}println!("Discovered plugins:");for entry in fs::read_dir(plugins_dir).unwrap(){let entry=entry.unwrap();let plugin_path=entry.path();if plugin_path.is_dir(){let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();let manifest_path=plugin_path.join("plugin.toml");let cargo_path=plugin_path.join("Cargo.toml");print!("  {} ",plugin_name);if manifest_path.exists(){print!("✓ manifest ");}else{print!("✗ manifest ");}if cargo_path.exists(){print!("✓ cargo ");}else{print!("✗ cargo ");}let has_library=check_for_built_library(&plugin_name);if has_library{print!("✓ built ");}else{print!("✗ built ");}let data_dir=get_plugin_data_dir();let plugin_data_dir=data_dir.join(&*plugin_name);if plugin_data_dir.exists(){print!("✓ installed");}else{print!("✗ installed");}println!();}}let data_dir=get_plugin_data_dir();if data_dir.exists(){println!("\nInstalled plugins:");for entry in fs::read_dir(&data_dir).unwrap_or_else(|_|fs::read_dir(".").unwrap()){if let Ok(entry)=entry{let plugin_path=entry.path();if plugin_path.is_dir(){let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();let manifest_path=plugin_path.join("plugin.toml");print!("  {} ",plugin_name);if manifest_path.exists(){print!("✓ manifest ");}else{print!("✗ manifest ");}let has_library=check_for_installed_library(&plugin_path,&plugin_name);if has_library{print!("✓ library");}else{print!("✗ library");}println!();}}}}}fn check_for_built_library(plugin_name:&str)->bool{let patterns=get_library_patterns(plugin_name);let plugin_target_dir=Path::new("plugins").join(plugin_name).join("target").join("release");for pattern in&patterns{if plugin_target_dir.join(pattern).exists(){return true;}}let main_target_base=get_target_dir();let main_target_release_dir=main_target_base.join("release");for pattern in&patterns{if main_target_release_dir.join(pattern).exists(){return true;}}false}fn check_for_installed_library(plugin_path:&Path,plugin_name:&str)->bool{let patterns=get_library_patterns(plugin_name);for pattern in&patterns{if plugin_path.join(pattern).exists(){return true;}}false}fn validate_plugins(plugin_name:Option<&str>){match plugin_name{Some(name)=>validate_single_plugin(name),None=>validate_all_plugins(),}}fn validate_single_plugin(plugin_name:&str){let plugin_path=Path::new("plugins").join(plugin_name);if!plugin_path.exists(){eprintln!("Error: Plugin '{}' not found",plugin_name);exit(1);}validate_plugin_at_path(&plugin_path,plugin_name);}fn validate_all_plugins(){let plugins_dir=Path::new("plugins");for entry in fs::read_dir(plugins_dir).unwrap(){let entry=entry.unwrap();let plugin_path=entry.path();if plugin_path.is_dir(){let plugin_name=plugin_path.file_name().unwrap().to_string_lossy();validate_plugin_at_path(&plugin_path,&plugin_name);}}}fn validate_plugin_at_path(plugin_path:&Path,plugin_name:&str){println!("Validating plugin: {}",plugin_name);let mut errors=Vec::new();let mut warnings=Vec::new();let manifest_path=plugin_path.join("plugin.toml");let cargo_path=plugin_path.join("Cargo.toml");let src_path=plugin_path.join("src").join("lib.rs");if!manifest_path.exists(){errors.push("Missing plugin.toml manifest file".to_string());}if!cargo_path.exists(){errors.push("Missing Cargo.toml build file".to_string());}if!src_path.exists(){errors.push("Missing src/lib.rs source file".to_string());}if manifest_path.exists(){if let Err(e)=validate_manifest(&manifest_path){errors.push(format!("Invalid manifest: {}",e));}}if!check_for_built_library(plugin_name){warnings.push("No built library found - run build command".to_string());}let data_dir=get_plugin_data_dir();let plugin_data_dir=data_dir.join(plugin_name);if!plugin_data_dir.exists(){warnings.push("Plugin not installed - run install command".to_string());}if errors.is_empty()&&warnings.is_empty(){println!("  ✓ Plugin is valid");}else{if!errors.is_empty(){for error in errors{println!("  ✗ Error: {}",error);}}if!warnings.is_empty(){for warning in warnings{println!("  ⚠ Warning: {}",warning);}}}}fn validate_manifest(manifest_path:&Path)->Result<(),String>{let content=fs::read_to_string(manifest_path).map_err(|e|format!("Failed to read manifest: {}",e))?;let parsed:toml::Value=content.parse().map_err(|e|format!("Invalid TOML syntax: {}",e))?;let plugin_section=parsed.get("plugin").ok_or("Missing [plugin] section")?;let required_fields=["id","name","version"];for field in&required_fields{if!plugin_section.get(field).is_some(){return Err(format!("Missing required field in [plugin]: {}",field));}}if let Some(id)=plugin_section.get("id").and_then(|v|v.as_str()){if!id.chars().all(|c|c.is_alphanumeric()||c=='_'||c=='-'){return Err("Plugin ID must contain only alphanumeric characters, underscores, and hyphens".to_string());}}Ok(())}fn init_plugin(plugin_name:&str){let plugin_path=Path::new("plugins").join(plugin_name);if plugin_path.exists(){eprintln!("Error: Plugin '{}' already exists",plugin_name);exit(1);}println!("Creating plugin: {}",plugin_name);fs::create_dir_all(&plugin_path).unwrap();fs::create_dir_all(plugin_path.join("src")).unwrap();let cargo_toml=format!(r#"[package]
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

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
dirs = "5.0"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
"#,plugin_name,plugin_name);fs::write(plugin_path.join("Cargo.toml"),cargo_toml).unwrap();let plugin_toml=format!(r#"[plugin]
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

[[permissions.required]]
resource = "ui"
action = "render"
scope = "Global"
"#,plugin_name,plugin_name.replace('_'," ").replace('-'," "),plugin_name);fs::write(plugin_path.join("plugin.toml"),plugin_toml).unwrap();let lib_rs=format!(r#"//! {} Plugin
//!
//! A plugin for the Qorzen framework.

use qorzen_oxide::plugin::*;
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
"#,plugin_name.replace('_'," ").replace('-'," "),plugin_name,plugin_name.replace('_'," ").replace('-'," "),plugin_name,plugin_name);fs::write(plugin_path.join("src").join("lib.rs"),lib_rs).unwrap();let readme=format!(r#"# {}

A plugin for the Qorzen framework.

## Building

```bash
cargo run --bin build_plugins build {}
```

## Installing

```bash
cargo run --bin build_plugins install {}
```

## Development

1. Make your changes to `src/lib.rs`
2. Build the plugin: `cargo run --bin build_plugins build {}`
3. Install the plugin: `cargo run --bin build_plugins install {}`
4. Test the plugin in the main application
"#,plugin_name.replace('_'," ").replace('-'," "),plugin_name,plugin_name,plugin_name,plugin_name);fs::write(plugin_path.join("README.md"),readme).unwrap();println!("✓ Plugin '{}' created successfully",plugin_name);println!("  Next steps:");println!("    1. cd plugins/{}",plugin_name);println!("    2. Implement your plugin logic in src/lib.rs");println!("    3. cargo run --bin build_plugins build {}",plugin_name);println!("    4. cargo run --bin build_plugins install {}",plugin_name);}
