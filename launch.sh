#!/bin/bash

# Tokyo RSS Reader Launch Script
# This script provides multiple ways to launch the RSS reader

RSS_DIR="/home/seawn/rust_rss_reader"
BINARY_PATH="/home/seawn/.cargo/bin/tokyo-rss"

echo "🌃 Tokyo RSS Reader Enhanced"
echo "=============================="
echo ""

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Tokyo RSS not found at $BINARY_PATH"
    echo "🔧 Building and installing..."
    cd "$RSS_DIR"
    cargo install --path . --force
    
    if [ $? -eq 0 ]; then
        echo "✅ Installation successful!"
    else
        echo "❌ Installation failed"
        exit 1
    fi
fi

echo "🚀 Starting Tokyo RSS Reader..."
echo "💡 Controls: Tab (switch panes), ↑/↓ (navigate), o (open browser), r (refresh), h (help), q (quit)"
echo ""

# Launch the app
exec "$BINARY_PATH"
