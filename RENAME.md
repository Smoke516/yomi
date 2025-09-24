# 🎌 Project Renamed: Tokyo RSS → Yomi (読み)

## ✨ What Changed

The project has been successfully renamed from **Tokyo RSS** to **Yomi** (読み), which means "reading" in Japanese. This better captures the essence of the application while maintaining the beautiful Tokyo Night aesthetic.

## 📦 Migration Summary

### ✅ **Completed Changes**
- **Binary name**: `tokyo-rss` → `yomi`
- **Project name**: `tokyo-rss` → `yomi` 
- **Version bump**: `0.1.1` → `1.0.0`
- **Config directory**: `~/.config/tokyo-rss/` → `~/.config/yomi/`
- **User agent**: `Tokyo-RSS-Reader/1.0` → `Yomi/1.0`
- **Shell aliases updated**: `rss`, `trss`, `news` now point to `yomi`
- **Project directory**: `rust_rss_reader/` → `yomi/`

### 🎯 **New Commands**
```bash
# Primary command
yomi

# Existing aliases still work
rss
news
trss  # legacy alias
```

### 📁 **Configuration Migration**
- Your existing config and read state were automatically migrated
- Config location: `~/.config/yomi/config.toml`
- State file: `~/.config/yomi/state.toml`

## 🚀 **What's New in v1.0.0**
- **Beautiful Enhanced Interface** with better Tokyo Night theming
- **Vim-like Navigation** (`hjkl` keys + arrow keys)
- **Modern TUI Components** with rich icons and visual hierarchy
- **Context-Aware Help** with organized keyboard shortcuts
- **Enhanced Feed Management** with unread count badges
- **Smart Status Messages** with emoji feedback
- **Improved Article Reading** experience

## 📝 **Updated Features**
- **Three-pane layout** with optimized proportions (28/38/34)
- **Focus indicators** - cyan borders for active panes
- **Rich Unicode icons** throughout the interface
- **Better article display** with visual separators
- **Enhanced help system** with categorized shortcuts

## 🎮 **New Keyboard Shortcuts**
| Key | Action |
|-----|--------|
| `?` | Enhanced help dialog |
| `h/j/k/l` | Vim-style navigation |
| `g/G` | Go to top/bottom |
| `m` | Toggle read/unread |
| `A` | Mark all in feed as read |
| `PageUp/PageDown` | Fast scrolling |

---

**The name "Yomi" (読み) perfectly captures what this app is all about - the art of reading and staying informed through RSS feeds, wrapped in a beautiful Tokyo Night interface.** 🌙

Enjoy your enhanced RSS reading experience! 📰✨