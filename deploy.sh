#!/bin/bash
# Tokyo RSS Reader - Quick Deploy Script

echo "🦀 Tokyo RSS Reader Deployment"
echo "=============================="

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "📦 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
else
    echo "✅ Rust is already installed"
fi

# Build and install
echo "🔨 Building Tokyo RSS Reader..."
cargo build --release

echo "📲 Installing Tokyo RSS Reader..."
cargo install --path . --force

# Create launcher script
echo "🚀 Creating launcher..."
mkdir -p ~/.local/bin
cat > ~/.local/bin/rss << 'EOF'
#!/bin/bash
# Tokyo RSS Reader Launcher
exec ~/.cargo/bin/tokyo-rss "$@"
EOF
chmod +x ~/.local/bin/rss

echo ""
echo "✅ Installation complete!"
echo ""
echo "🎯 Usage:"
echo "  tokyo-rss    # Run directly"
echo "  rss          # Run with alias"
echo ""
echo "🔧 Controls:"
echo "  r     - Refresh feeds (background)"
echo "  o     - Open article in browser"
echo "  m     - Mark as read"
echo "  Tab   - Switch panes"
echo "  ↑/↓   - Navigate"
echo "  q     - Quit"
echo ""
