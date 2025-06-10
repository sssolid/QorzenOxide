#!/usr/bin/env bash
set -euo pipefail

echo "🧹 Cleaning build artifacts..."
cargo clean
cargo run --bin build_plugins --features native-only -- clean

echo "🔨 Building all plugins..."
cargo run --bin build_plugins --features native-only -- build

echo "📦 Installing all plugins..."
cargo run --bin build_plugins --features native-only -- install

echo "📋 Listing installed plugins..."
cargo run --bin build_plugins --features native-only -- list

echo "🚀 Running application..."
cargo run --features desktop --bin qorzen_desktop -- --debug --verbose

echo "✅ All steps completed successfully."