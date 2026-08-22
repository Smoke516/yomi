use crate::{App, CurrentScreen, FeedState, PopUp};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

// Enhanced Tokyo Night Theme with additional variants
pub struct EnhancedTokyoNight;

// The palette is kept complete rather than trimmed to what the UI happens to
// render today. Tokyo Night is a published colour scheme; a partial copy of it
// is worse than an unused constant, because the next person to add a widget
// reaches for a hex value instead of a name.
#[allow(dead_code)]
impl EnhancedTokyoNight {
    // Core colors (same as original)
    pub const BG: Color = Color::Rgb(26, 27, 38);         // #1a1b26
    pub const BG_DARK: Color = Color::Rgb(22, 23, 32);    // #16172020 - darker variant
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 44, 59); // #292c3b - selection
    pub const FG: Color = Color::Rgb(192, 202, 245);      // #c0caf5
    pub const FG_DARK: Color = Color::Rgb(169, 177, 214); // #a9b1d6 - dimmed text
    
    // Enhanced color palette
    pub const BLUE: Color = Color::Rgb(122, 162, 247);    // #7aa2f7 - primary
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);  // #bb9af7 - secondary
    pub const CYAN: Color = Color::Rgb(125, 207, 255);    // #7dcfff - accent
    pub const GREEN: Color = Color::Rgb(158, 206, 106);   // #9ece6a - success
    pub const RED: Color = Color::Rgb(247, 118, 142);     // #f7768e - error
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);  // #ff9e64 - warning
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);  // #e0af68 - info
    pub const GRAY: Color = Color::Rgb(86, 95, 137);      // #565f89 - muted
    pub const GRAY_LIGHT: Color = Color::Rgb(114, 124, 172); // #727ca8 - borders
    
    // Status styles
    pub fn focused_border() -> Style {
        Style::default().fg(Self::CYAN).add_modifier(Modifier::BOLD)
    }
    
    pub fn inactive_border() -> Style {
        Style::default().fg(Self::GRAY)
    }
    
    pub fn selected_item() -> Style {
        Style::default()
            .bg(Self::BG_HIGHLIGHT)
            .fg(Self::CYAN)
            .add_modifier(Modifier::BOLD)
    }
    
    pub fn success_text() -> Style {
        Style::default().fg(Self::GREEN)
    }
    
    pub fn error_text() -> Style {
        Style::default().fg(Self::RED)
    }
    
    pub fn warning_text() -> Style {
        Style::default().fg(Self::ORANGE)
    }
    
    pub fn info_text() -> Style {
        Style::default().fg(Self::YELLOW)
    }
    
    pub fn muted_text() -> Style {
        Style::default().fg(Self::GRAY)
    }
}

// Enhanced UI icons (using Unicode characters for better visual hierarchy)
pub struct Icons;

// Same reasoning as the palette: the icon set is a complete vocabulary, and a
// couple of its members are not on screen in the current layout.
#[allow(dead_code)]
impl Icons {
    pub const FEED_LOADED: &'static str = "📡";
    pub const FEED_LOADING: &'static str = "⏳";
    pub const FEED_ERROR: &'static str = "❌";
    pub const FEED_EMPTY: &'static str = "📭";
    
    pub const ARTICLE_UNREAD: &'static str = "●";
    pub const ARTICLE_READ: &'static str = "○";
    pub const ARTICLE_SELECTED: &'static str = "▶";
    
    pub const STATUS_OK: &'static str = "✓";
    pub const STATUS_ERROR: &'static str = "✗";
    pub const STATUS_REFRESH: &'static str = "↻";
    pub const STATUS_LOADING: &'static str = "⟳";
    
    pub const PANE_FEEDS: &'static str = "📰";
    pub const PANE_ARTICLES: &'static str = "📄";
    pub const PANE_PREVIEW: &'static str = "👀";
}

pub fn render_enhanced_ui(f: &mut Frame, app: &mut App) {
    let size = f.area();
    
    // Enhanced layout with better proportions and spacing
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Enhanced header
            Constraint::Min(5),     // Main content
            Constraint::Length(3),  // Enhanced status bar
        ])
        .split(size);

    // Render header with app info and quick help
    render_enhanced_header(f, main_chunks[0], app);

    match app.current_screen {
        CurrentScreen::FeedList | CurrentScreen::ArticleList => {
            render_enhanced_main_view(f, main_chunks[1], app);
        }
        CurrentScreen::ArticleView => {
            render_enhanced_article_view(f, main_chunks[1], app);
        }
    }
    
    // Enhanced status bar with contextual information
    render_enhanced_status_bar(f, main_chunks[2], app);

    // Modal popups
    if app.popup == PopUp::Help {
        let popup_area = centered_rect(70, 80, size);
        f.render_widget(Clear, popup_area);
        render_enhanced_help(f, popup_area);
    }
}

fn render_enhanced_header(f: &mut Frame, area: Rect, app: &App) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),        // Title and info
            Constraint::Length(30),    // Quick help
        ])
        .split(area);

    // App title with version and status
    let refresh_status = if app.is_refreshing {
        format!(" {} Refreshing...", Icons::STATUS_LOADING)
    } else {
        String::new()
    };
    
    let title_text = format!(
        " {} Yomi - {} feeds | {} unread{}",
        Icons::PANE_FEEDS,
        app.feeds.len(),
        app.unread_count,
        refresh_status
    );
    
    let title_style = if app.is_refreshing {
        EnhancedTokyoNight::info_text()
    } else {
        Style::default().fg(EnhancedTokyoNight::FG)
    };
    
    let title = Paragraph::new(title_text)
        .style(title_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(EnhancedTokyoNight::BLUE))
        );
    
    f.render_widget(title, header_chunks[0]);
    
    // Quick help hints
    let current_mode_help = match app.current_screen {
        CurrentScreen::FeedList => "Tab→Articles | r→Refresh | ?→Help",
        CurrentScreen::ArticleList => "↵→Read | o→Browser | Tab→Feeds",
        CurrentScreen::ArticleView => "↕→Scroll | Esc→Back | o→Browser",
    };
    
    let help_hint = Paragraph::new(current_mode_help)
        .style(EnhancedTokyoNight::muted_text())
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(EnhancedTokyoNight::GRAY))
        );
    
    f.render_widget(help_hint, header_chunks[1]);
}

fn render_enhanced_main_view(f: &mut Frame, area: Rect, app: &mut App) {
    // Improved three-pane layout with better proportions
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),  // Feeds - slightly larger
            Constraint::Percentage(38),  // Articles - main focus
            Constraint::Percentage(34),  // Preview - good size for reading
        ])
        .split(area);

    render_enhanced_feeds(f, chunks[0], app);
    render_enhanced_articles(f, chunks[1], app);
    render_enhanced_preview(f, chunks[2], app);
}

fn render_enhanced_feeds(f: &mut Frame, area: Rect, app: &mut App) {
    let feed_items: Vec<ListItem> = app.feeds
        .iter()
        .map(|feed| {
            let (icon, status_style) = match &feed.state {
                FeedState::Loading => (Icons::FEED_LOADING, EnhancedTokyoNight::info_text()),
                FeedState::Loaded(articles) => {
                    if articles.is_empty() {
                        (Icons::FEED_EMPTY, EnhancedTokyoNight::muted_text())
                    } else {
                        let unread_count = articles.iter().filter(|a| !a.read).count();
                        if unread_count > 0 {
                            (Icons::FEED_LOADED, EnhancedTokyoNight::success_text())
                        } else {
                            (Icons::FEED_LOADED, EnhancedTokyoNight::muted_text())
                        }
                    }
                }
                FeedState::Error(_) => (Icons::FEED_ERROR, EnhancedTokyoNight::error_text()),
            };
            
            // Show unread count for loaded feeds
            let unread_info = match &feed.state {
                FeedState::Loaded(articles) => {
                    let unread = articles.iter().filter(|a| !a.read).count();
                    if unread > 0 {
                        format!(" ({})", unread)
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            };
            
            let content = Line::from(vec![
                Span::styled(format!("{} ", icon), status_style),
                Span::styled(&feed.name, Style::default().fg(EnhancedTokyoNight::FG)),
                Span::styled(unread_info, EnhancedTokyoNight::info_text()),
            ]);
            
            ListItem::new(content)
        })
        .collect();
    
    let is_focused = app.current_screen == CurrentScreen::FeedList;
    let border_style = if is_focused {
        EnhancedTokyoNight::focused_border()
    } else {
        EnhancedTokyoNight::inactive_border()
    };
    
    let title = if app.is_refreshing {
        format!("{} Feeds {}", Icons::PANE_FEEDS, Icons::STATUS_REFRESH)
    } else {
        format!("{} Feeds", Icons::PANE_FEEDS)
    };
    
    let feeds_list = List::new(feed_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .title(title)
                .title_style(
                    if app.is_refreshing {
                        EnhancedTokyoNight::info_text().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(EnhancedTokyoNight::BLUE).add_modifier(Modifier::BOLD)
                    }
                )
        )
        .style(Style::default().fg(EnhancedTokyoNight::FG))
        .highlight_style(EnhancedTokyoNight::selected_item())
        .highlight_symbol("▶ ");

    // Use a temporary mutable reference to avoid borrowing conflicts
    let mut feed_state = app.feed_list_state.clone();
    f.render_stateful_widget(feeds_list, area, &mut feed_state);
    app.feed_list_state = feed_state;
}

fn render_enhanced_articles(f: &mut Frame, area: Rect, app: &mut App) {
    // Get current feed info first
    let current_feed = &app.feeds[app.current_feed_index];
    let title = match &current_feed.state {
        FeedState::Loading => format!("{} {} • Loading...", Icons::PANE_ARTICLES, current_feed.name),
        FeedState::Loaded(articles) => {
            let unread = articles.iter().filter(|a| !a.read).count();
            format!("{} {} • {} articles ({} new)", Icons::PANE_ARTICLES, current_feed.name, articles.len(), unread)
        },
        FeedState::Error(_e) => format!("{} {} • Error", Icons::PANE_ARTICLES, current_feed.name),
    };
    
    // Get articles and create list items
    let articles = app.current_articles();
    let article_items: Vec<ListItem> = articles
        .iter()
        .enumerate()
        .map(|(i, article)| {
            // Enhanced article display with better visual hierarchy
            let read_icon = if article.read {
                Icons::ARTICLE_READ
            } else {
                Icons::ARTICLE_UNREAD
            };
            
            let read_style = if article.read {
                EnhancedTokyoNight::muted_text()
            } else {
                EnhancedTokyoNight::success_text()
            };
            
            let date_str = article.pub_date
                .map(|date| format!(" • {}", date.format("%m/%d %H:%M")))
                .unwrap_or_default();
            
            let title_style = if article.read {
                Style::default().fg(EnhancedTokyoNight::FG_DARK)
            } else {
                Style::default().fg(EnhancedTokyoNight::FG)
            };
            
            // Add visual separator every 5 articles for better scanning
            let line_prefix = if i > 0 && i % 5 == 0 {
                "─ "
            } else {
                ""
            };
            
            let content = Line::from(vec![
                Span::raw(line_prefix),
                Span::styled(format!("{} ", read_icon), read_style),
                Span::styled(&article.title, title_style),
                Span::styled(date_str, EnhancedTokyoNight::muted_text()),
            ]);
            
            ListItem::new(content)
        })
        .collect();

    let is_focused = app.current_screen == CurrentScreen::ArticleList;
    let border_style = if is_focused {
        EnhancedTokyoNight::focused_border()
    } else {
        EnhancedTokyoNight::inactive_border()
    };

    let articles_list = List::new(article_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .title(title)
                .title_style(Style::default().fg(EnhancedTokyoNight::PURPLE).add_modifier(Modifier::BOLD))
        )
        .style(Style::default().fg(EnhancedTokyoNight::FG))
        .highlight_style(EnhancedTokyoNight::selected_item())
        .highlight_symbol("▶ ");

    // Use a temporary mutable reference to avoid borrowing conflicts
    let mut article_state = app.article_list_state.clone();
    f.render_stateful_widget(articles_list, area, &mut article_state);
    app.article_list_state = article_state;
}

fn render_enhanced_preview(f: &mut Frame, area: Rect, app: &App) {
    if let Some(article) = app.current_article() {
        // Enhanced preview with better formatting
        let title_line = format!("📰 {}", article.title);
        let link_line = format!("🔗 {}", article.link);
        let separator = "─".repeat(area.width.saturating_sub(4) as usize);
        
        let preview_text = format!(
            "{}\n{}\n{}\n\n{}",
            title_line,
            separator,
            link_line,
            strip_html_tags(&article.description)
        );

        let preview = Paragraph::new(preview_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(EnhancedTokyoNight::GREEN))
                    .title(format!("{} Preview", Icons::PANE_PREVIEW))
                    .title_style(Style::default().fg(EnhancedTokyoNight::GREEN).add_modifier(Modifier::BOLD))
            )
            .style(Style::default().fg(EnhancedTokyoNight::FG))
            .wrap(Wrap { trim: true });

        f.render_widget(preview, area);
    } else {
        let empty_preview = Paragraph::new("Select an article to preview")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(EnhancedTokyoNight::inactive_border())
                    .title(format!("{} Preview", Icons::PANE_PREVIEW))
                    .title_style(EnhancedTokyoNight::muted_text())
            )
            .style(EnhancedTokyoNight::muted_text())
            .alignment(Alignment::Center);

        f.render_widget(empty_preview, area);
    }
}

fn render_enhanced_article_view(f: &mut Frame, area: Rect, app: &App) {
    if let Some(article) = app.current_article() {
        // Enhanced article view with better formatting and reading experience
        let header = format!(
            "📰 {}\n🔗 {}\n📅 {}\n{}",
            article.title,
            article.link,
            article.pub_date.map(|d| d.format("%B %d, %Y at %H:%M").to_string()).unwrap_or("Unknown date".to_string()),
            "─".repeat(area.width.saturating_sub(4) as usize)
        );
        
        let content = format!("{}\n\n{}", header, strip_html_tags(&article.description));

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(EnhancedTokyoNight::focused_border())
                    .title(" 📖 Article Reader ")
                    .title_style(Style::default().fg(EnhancedTokyoNight::BLUE).add_modifier(Modifier::BOLD))
            )
            .style(Style::default().fg(EnhancedTokyoNight::FG))
            .wrap(Wrap { trim: true })
            .scroll((app.scroll_offset, 0));

        f.render_widget(paragraph, area);
    }
}

fn render_enhanced_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),        // Main status
            Constraint::Length(20),    // Last refresh time
        ])
        .split(area);

    // Main status with enhanced information
    let main_status = if !app.status_message.is_empty() {
        app.status_message.clone()
    } else {
        format!(
            "{} {} feeds • {} {} unread • Press ? for help",
            Icons::STATUS_OK,
            app.feeds.len(),
            Icons::ARTICLE_UNREAD,
            app.unread_count
        )
    };
    
    let status_style = if app.status_message.contains("Error") || app.status_message.contains("failed") {
        EnhancedTokyoNight::error_text()
    } else if app.is_refreshing {
        EnhancedTokyoNight::info_text()
    } else {
        Style::default().fg(EnhancedTokyoNight::FG)
    };

    let main_status_bar = Paragraph::new(main_status)
        .style(status_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(EnhancedTokyoNight::GRAY))
        );

    f.render_widget(main_status_bar, status_chunks[0]);

    // Last refresh info
    let elapsed = app.last_refresh.elapsed();
    let refresh_text = if elapsed.as_secs() < 60 {
        format!("⟲ {}s ago", elapsed.as_secs())
    } else if elapsed.as_secs() < 3600 {
        format!("⟲ {}m ago", elapsed.as_secs() / 60)
    } else {
        format!("⟲ {}h ago", elapsed.as_secs() / 3600)
    };

    let refresh_bar = Paragraph::new(refresh_text)
        .style(EnhancedTokyoNight::muted_text())
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(EnhancedTokyoNight::GRAY))
        );

    f.render_widget(refresh_bar, status_chunks[1]);
}

fn render_enhanced_help(f: &mut Frame, area: Rect) {
    let help_content = vec![
        Line::from(vec![
            Span::styled("🎌 Yomi RSS Reader - Help & CLI", 
                Style::default().fg(EnhancedTokyoNight::CYAN).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("💻 CLI COMMANDS", 
                Style::default().fg(EnhancedTokyoNight::CYAN).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  yomi add <url> [-n name]    Add RSS/Atom feed"),
        Line::from("  yomi list                   Show all feeds"),
        Line::from("  yomi remove <name|index>    Remove feed"),
        Line::from("  yomi refresh [-f <feed>]    Refresh feeds"),
        Line::from("  yomi read                   Start TUI mode"),
        Line::from("  yomi --help                 CLI help"),
        Line::from(""),
        Line::from(vec![
            Span::styled("⌨️  TUI NAVIGATION", 
                Style::default().fg(EnhancedTokyoNight::BLUE).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  ↑/↓,k/j   Navigate lists & scroll content"),
        Line::from("  ←/→,h/l   Move between panes"),
        Line::from("  Tab       Switch panes (feeds/articles/preview)"),
        Line::from("  Enter     Open article in full view"),
        Line::from("  Esc       Go back / Close dialogs"),
        Line::from("  g/G       Go to top/bottom"),
        Line::from("  q         Quit application"),
        Line::from(""),
        Line::from(vec![
            Span::styled("📰 ARTICLE ACTIONS", 
                Style::default().fg(EnhancedTokyoNight::GREEN).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  r, F5     Refresh all feeds"),
        Line::from("  o         Open in browser (mark as read)"),
        Line::from("  m         Toggle read/unread status"),
        Line::from("  A         Mark all feed articles as read"),
        Line::from(""),
        Line::from(vec![
            Span::styled("🎨 INTERFACE GUIDE", 
                Style::default().fg(EnhancedTokyoNight::PURPLE).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  [+] Feed loaded  [~] Loading  [!] Error"),
        Line::from("  [ ] Unread       [R] Read     [*] Refreshing"),
        Line::from("  Blue border = Active pane"),
        Line::from(""),
        Line::from(vec![
            Span::styled("⚙️  CONFIGURATION", 
                Style::default().fg(EnhancedTokyoNight::YELLOW).add_modifier(Modifier::BOLD))
        ]),
        Line::from("  Config: ~/.config/yomi/config.toml"),
        Line::from("  State:  ~/.config/yomi/state.toml"),
        Line::from("  Auto-refresh: Every 10 minutes"),
    ];

    let help_paragraph = Paragraph::new(help_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(EnhancedTokyoNight::focused_border())
                .title(" ❓ Help & Keyboard Shortcuts ")
                .title_style(Style::default().fg(EnhancedTokyoNight::BLUE).add_modifier(Modifier::BOLD))
        )
        .style(Style::default().fg(EnhancedTokyoNight::FG))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(help_paragraph, area);
}

// Utility function for HTML tag stripping (enhanced)
fn strip_html_tags(html: &str) -> String {
    // More sophisticated HTML cleanup
    let cleaned = html
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(&cleaned, "")
        .trim()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// Utility function for centered rectangles
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}