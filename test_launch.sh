#!/bin/bash

echo "🧪 Testing Tokyo RSS Reader Launch..."
echo "======================================"
echo ""

echo "📍 Checking binary locations:"
echo "   tokyo-rss: $(which tokyo-rss)"
echo "   rss: $(which rss)"
echo ""

echo "📝 Testing launcher script:"
cat /home/seawn/.local/bin/rss
echo ""

echo "🔗 Checking symlink:"
ls -la /home/seawn/.local/bin/tokyo-rss
echo ""

echo "✅ Everything should now point to the enhanced version!"
echo "🚀 Try running: rss"
echo ""
echo "Features in enhanced version:"
echo "  • 3-column layout with live preview"  
echo "  • Status bar with unread counts"
echo "  • Persistent read/unread state"
echo "  • Browser integration (press 'o')"
echo "  • Retry logic and parallel updates"
