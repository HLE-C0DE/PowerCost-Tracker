#!/bin/bash
# PowerCost Tracker - Build Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Building PowerCost Tracker..."
echo "=============================="

# Check prerequisites
command -v cargo >/dev/null 2>&1 || { echo "Error: Rust/Cargo not installed"; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "Error: Node.js/npm not installed"; exit 1; }

# Install frontend dependencies
echo ""
echo "Installing frontend dependencies..."
cd "$PROJECT_DIR/ui"
npm install

# Build the application
echo ""
echo "Building Tauri application..."
cd "$PROJECT_DIR"
cargo tauri build

echo ""
echo "Build complete!"
echo "Output: src-tauri/target/release/"
