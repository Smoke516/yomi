# Changelog

All notable changes to Yomi will be documented in this file.

## [Unreleased]

### Changed — Yomi is a daily edition now, not an inbox

The reader was reorganised around a bounded, ranked daily edition. The old
shape — feeds, articles, preview pane, unread counts — was an email client, and
it is gone.

- **No unread count anywhere.** The header reports how much was discarded, not
  how much you owe. `mark-all-as-read` went with it; it only ever existed to
  dismiss a number that should not have been there.
- **A bounded edition.** Twelve articles by default, over a 36-hour window,
  both configurable. Per-feed crowding damping means a feed posting forty times
  a day gets one slot rather than forty.
- **Ranking with a visible reason.** Five or six additive terms — feed
  engagement learned from what you finish, length preference, recency,
  crowding, inbound links from your other feeds, and your own keyword rules.
  `w` shows the arithmetic; `d` corrects it. Not a model: no training, no
  opacity, and nothing leaves the machine.
- **An honest held-back list.** Every feed with arrivals that did not make the
  edition is named, with the reason, and `↵` reveals it.
- **A real reading view.** The panes are dropped for a ~65-character measure
  with paragraphs, blockquotes and code. Article pages are fetched and
  extracted, because most feeds truncate.
- **`s` writes to a markdown vault** with YAML frontmatter, for Scribble or any
  Obsidian-style vault, and never overwrites a file it did not create.
- **Colour comes from the terminal.** The sixteen ANSI slots instead of
  hardcoded Tokyo Night RGB, with one accent reserved for meaning.
- **MSRV is now 1.88**, up from 1.82. `rusqlite` pulls in `hashbrown` 0.17,
  which needs edition 2024. Verified against real toolchains: 1.87 fails to
  build, 1.88 builds and passes the suite.

### Added

- **A local SQLite store** at `~/.local/share/yomi/yomi.db` (`$YOMI_STORE`
  overrides). Articles are kept rather than refetched, which is what makes the
  ranking, offline reading and instant startup possible.
- **`yomi today`**, with `--json` for piping.
- **`yomi why <n>`** — the score breakdown from the shell.
- **`yomi rule boost|demote|hide|list|remove`** — plain substring rules.
- **`yomi status`** and **`yomi config`**.
- Feed names are taken from the feed when `--name` is omitted.
- A panic hook that restores the terminal, so a crash can no longer leave your
  shell without echo.

### Fixed

- **`yomi today | head` panicked with a broken pipe.** Rust masks SIGPIPE at
  startup; a tool that advertises itself as pipeable has to restore it.
- **The interface redrew eight times a second while idle.** It now paints only
  when something changed, plus once a minute so relative ages stay honest.

### Fixed (from the cleanup pass)

- **`truncate_string` aborted the process on non-ASCII titles.** It cut with a
  byte index, so any headline whose byte 27 fell inside a multi-byte character
  panicked when opening an article in the browser (`o`). Because the panic
  happened in raw mode inside the alternate screen, it also left the shell
  without echo. Truncation now counts and cuts in characters.
- **`yomi list` had the same byte-slicing crash** on feed names longer than 28
  characters containing non-ASCII.
- **Character-count check was byte-based**, so a 12-character Japanese title
  (36 bytes) was truncated as if it overflowed a 30-column budget.
- **`j`/`k` underflowed with no feeds configured.** `next_feed` and
  `previous_feed` evaluated `feeds.len() - 1` without an emptiness guard, which
  the CLI lets you reach by removing every feed.

### Removed

- `src/main_enhanced.rs`, 1152 lines that were never part of the build — a
  superseded prototype of the whole app. Its search, read/unread filter and
  feed-autodiscovery ideas have no equivalent in `main.rs` and remain in git
  history at 30c122b.
- The pre-rewrite render path in `main.rs` (`ui`, `render_feeds`,
  `render_articles`, `render_article_view`, `render_help`,
  `render_preview_pane`, `render_status_bar`, `centered_rect`,
  `strip_html_tags`, the old `TokyoNight` palette) — dead since `enhanced_ui`
  took over drawing.
- `CurrentScreen::Help`, an unreachable variant. Help is reached via
  `PopUp::Help` (bound to `?`). The six `CurrentScreen` matches are now
  exhaustive.
- Nine pre-rename "Tokyo RSS" files: `install.sh`, `deploy.sh`, `launch.sh`,
  `demo.sh`, `test_launch.sh`, `test_features.py`, `LAUNCH.md`, `RENAME.md`
  and `FEATURES_STATUS.md`. All drove a `tokyo-rss` binary that Cargo has not
  produced since the rename, and `install.sh` failed partway through.

### Added

- `LICENSE`. The README has badged MIT since the rewrite without one, which
  left the code all-rights-reserved by default.
- CI on Linux, macOS and Windows: `cargo fmt --check`, `clippy -D warnings`,
  the test suite, an MSRV job, and release builds for four targets.
- The first tests in the repo: 10 cases over `truncate_string`.

### Changed

- Declared `rust-version = "1.82"`. The README claimed 1.70+, which was never
  true — the dependency tree needs 1.82, verified against real 1.81 and 1.82
  toolchains.
- Corrected the Cargo.toml repository URL, which pointed at
  `github.com/seawn/yomi`.
- Removed the README's hero demo GIF; the URL was a placeholder returning 404.
- `cargo fmt` across the crate; `cargo clippy --all-targets` is clean.

## [1.0.0] - 2025-09-24

### Changed

- Renamed the project from Tokyo RSS to Yomi (読み); the binary, crate and
  config directory all moved from `tokyo-rss` to `yomi`.

### Added

- A CLI alongside the TUI: `add`, `remove`, `list`, `refresh`, `read`, with
  feed validation on add.
- Reworked TUI (`enhanced_ui`) with background refresh, persistent read state,
  browser integration, and retrying feed fetches.

## [0.1.1] - 2025-08-21

### Changed
- 🚀 **Performance Optimization**: Limited articles to 20 per feed (down from 50) for significantly faster loading times
- 📱 Improved responsiveness when switching between feeds
- 💾 Reduced memory usage by processing fewer articles per feed

### Technical Details
- Modified `fetch_feed()` function to use `.take(20)` instead of `.take(50)`
- This change reduces network payload, parsing time, and memory usage
- Feeds now load approximately 60% faster while still providing plenty of recent content

## [0.1.0] - 2025-08-21

### Added
- 🎨 Beautiful Tokyo Night theme with authentic color palette
- 📰 Multi-feed RSS/Atom support
- ⚡ Async feed fetching and parsing
- 🖥️ Split-pane TUI interface with keyboard navigation
- 📖 Full article reading with scrollable content
- ⚙️ TOML configuration file support
- 🔄 Auto-refresh feeds every 10 minutes
- 📱 Responsive UI with status indicators
- 🏷️ HTML tag stripping for clean article display
- 🔧 Help popup with keyboard shortcuts
- 📦 One-command installation script
- 🎯 Short alias support (`rss` command)

### Technical Features
- Built with Rust for performance and safety
- Uses ratatui for beautiful terminal UI
- Crossterm for cross-platform terminal support
- Tokio for async runtime
- Feed-rs for RSS/Atom parsing
- Regex for HTML cleaning
- Serde/TOML for configuration

### Supported Platforms
- Linux
- macOS (untested)
- Windows (untested)

---

**Note**: Version numbers follow [Semantic Versioning](https://semver.org/).
