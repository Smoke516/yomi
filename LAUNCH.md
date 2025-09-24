# 🚀 Tokyo RSS Reader - Easy Launch Guide

The Tokyo RSS Reader is now installed and ready to use! Here are all the ways you can launch it:

## 🎯 **Quick Launch Methods**

### **1. Global Commands (Available Anywhere)**
```bash
tokyo-rss    # Full command name
rss          # Short alias  
trss         # Tokyo RSS alias
news         # News alias
```

### **2. Desktop Launcher**
- **GUI**: Search for "Tokyo RSS Reader" in your application launcher
- **Pop!_OS**: Press Super key and type "tokyo" or "rss"
- Will open in a new terminal window automatically

### **3. Direct Script**
```bash
./launch.sh  # From the project directory
```

### **4. Development Mode**
```bash
cargo run    # Run from source (from project directory)
```

---

## ✨ **Features Overview**

### **🎨 Interface**
- **3-Column Layout**: Feeds (25%) | Articles (35%) | Live Preview (40%)
- **Status Bar**: Feed count, unread count, last refresh, live status
- **Tokyo Night Theme**: Beautiful dark theme with purple/cyan accents

### **🔄 Smart Features**  
- **Read/Unread Persistence**: Your reading progress is saved automatically
- **Retry Logic**: Network errors handled with exponential backoff
- **Parallel Updates**: All feeds refresh simultaneously
- **Auto-refresh**: Feeds update every 10 minutes automatically

### **⌨️ Controls**
| Key | Action |
|-----|--------|
| `q` | Quit |
| `h` | Help popup |
| `r` | Refresh feeds |
| `o` | **Open in browser** (marks as read) |
| `m` | Mark as read |
| `Tab` | Switch between panes |
| `↑/↓` | Navigate lists/scroll |
| `Enter` | Full article view (marks as read) |
| `Esc` | Go back |

---

## 📁 **Configuration**

### **Config Location**
```
~/.config/tokyo-rss/config.toml
```

### **Add Your Feeds**
Edit the config file to add custom RSS feeds:
```toml
[feeds]
"My Blog" = "https://myblog.com/feed.xml"
"Tech News" = "https://technews.com/rss"
"Hacker News" = "https://feeds.feedburner.com/TheHackersNews"
```

### **State File**  
Reading progress saved to: `~/.config/tokyo-rss/state.toml`

---

## 🛠️ **Troubleshooting**

### **Command Not Found?**
```bash
# Reload your shell configuration
source ~/.zshrc

# Or manually add to PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
```

### **Desktop Entry Not Showing?**
```bash
# Update desktop database
update-desktop-database ~/.local/share/applications/
```

### **Reinstall if Needed**
```bash
cd /home/seawn/rust_rss_reader
cargo install --path . --force
```

---

## 🎉 **You're All Set!**

The Tokyo RSS Reader is now fully installed and accessible from anywhere on your system. Enjoy reading your feeds with style! 

**Happy Reading! 📰✨**
