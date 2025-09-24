# 🎌 Yomi (読み) - Beautiful Terminal RSS Reader

A fast, elegant RSS reader for the terminal with Tokyo Night theme. *Yomi* means "reading" in Japanese, perfectly capturing the essence of this beautiful feed reader.

![Tokyo Night Theme](https://user-images.githubusercontent.com/placeholder/tokyo-night-preview.png)

## ✨ Features

- 🎨 **Beautiful Tokyo Night Theme** - Enhanced color scheme with improved contrast
- ⚡ **Fast & Efficient** - Written in Rust with async architecture
- 🖥️ **Enhanced TUI Experience** - Modern 3-pane interface with vim-style navigation
- 📰 **Multiple Feed Support** - RSS/Atom feeds with automatic validation
- 💻 **CLI Commands** - Add, remove, list, and refresh feeds from command line
- ⚙️ **Easy Configuration** - Simple TOML config with persistent state tracking
- 🔄 **Smart Auto-refresh** - Background updates with progress indicators
- 📖 **Advanced Article Reading** - Full-text view with read/unread tracking
- 🌐 **Browser Integration** - Open articles directly in your default browser
- 🔍 **Feed Validation** - Automatic checking when adding new sources

## 🚀 Quick Start

### Installation

```bash
# Clone and install
git clone https://github.com/seawn/yomi.git
cd yomi
cargo install --path .
```

### Usage

```bash
# Start interactive TUI (default)
yomi
# or explicitly
yomi read

# Use CLI commands for feed management
yomi add https://feeds.example.com/rss
yomi list
yomi refresh

# Create an alias for convenience
alias rss='yomi'
```

## 💻 CLI Commands

### Feed Management

```bash
# Add feeds
yomi add https://feeds.example.com/rss          # Auto-detect name
yomi add https://techcrunch.com/feed/ -n "Tech"  # Custom name

# List all feeds
yomi list

# Remove feeds
yomi remove "Feed Name"    # By name
yomi remove 0              # By index

# Refresh feeds
yomi refresh               # All feeds
yomi refresh -f "TechNews" # Specific feed

# Get help
yomi --help
yomi add --help
```

## 🎮 TUI Controls

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `?` | Show help popup |
| `r`/`F5` | Refresh all feeds |
| `Tab` | Switch between panes |
| `↑/↓`/`k/j` | Navigate lists/scroll content |
| `←/→`/`h/l` | Move between panes |
| `Enter` | Read selected article |
| `Esc` | Go back/close popup |
| `o` | Open article in browser |
| `m` | Toggle read/unread status |
| `g`/`G` | Go to top/bottom |
| `A` | Mark all articles as read |

## ⚙️ Configuration

Yomi stores configuration in your system config directory:
- **Linux/macOS**: `~/.config/yomi/`
- **Windows**: `%APPDATA%/yomi/`

### Files
- `config.toml` - Feed configuration
- `state.toml` - Read articles and app state

### config.toml Example

```toml
[feeds]
"Hacker News" = "https://feeds.feedburner.com/TheHackersNews"
"BleepingComputer" = "https://www.bleepingcomputer.com/feed/"
"TechCrunch" = "https://techcrunch.com/feed/"
"Your Custom Feed" = "https://example.com/rss.xml"
```

### Default Feeds

Yomi comes with these feeds pre-configured:
- **Hacker News** - Security and tech news
- **BleepingComputer** - Computer security news
- **TechCrunch** - Technology startup news

## 🎨 Tokyo Night Color Palette

The reader uses the authentic Tokyo Night color scheme:

- **Background**: `#1a1b26`
- **Foreground**: `#c0caf5`
- **Blue**: `#7aa2f7`
- **Purple**: `#bb9af7`
- **Cyan**: `#7dcfff`
- **Yellow**: `#e0af68`
- **Gray**: `#565f89`

## 📖 Enhanced UI Layout

```
┌─────────────┬─────────────────────────────┬─────────────────────┐
│ [RSS] Feeds │ [RSS] Feed Name (20 items)  │ [PREVIEW]           │
│             │                             │                     │
│ [+] Tech(5) │ [ ] New Article Title       │ Article Title       │
│ [+] News(3) │ [R] Read Article            │ ─────────────────── │
│ [~] Loading │ [ ] Another Article         │ https://example.com │
│ [!] Error   │ [R] Old Article             │                     │
│             │ ...                         │ Article description │
│             │                             │ and content preview │
└─────────────┴─────────────────────────────┴─────────────────────┘
│ [RSS] 4 feeds | [NEW] 8 unread | [STATUS] All feeds updated | Help: ? │
└─────────────────────────────────────────────────────────────────────────┘
```

### Status Indicators
- `[+]` Feed loaded successfully
- `[~]` Feed loading/refreshing
- `[!]` Feed error
- `[ ]` Unread article
- `[R]` Read article
- `[*]` Background refresh in progress

## 🔧 Development

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run directly
cargo run
```

### Dependencies

- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `tokio` - Async runtime
- `feed-rs` - RSS/Atom parser
- `reqwest` - HTTP client with timeout support
- `clap` - Command line argument parsing
- `serde` & `toml` - Configuration and state persistence
- `chrono` - Date/time handling
- `anyhow` - Error handling
- `regex` - HTML tag stripping
- `open` - Browser integration

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🎉 Acknowledgments

- [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme) - For the beautiful color scheme
- [ratatui](https://github.com/ratatui/ratatui) - For the excellent TUI framework  
- [feed-rs](https://github.com/feed-rs/feed-rs) - For RSS/Atom parsing

## 🌙 Why "Yomi"?

**Yomi** (読み) means "reading" in Japanese, perfectly capturing what this app is all about. The name reflects both the core functionality and the Tokyo Night aesthetic that makes reading RSS feeds a beautiful experience.

---

**Built with ❤️ and 🦀 Rust**
