#!/usr/bin/env bash
set -euo pipefail

echo "🧹 Cleaning build artifacts..."
cargo clean
cargo run --bin build_plugins --features native-only -- clean product_catalog

echo "🔨 Building plugin: product_catalog..."
cargo run --bin build_plugins --features native-only -- build product_catalog

echo "📦 Installing plugin: product_catalog..."
cargo run --bin build_plugins --features native-only -- install product_catalog

echo "📋 Listing installed plugins..."
cargo run --bin build_plugins --features native-only -- list

echo "🚀 Running application..."
cargo run --features desktop --bin qorzen_desktop -- --debug --verbose

echo "✅ All steps completed successfully."