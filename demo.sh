#!/bin/bash

# Tokyo RSS Reader Demo Script
# Shows off the features and capabilities

echo "🎌 Tokyo RSS Reader Demo"
echo "========================"
echo ""

echo "📦 Build Information:"
echo "   Binary size: $(du -h ~/.local/bin/tokyo-rss | cut -f1)"
echo "   Config: ~/.config/tokyo-rss/config.toml"
echo "   Available as: tokyo-rss, rss"
echo ""

echo "📰 Current Feed Configuration:"
cat ~/.config/tokyo-rss/config.toml
echo ""

echo "🎮 Usage Examples:"
echo "   rss           # Quick launch"
echo "   tokyo-rss     # Full name"
echo ""

echo "🔧 Key Controls:"
echo "   q     - Quit"
echo "   h     - Help popup"
echo "   r     - Refresh feeds"
echo "   Tab   - Switch between feed list and article list"
echo "   ↑/↓   - Navigate lists"
echo "   Enter - Read selected article"
echo "   Esc   - Go back"
echo ""

echo "🎨 Tokyo Night Features:"
echo "   • Authentic Tokyo Night color palette"
echo "   • Beautiful rounded borders"  
echo "   • Status indicators (⏳ loading, ✓ loaded, ❌ error)"
echo "   • Emoji icons for visual appeal"
echo "   • Responsive split-pane layout"
echo ""

echo "⚡ Technical Features:"
echo "   • Async RSS/Atom parsing"
echo "   • Speed optimized (20 articles per feed)"
echo "   • Automatic feed refresh (10 min)"
echo "   • HTML tag stripping"
echo "   • Scrollable article view"
echo "   • Memory efficient (Rust)"
echo ""

echo "🚀 Ready to launch! Try:"
echo "   ./rss"
echo ""
echo "   (Press 'q' to quit when you're done exploring)"
