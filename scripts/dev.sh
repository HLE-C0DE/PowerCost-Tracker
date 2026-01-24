#!/bin/bash
# PowerCost Tracker - Development Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Starting PowerCost Tracker in development mode..."
echo "==================================================="

# Check prerequisites
command -v cargo >/dev/null 2>&1 || { echo "Error: Rust/Cargo not installed"; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "Error: Node.js/npm not installed"; exit 1; }

# Install frontend dependencies if needed
if [ ! -d "$PROJECT_DIR/ui/node_modules" ]; then
    echo "Installing frontend dependencies..."
    cd "$PROJECT_DIR/ui"
    npm install
fi

# Start development server
cd "$PROJECT_DIR"
cargo tauri dev
