<div align="center">

# 🎌 Yomi (読み)

### A Beautiful Terminal RSS Reader with Tokyo Night Theme

*"Yomi" (読み) means "reading" in Japanese - perfectly capturing the essence of this elegant feed reader.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()
[![Status](https://img.shields.io/badge/status-Active-brightgreen.svg)]()

![Yomi Demo](https://github.com/Smoke516/yomi/assets/placeholder/yomi-demo.gif)

*Experience RSS reading like never before with dual CLI/TUI interfaces and stunning Tokyo Night aesthetics.*

</div>

## ✨ Features

<table>
<tr>
<td width="50%">

### 🎯 **Core Features**
- 🎨 **Tokyo Night Theme** - Authentic color palette with enhanced contrast
- ⚡ **Lightning Fast** - Async Rust architecture with minimal memory footprint
- 🖥️ **Dual Interface** - Both CLI commands and interactive TUI
- 📰 **Universal Feed Support** - RSS, Atom, and feed auto-detection
- 🔄 **Smart Refresh** - Background updates with visual progress indicators

</td>
<td width="50%">

### 🚀 **Advanced Features**
- ⌨️ **Vim-Style Navigation** - hjkl keys plus arrow key support
- 📖 **Read Tracking** - Persistent state across sessions
- 🌐 **Browser Integration** - One-key article opening
- 🔍 **Feed Validation** - URL verification and error handling
- 🎨 **Rich UI Components** - Modern TUI with visual feedback

</td>
</tr>
</table>

## 🚀 Installation

### Method 1: Install from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/Smoke516/yomi.git
cd yomi

# Install globally
cargo install --path .

# Verify installation
yomi --version
```

### Method 2: Development Build

```bash
# For development and testing
git clone https://github.com/Smoke516/yomi.git
cd yomi
cargo build --release

# Run directly
./target/release/yomi
```

### Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Terminal with TrueColor support** - For optimal Tokyo Night theme display
- **Modern shell** - bash, zsh, fish, or PowerShell

## 🎯 Quick Start Guide

### 🚀 **Get Started in 30 Seconds**

```bash
# 1. Start the beautiful TUI interface
yomi

# 2. Add your first feed
yomi add https://feeds.feedburner.com/oreilly --name "O'Reilly"

# 3. List all feeds
yomi list

# 4. Refresh and see latest articles
yomi refresh
```

### 🎨 **Create an Alias for Quick Access**

```bash
# Add to your shell config (~/.bashrc, ~/.zshrc, etc.)
alias rss='yomi'
alias news='yomi refresh && yomi'

# Then use:
rss                    # Start reading
news                   # Quick update and read
```

## 💻 Command Line Interface

### 📋 **Feed Management Commands**

<table>
<tr><th>Command</th><th>Description</th><th>Example</th></tr>
<tr><td><code>yomi add &lt;url&gt;</code></td><td>Add feed (auto-detect name)</td><td><code>yomi add https://feeds.example.com/rss</code></td></tr>
<tr><td><code>yomi add &lt;url&gt; -n &lt;name&gt;</code></td><td>Add feed with custom name</td><td><code>yomi add https://techcrunch.com/feed/ -n "Tech"</code></td></tr>
<tr><td><code>yomi list</code></td><td>Show all configured feeds</td><td><code>yomi list</code></td></tr>
<tr><td><code>yomi remove &lt;target&gt;</code></td><td>Remove feed by name or index</td><td><code>yomi remove "TechCrunch"</code></td></tr>
<tr><td><code>yomi refresh</code></td><td>Update all feeds</td><td><code>yomi refresh</code></td></tr>
<tr><td><code>yomi refresh -f &lt;feed&gt;</code></td><td>Update specific feed</td><td><code>yomi refresh -f "Hacker News"</code></td></tr>
<tr><td><code>yomi</code> or <code>yomi read</code></td><td>Start interactive TUI</td><td><code>yomi</code></td></tr>
</table>

### 🚑 **Real-World Usage Examples**

```bash
# Technology feeds
yomi add https://feeds.feedburner.com/oreilly --name "O'Reilly Radar"
yomi add https://www.theverge.com/rss/index.xml -n "The Verge"
yomi add https://techcrunch.com/feed/ -n "TechCrunch"

# News and blogs
yomi add https://feeds.feedburner.com/TheHackersNews -n "Hacker News"
yomi add https://www.reddit.com/r/programming/.rss -n "r/programming"

# Check what you have
yomi list

# Update everything and start reading
yomi refresh && yomi
```

### 🆘 **Pro Tips**

- **Batch management**: Use shell scripts to add multiple feeds
- **Feed discovery**: Many sites have `/feed`, `/rss`, or `/atom.xml` endpoints
- **Validation**: Yomi automatically validates feeds when adding them
- **Error handling**: Detailed error messages help troubleshoot feed issues

## ⌨️ Terminal User Interface (TUI)

### 🎮 **Keyboard Controls**

<table>
<tr><th width="25%">Category</th><th width="25%">Keys</th><th width="50%">Action</th></tr>

<tr><td rowspan="3"><strong>🧭 Navigation</strong></td>
<td><code>Tab</code></td><td>Switch between panes (Feeds → Articles → Preview)</td></tr>
<tr><td><code>↑↓</code> <code>k j</code></td><td>Navigate lists and scroll content</td></tr>
<tr><td><code>←→</code> <code>h l</code></td><td>Move between panes (left/right)</td></tr>

<tr><td rowspan="4"><strong>📰 Reading</strong></td>
<td><code>Enter</code></td><td>Open article in full reading view</td></tr>
<tr><td><code>o</code></td><td>Open article in browser (auto-mark as read)</td></tr>
<tr><td><code>m</code></td><td>Toggle article read/unread status</td></tr>
<tr><td><code>A</code></td><td>Mark all articles in current feed as read</td></tr>

<tr><td rowspan="4"><strong>🚀 Quick Actions</strong></td>
<td><code>r</code> <code>F5</code></td><td>Refresh all feeds</td></tr>
<tr><td><code>g</code> <code>G</code></td><td>Jump to top / bottom</td></tr>
<tr><td><code>PgUp</code> <code>PgDn</code></td><td>Page up / down (bulk navigation)</td></tr>
<tr><td><code>Esc</code></td><td>Go back / close dialogs</td></tr>

<tr><td rowspan="2"><strong>🆘 System</strong></td>
<td><code>?</code></td><td>Show comprehensive help screen</td></tr>
<tr><td><code>q</code></td><td>Quit application</td></tr>
</table>

### 🎆 **Interface Layout**

```
┌───────────────┬───────────────────────────────────┬───────────────────────┐
│ 📡 RSS Feeds    │ 📄 TechCrunch • 42 articles (12 new) │ 👁️ Preview Pane      │
│                │                                    │                      │
│ ▶ [+] Tech (12)   │ ▶ [ ] Breaking: New AI Model...   │ 📰 Article Title     │
│   [+] News (8)    │   [R] Yesterday's Article      │ ────────────────── │
│   [~] Loading...  │   [ ] Another New Story        │ 🔗 Article URL       │
│   [!] Error       │   [R] Read Article             │                      │
│                │   ...                          │ Article description  │
│                │                                    │ and content preview  │
│                │                                    │ shows here...        │
└───────────────┴───────────────────────────────────┴───────────────────────┘
│ 📡 5 feeds • 🆕 20 unread • ⚙️ Last refresh: 2min ago • Press ? for help │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 🟢 **Status Indicators**
- **`[+]`** Feed loaded successfully  
- **`[~]`** Feed currently loading/refreshing
- **`[!]`** Feed error (network, parsing, etc.)
- **`[ ]`** Unread article
- **`[R]`** Read article  
- **`[*]`** Background refresh in progress

## ⚙️ Configuration

### 📁 **File Locations**

Yomi stores its configuration in standard system directories:

| Platform | Location |
|----------|----------|
| **Linux** | `~/.config/yomi/` |
| **macOS** | `~/.config/yomi/` |
| **Windows** | `%APPDATA%/yomi/` |

### 📄 **Configuration Files**

| File | Purpose | Auto-created |
|------|---------|-------------|
| `config.toml` | Feed URLs and names | ✓ |
| `state.toml` | Read/unread article tracking | ✓ |

### 📝 **config.toml Structure**

```toml
# Yomi RSS Reader Configuration
# This file is automatically managed by the CLI commands

[feeds]
# Format: "Display Name" = "RSS/Atom URL"
"Hacker News" = "https://feeds.feedburner.com/TheHackersNews"
"TechCrunch" = "https://techcrunch.com/feed/"
"The Verge" = "https://www.theverge.com/rss/index.xml"
"Ars Technica" = "https://feeds.arstechnica.com/arstechnica/index"
"GitHub Blog" = "https://github.blog/feed/"
"Rust Blog" = "https://blog.rust-lang.org/feed.xml"
```

### 🎆 **Default Feed Collection**

Yomi comes pre-configured with curated feeds:

| Feed | Category | Description |
|------|----------|-------------|
| **Hacker News** | Security/Tech | Cybersecurity and hacking news |
| **BleepingComputer** | Security | Computer security and malware |
| **TechCrunch** | Startup/Tech | Technology startup news |

### 🔧 **Advanced Configuration**

```bash
# View configuration files
ls -la ~/.config/yomi/

# Edit config manually (not recommended)
nano ~/.config/yomi/config.toml

# Reset to defaults (removes all feeds!)
rm -rf ~/.config/yomi/
# Yomi will recreate with defaults on next run
```

## 🎨 Tokyo Night Aesthetic

### 🌌 **Color Palette**

Yomi uses an enhanced Tokyo Night theme with carefully selected colors for optimal readability:

<table>
<tr>
<td>

**Core Colors**
- 🌑 **Background**: `#1a1b26` 
- ✨ **Foreground**: `#c0caf5`
- 🔵 **Blue**: `#7aa2f7`
- 🟪 **Purple**: `#bb9af7`
- 🔷 **Cyan**: `#7dcfff`

</td>
<td>

**Accent Colors**
- 🟨 **Green**: `#9ece6a`
- 🟡 **Yellow**: `#e0af68`
- 🟠 **Orange**: `#ff9e64`
- 🔴 **Red**: `#f7768e`
- ⬛ **Gray**: `#565f89`

</td>
</tr>
</table>

### 🎨 **Design Philosophy**

- **Eye Comfort**: Carefully calibrated contrast ratios reduce eye strain
- **Visual Hierarchy**: Colors convey meaning (green=unread, gray=read, blue=focus)
- **Accessibility**: Tested for colorblind-friendly visibility
- **Consistency**: Matches the beloved Tokyo Night theme across editors and tools

## 🚀 Performance & Reliability

### ⚡ **Speed Optimizations**
- **Async Architecture**: Non-blocking feed updates using Tokio
- **Parallel Processing**: Multiple feeds refresh simultaneously
- **Memory Efficient**: Minimal RSS parsing with smart caching
- **Instant Startup**: TUI renders immediately while feeds load in background

### 🔒 **Reliability Features**
- **Auto-Retry**: Failed feed requests retry with exponential backoff
- **Timeout Handling**: 30-second request / 45-second operation timeouts
- **Error Recovery**: Graceful degradation when feeds are unavailable
- **State Persistence**: Read status survives app restarts and crashes

### 📊 **Smart Refresh System**
- **Background Updates**: Feeds refresh every 10 minutes automatically
- **Visual Indicators**: Real-time status shows refresh progress
- **Selective Updates**: Refresh individual feeds or all at once
- **Network Awareness**: Handles offline scenarios gracefully

## 🔧 Development

### 🚀 **Building from Source**

```bash
# Development build (faster compilation, debug info)
cargo build

# Optimized release build (slower compilation, optimized binary)
cargo build --release

# Run with live code reloading during development
cargo run

# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

### 📦 **Project Architecture**

```
yomi/
├── src/
│   ├── main.rs           # Application entry point & TUI logic
│   ├── cli.rs            # Command-line interface implementation
│   └── enhanced_ui.rs    # Modern TUI components & Tokyo Night theme
├── Cargo.toml            # Dependencies and project metadata
├── README.md             # This comprehensive guide
└── .gitignore            # Git exclusion rules
```

### 📦 **Key Dependencies**

<table>
<tr><th>Category</th><th>Crate</th><th>Purpose</th><th>Version</th></tr>

<tr><td rowspan="2"><strong>UI</strong></td>
<td><code>ratatui</code></td><td>Modern terminal UI framework</td><td>0.29</td></tr>
<tr><td><code>crossterm</code></td><td>Cross-platform terminal control</td><td>0.29</td></tr>

<tr><td rowspan="3"><strong>Async</strong></td>
<td><code>tokio</code></td><td>Async runtime with full features</td><td>1.0</td></tr>
<tr><td><code>reqwest</code></td><td>HTTP client with rustls-tls</td><td>0.12</td></tr>
<tr><td><code>futures</code></td><td>Future utilities and combinators</td><td>0.3</td></tr>

<tr><td rowspan="2"><strong>Parsing</strong></td>
<td><code>feed-rs</code></td><td>RSS/Atom feed parser</td><td>2.0</td></tr>
<tr><td><code>regex</code></td><td>HTML tag removal</td><td>1.0</td></tr>

<tr><td rowspan="3"><strong>CLI</strong></td>
<td><code>clap</code></td><td>Command-line argument parsing</td><td>4.0</td></tr>
<tr><td><code>serde</code></td><td>Serialization framework</td><td>1.0</td></tr>
<tr><td><code>toml</code></td><td>Configuration file format</td><td>0.8</td></tr>

<tr><td rowspan="3"><strong>Utils</strong></td>
<td><code>anyhow</code></td><td>Error handling and context</td><td>1.0</td></tr>
<tr><td><code>chrono</code></td><td>Date and time handling</td><td>0.4</td></tr>
<tr><td><code>open</code></td><td>Cross-platform file/URL opening</td><td>5.0</td></tr>
</table>

## 🎓 Workflow Examples

### 📰 **Daily News Routine**

```bash
# Morning: Quick update
yomi refresh

# Browse headlines in beautiful TUI
yomi

# Add new feeds as you discover them
yomi add https://blog.rust-lang.org/feed.xml -n "Rust Blog"

# Evening: Mark everything as read
# (Use 'A' key in TUI for current feed, or refresh and browse)
```

### 🛠️ **Feed Curation**

```bash
# Start with tech feeds
yomi add https://feeds.feedburner.com/oreilly -n "O'Reilly"
yomi add https://www.theverge.com/rss/index.xml -n "The Verge"
yomi add https://techcrunch.com/feed/ -n "TechCrunch"

# Add development blogs
yomi add https://github.blog/feed/ -n "GitHub Blog"
yomi add https://blog.rust-lang.org/feed.xml -n "Rust Official"

# News sources
yomi add https://feeds.feedburner.com/TheHackersNews -n "Hacker News"

# Review your collection
yomi list

# Remove feeds you don't read
yomi remove "Old Feed Name"
```

### 🧩 **Integration with Other Tools**

```bash
# Create shell aliases
echo 'alias news="yomi refresh && yomi"' >> ~/.bashrc
echo 'alias feeds="yomi list"' >> ~/.bashrc

# Use in scripts
#!/bin/bash
# Update feeds and show count
yomi refresh > /dev/null 2>&1
echo "Updated $(yomi list | wc -l) feeds"

# Combine with other tools
yomi refresh && notify-send "RSS feeds updated"
```

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### 🐛 **Report Issues**
- **Bug Reports**: Use the issue template with reproduction steps
- **Feature Requests**: Describe the use case and proposed solution
- **Questions**: Check existing issues first, then ask!

### 🛠️ **Development Process**

```bash
# 1. Fork and clone
git clone https://github.com/YOUR-USERNAME/yomi.git
cd yomi

# 2. Create a feature branch
git checkout -b feature/amazing-feature

# 3. Make your changes
# - Follow Rust conventions
# - Add tests for new features
# - Update documentation

# 4. Test your changes
cargo test
cargo clippy
cargo fmt

# 5. Submit PR
git commit -am "Add amazing feature"
git push origin feature/amazing-feature
# Then create PR on GitHub
```

### 🎯 **Contribution Areas**
- **UI/UX improvements**: Better layouts, themes, interactions
- **Feed parsing**: Support for more feed formats and edge cases
- **Performance**: Optimization of refresh logic and memory usage
- **Platform support**: Windows-specific enhancements
- **Documentation**: Examples, tutorials, and guides

## 🏆 Comparison with Other RSS Readers

| Feature | Yomi | Newsboat | Elfeed | Web Readers |
|---------|------|----------|--------|-------------|
| **Interface** | Modern TUI + CLI | Classic TUI | Emacs-based | Web UI |
| **Performance** | ⚡ Async Rust | ✅ Fast C++ | ✅ Fast Elisp | 🐢 Varies |
| **Offline** | ✅ Full support | ✅ Full support | ✅ Full support | ❌ Online only |
| **Customization** | 🎨 Tokyo Night | 🔧 Extensive | 🔧 Infinite | ⚠️ Limited |
| **Setup** | 🚀 Zero config | 🛠️ Complex | 🧠 Requires Emacs | 🌐 Account signup |
| **CLI Management** | ✅ Built-in | ❌ Manual editing | ❌ Manual editing | ❌ Web interface |
| **Cross-platform** | ✅ All platforms | ✅ Unix/Linux | ✅ Where Emacs runs | ✅ All platforms |

## 🔍 Troubleshooting

### Common Issues

<details>
<summary><strong>🔄 Feed won't refresh / "Network error"</strong></summary>

**Symptoms**: Feed shows `[!] Error` or timeout messages

**Solutions**:
```bash
# Test the feed URL manually
curl -I "https://your-feed-url.com/rss"

# Check if URL redirects (many feeds do)
curl -L "https://your-feed-url.com/rss"

# Try re-adding with the final redirect URL
yomi remove "Problematic Feed"
yomi add "https://final-redirect-url.com/feed" -n "Fixed Feed"
```
</details>

<details>
<summary><strong>🎨 Colors look wrong / No color support</strong></summary>

**Symptoms**: Plain text appearance or wrong colors

**Solutions**:
```bash
# Check terminal color support
echo $COLORTERM
# Should show "truecolor" or "24bit"

# For tmux users, add to ~/.tmux.conf:
set -g default-terminal "screen-256color"
set -ga terminal-overrides ",*256col*:Tc"

# Test color support
printf "[38;2;255;100;0mTRUECOLOR[0m
"
```
</details>

<details>
<summary><strong>⌨️ Keys not working / Strange behavior</strong></summary>

**Symptoms**: Arrow keys, hjkl, or other keys don't work

**Solutions**:
```bash
# Check terminal type
echo $TERM
# Should be xterm-256color or similar modern terminal

# For SSH sessions, ensure terminal forwarding:
ssh -t user@host

# Try resetting terminal
reset
```
</details>

### 📞 Getting Help

- **Built-in help**: Press `?` in the TUI or run `yomi --help`
- **GitHub Issues**: [Report bugs or request features](https://github.com/Smoke516/yomi/issues)
- **Discussions**: [Community Q&A and tips](https://github.com/Smoke516/yomi/discussions)

## 📝 License

**MIT License** - See [LICENSE](LICENSE) file for full details.

This means you can use, modify, and distribute Yomi freely, including for commercial purposes.

## 🎉 Acknowledgments & Inspiration

### 🚀 **Core Technologies**
- **[ratatui](https://github.com/ratatui/ratatui)** - Outstanding TUI framework that makes beautiful terminals possible
- **[Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme)** - The gorgeous color scheme that inspired our aesthetic
- **[feed-rs](https://github.com/feed-rs/feed-rs)** - Robust RSS/Atom parsing library

### 🎆 **Design Inspiration**
- **Newsboat** - Pioneered terminal RSS reading with excellent UX patterns
- **Vim/Neovim** - Keyboard-driven interface philosophy
- **Tokyo Night theme ecosystem** - Consistent aesthetic across tools

### 👥 **Community**
Special thanks to early testers, contributors, and the Rust community for feedback and support!

---

<div align="center">

## 🌙 Why "Yomi"?

**Yomi** (読み) means "reading" in Japanese, perfectly capturing this app's essence.

The name reflects both the core functionality and the Tokyo Night aesthetic that makes RSS reading a beautiful, meditative experience.

Like the Japanese concept of *mono no aware* (the beauty of transient things), Yomi helps you mindfully consume the ever-flowing stream of information.

---

### Built with ❤️ and 🦀 Rust

**⭐ Star this project if you find it useful! ⭐**

[![GitHub stars](https://img.shields.io/github/stars/Smoke516/yomi?style=social)](https://github.com/Smoke516/yomi/stargazers)

</div>
