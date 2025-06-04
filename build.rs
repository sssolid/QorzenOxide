// build.rs - Build Script for Plugin System

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let exe_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3) // Traverse from OUT_DIR to target/debug/
        .expect("Failed to determine target directory");

    let source = Path::new("public/static");
    let destination = exe_dir.join("static");

    if destination.exists() {
        fs::remove_dir_all(&destination).expect("Failed to clean old static folder");
    }

    copy_dir_recursive(source, &destination).expect("Failed to copy static assets");

    // Generate plugin registry based on target
    if target_arch == "wasm32" {
        generate_wasm_plugin_registry(&out_dir);
    } else {
        generate_native_plugin_registry(&out_dir);
    }

    // Build native plugins if we're not building for WASM
    // if target_arch != "wasm32" {
    //     build_native_plugins();
    // }

    // Tell cargo to rerun if plugins directory changes
    println!("cargo:rerun-if-changed=plugins,public/static");
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Generate plugin registry for WASM (compile-time inclusion)
fn generate_wasm_plugin_registry(out_dir: &str) {
    let plugins_dir = Path::new("plugins");
    let mut registry_code = String::new();

    registry_code.push_str(r#"
// Auto-generated plugin registry for WASM
use crate::plugin::registry::{PluginFactoryRegistry, SimplePluginFactory};
use crate::error::Result;

pub async fn register_wasm_plugins() -> Result<()> {
"#);

    if plugins_dir.exists() {
        for entry in fs::read_dir(plugins_dir).unwrap() {
            let entry = entry.unwrap();
            let plugin_path = entry.path();

            if plugin_path.is_dir() {
                let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
                let manifest_path = plugin_path.join("plugin.toml");

                if manifest_path.exists() {
                    if let Ok(manifest_content) = fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = parse_plugin_manifest(&manifest_content) {
                            // Generate registration code for this plugin
                            registry_code.push_str(&format!(r#"
    // Register {plugin_name} plugin
    {{
        use {plugin_name}::{plugin_name}Plugin;
        let factory = SimplePluginFactory::<{plugin_name}Plugin>::new(
            {plugin_name}Plugin::default().info()
        );
        PluginFactoryRegistry::register(factory).await?;
    }}
"#, plugin_name = manifest.id));
                        }
                    }
                }
            }
        }
    }

    registry_code.push_str(r#"
    Ok(())
}
"#);

    let registry_path = Path::new(out_dir).join("wasm_plugin_registry.rs");
    fs::write(registry_path, registry_code).unwrap();
}

/// Generate plugin registry for native (dynamic loading)
fn generate_native_plugin_registry(out_dir: &str) {
    let registry_code = r#"
// Auto-generated plugin registry for native
use crate::error::Result;

pub async fn register_native_plugins() -> Result<()> {
    // Native plugins are loaded dynamically from the filesystem
    // No compile-time registration needed
    Ok(())
}
"#;

    let registry_path = Path::new(out_dir).join("native_plugin_registry.rs");
    fs::write(registry_path, registry_code).unwrap();
}

/// Build native plugins
fn build_native_plugins() {
    let plugins_dir = Path::new("plugins");

    if !plugins_dir.exists() {
        return;
    }

    for entry in fs::read_dir(plugins_dir).expect("Failed to read plugin directory") {
        let entry = entry.expect("Failed to read directory entry");
        let plugin_path = entry.path();

        if plugin_path.is_dir() {
            let cargo_toml = plugin_path.join("Cargo.toml");

            if cargo_toml.exists() {
                println!("Building plugin: {:?}", plugin_path);

                let output = Command::new("cargo")
                    .args(&["build", "--release"])
                    .current_dir(&plugin_path)
                    .output();

                match output {
                    Ok(output) => {
                        if !output.status.success() {
                            println!("Warning: Failed to build plugin {:?}", plugin_path);
                            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                        } else {
                            // Copy built library to plugin directory
                            copy_plugin_library(&plugin_path);
                        }
                    }
                    Err(e) => {
                        println!("Warning: Failed to execute cargo for plugin {:?}: {}", plugin_path, e);
                    }
                }
            }
        }
    }
}

/// Copy built plugin library to plugin directory
fn copy_plugin_library(plugin_path: &Path) {
    let plugin_name = plugin_path.file_name().unwrap().to_string_lossy();
    let target_dir = plugin_path.join("target").join("release");

    if !target_dir.exists() {
        return;
    }

    // Determine library extension based on platform
    let (prefix, extension) = if cfg!(target_os = "windows") {
        ("", "dll")
    } else if cfg!(target_os = "macos") {
        ("lib", "dylib")
    } else {
        ("lib", "so")
    };

    let lib_name = format!("{}{}.{}", prefix, plugin_name, extension);
    let source_path = target_dir.join(&lib_name);
    let dest_path = plugin_path.join(&lib_name);

    if source_path.exists() {
        if let Err(e) = fs::copy(&source_path, &dest_path) {
            println!("Warning: Failed to copy plugin library: {}", e);
        } else {
            println!("Copied plugin library: {:?}", dest_path);
        }
    }
}

/// Simple manifest parser (minimal implementation)
fn parse_plugin_manifest(content: &str) -> Result<PluginManifest, Box<dyn std::error::Error>> {
    // This is a simplified parser - in a real implementation you'd use toml crate
    let mut manifest = PluginManifest {
        id: String::new(),
        name: String::new(),
        version: String::new(),
    };

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("id = ") {
            manifest.id = line.split('=').nth(1).unwrap().trim().trim_matches('"').to_string();
        } else if line.starts_with("name = ") {
            manifest.name = line.split('=').nth(1).unwrap().trim().trim_matches('"').to_string();
        } else if line.starts_with("version = ") {
            manifest.version = line.split('=').nth(1).unwrap().trim().trim_matches('"').to_string();
        }
    }

    if manifest.id.is_empty() {
        return Err("Plugin manifest missing id".into());
    }

    Ok(manifest)
}

#[derive(Debug)]
struct PluginManifest {
    id: String,
    name: String,
    version: String,
}