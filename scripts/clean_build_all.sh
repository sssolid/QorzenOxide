#!/usr/bin/env bash
set -euo pipefail

export RUSTC_WRAPPER=sccache
export SCCACHE_CACHE_SIZE=50G
export SCCACHE_DISABLE_DIST=1

echo "🧹 Cleaning main project..."
cargo clean

echo "🧹 Cleaning all plugin targets..."
for plugin_dir in ./plugins/*/; do
  if [[ -f "${plugin_dir}/Cargo.toml" ]]; then
    echo "  → Cleaning ${plugin_dir}"
    rm -rf "${plugin_dir}/target"
  fi
done

echo "🧹 Cleaning built plugins..."
cargo run --bin build_plugins --features native-only -- clean

echo "🔨 Building all plugins (via build_plugins)..."
cargo run --bin build_plugins --features native-only -- build

echo "📦 Installing all plugins..."
cargo run --bin build_plugins --features native-only -- install

echo "📋 Listing installed plugins..."
cargo run --bin build_plugins --features native-only -- list

echo "🚀 Running application..."
cargo run --features desktop --bin qorzen_desktop -- --debug --verbose

echo "✅ All steps completed successfully."