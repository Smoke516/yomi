#!/bin/bash

# Tokyo RSS Reader Installation Script
# This script installs tokyo-rss to ~/.local/bin and sets up the config

set -e

echo "🦀 Tokyo RSS Reader - Installation Script"
echo "=========================================="

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: cargo not found. Please install Rust first."
    exit 1
fi

# Build in release mode
echo "📦 Building tokyo-rss in release mode..."
cargo build --release

# Create ~/.local/bin if it doesn't exist
mkdir -p ~/.local/bin

# Copy the binary
echo "📋 Installing to ~/.local/bin/tokyo-rss..."
cp target/release/tokyo-rss ~/.local/bin/

# Make sure ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  Note: ~/.local/bin is not in your PATH"
    echo "   Add this line to your ~/.bashrc or ~/.zshrc:"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Create config directory and sample config
echo "⚙️  Setting up configuration..."
mkdir -p ~/.config/tokyo-rss

# Create default config if it doesn't exist
if [[ ! -f ~/.config/tokyo-rss/config.toml ]]; then
    cat > ~/.config/tokyo-rss/config.toml << 'EOF'
[feeds]
"Hacker News" = "https://feeds.feedburner.com/TheHackersNews"
"BleepingComputer" = "https://www.bleepingcomputer.com/feed/"
"TechCrunch" = "https://techcrunch.com/feed/"
"Ars Technica" = "http://feeds.arstechnica.com/arstechnica/index/"
"The Verge" = "https://www.theverge.com/rss/index.xml"
EOF
    echo "✅ Created default config at ~/.config/tokyo-rss/config.toml"
else
    echo "✅ Config file already exists"
fi

# Create wrapper script
echo "🔗 Creating launcher script..."
cat > ~/.local/bin/rss << 'EOF'
#!/bin/bash
# Tokyo RSS Reader Launcher
exec ~/.local/bin/tokyo-rss "$@"
EOF
chmod +x ~/.local/bin/rss

echo ""
echo "🎉 Installation Complete!"
echo ""
echo "Usage:"
echo "  tokyo-rss    # Run the full name"
echo "  rss          # Run with short alias"
echo ""
echo "Key Controls:"
echo "  q     - Quit"
echo "  h     - Help"
echo "  r     - Refresh feeds"
echo "  Tab   - Switch between panes"
echo "  ↑/↓   - Navigate"
echo "  Enter - Read article"
echo "  Esc   - Go back"
echo ""
echo "Config file: ~/.config/tokyo-rss/config.toml"
echo "🎨 Enjoy your beautiful Tokyo Night RSS reader!"
