use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::future;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::ListState,
    Terminal,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io,
    time::{Duration, Instant},
};
use tokio::time::{sleep, timeout};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// CLI module
mod cli;
use cli::{Cli, Commands};
use clap::Parser;

// Enhanced UI module
mod enhanced_ui;
use enhanced_ui::render_enhanced_ui;

mod util;
use util::truncate_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    feeds: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut feeds = HashMap::new();
        feeds.insert("Hacker News".to_string(), "https://feeds.feedburner.com/TheHackersNews".to_string());
        feeds.insert("BleepingComputer".to_string(), "https://www.bleepingcomputer.com/feed/".to_string());
        feeds.insert("TechCrunch".to_string(), "https://techcrunch.com/feed/".to_string());
        Self { feeds }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Article {
    id: String,
    title: String,
    link: String,
    description: String,
    pub_date: Option<DateTime<Local>>,
    read: bool,
}

impl Article {
    fn new(title: String, link: String, description: String, pub_date: Option<DateTime<Local>>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(link.as_bytes());
        let id = hex::encode(hasher.finalize())[..16].to_string();
        
        Self {
            id,
            title,
            link,
            description,
            pub_date,
            read: false,
        }
    }
}

#[derive(Debug, Clone)]
enum FeedState {
    Loading,
    Loaded(Vec<Article>),
    Error(String),
}

#[derive(Debug, Clone)]
struct Feed {
    name: String,
    url: String,
    state: FeedState,
    last_updated: Option<Instant>,
    retry_count: u32,
}

// Background task communication types
#[derive(Debug)]
enum BackgroundMessage {
    RefreshComplete(Vec<(usize, String, Result<Vec<Article>, String>)>),
    RefreshStarted,
}

#[derive(Debug)]
enum BackgroundCommand {
    StartRefresh(Vec<(usize, String, String)>), // (index, name, url)
}

#[derive(Debug, PartialEq)]
enum CurrentScreen {
    FeedList,
    ArticleList,
    ArticleView,
}

#[derive(Debug, PartialEq)]
enum PopUp {
    None,
    Help,
}

// App state persistence
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppState {
    read_articles: HashSet<String>,
    last_update: Option<DateTime<Utc>>,
}

struct App {
    current_screen: CurrentScreen,
    popup: PopUp,
    feeds: Vec<Feed>,
    feed_list_state: ListState,
    article_list_state: ListState,
    current_feed_index: usize,
    current_article_index: usize,
    should_quit: bool,
    last_refresh: Instant,
    app_state: AppState,
    scroll_offset: u16,
    // Enhanced UI state
    unread_count: usize,
    status_message: String,
    // Background refresh state
    is_refreshing: bool,
    // Background task communication
    bg_message_rx: Option<mpsc::UnboundedReceiver<BackgroundMessage>>,
    bg_command_tx: Option<mpsc::UnboundedSender<BackgroundCommand>>,
    bg_task_handle: Option<JoinHandle<()>>,
}

impl App {
    async fn new() -> Result<Self> {
        let config = load_config()?;
        let app_state = load_app_state().await?;
        
        let feeds: Vec<Feed> = config.feeds.iter().map(|(name, url)| Feed {
            name: name.clone(),
            url: url.clone(),
            state: FeedState::Loading,
            last_updated: None,
            retry_count: 0,
        }).collect();

        let mut feed_list_state = ListState::default();
        feed_list_state.select(Some(0));

        // Setup background task communication channels
        let (bg_message_tx, bg_message_rx) = mpsc::unbounded_channel::<BackgroundMessage>();
        let (bg_command_tx, bg_command_rx) = mpsc::unbounded_channel::<BackgroundCommand>();
        
        // Spawn background task handler
        let bg_task_handle = tokio::spawn(background_task_handler(bg_command_rx, bg_message_tx));

        Ok(Self {
            current_screen: CurrentScreen::FeedList,
            popup: PopUp::None,
            feeds,
            feed_list_state,
            article_list_state: ListState::default(),
            current_feed_index: 0,
            current_article_index: 0,
            should_quit: false,
            last_refresh: Instant::now(),
            app_state,
            scroll_offset: 0,
            unread_count: 0,
            status_message: "Loading...".to_string(),
            is_refreshing: false,
            bg_message_rx: Some(bg_message_rx),
            bg_command_tx: Some(bg_command_tx),
            bg_task_handle: Some(bg_task_handle),
        })
    }

    // Start background refresh - non-blocking
    async fn start_background_refresh(&mut self) {
        if self.is_refreshing {
            self.status_message = "[*] Refresh already in progress...".to_string();
            return;
        }
        
        self.is_refreshing = true;
        self.status_message = "[*] Starting background refresh...".to_string();
        
        // Prepare feed data for background task
        let feed_data: Vec<(usize, String, String)> = self.feeds
            .iter()
            .enumerate()
            .map(|(i, feed)| (i, feed.name.clone(), feed.url.clone()))
            .collect();
        
        // Send command to background task
        if let Some(ref tx) = self.bg_command_tx {
            let _ = tx.send(BackgroundCommand::StartRefresh(feed_data));
        }
    }
    
    // Process background messages - called from main event loop
    fn handle_background_messages(&mut self) {
        let mut messages = Vec::new();
        
        // Collect all pending messages first to avoid borrowing conflicts
        if let Some(ref mut rx) = self.bg_message_rx {
            while let Ok(message) = rx.try_recv() {
                messages.push(message);
            }
        }
        
        // Process collected messages
        for message in messages {
            match message {
                BackgroundMessage::RefreshStarted => {
                    self.is_refreshing = true;
                    self.status_message = "[*] Refreshing feeds in background...".to_string();
                }
                BackgroundMessage::RefreshComplete(results) => {
                    self.is_refreshing = false;
                    self.process_refresh_results(results);
                }
            }
        }
    }
    
    fn process_refresh_results(&mut self, results: Vec<(usize, String, Result<Vec<Article>, String>)>) {
        for (index, _name, result) in results {
            match result {
                Ok(mut articles) => {
                    // Apply read state from persistent storage
                    for article in &mut articles {
                        article.read = self.app_state.read_articles.contains(&article.id);
                    }
                    self.feeds[index].state = FeedState::Loaded(articles);
                    self.feeds[index].retry_count = 0;
                    self.feeds[index].last_updated = Some(Instant::now());
                },
                Err(e) => {
                    self.feeds[index].retry_count += 1;
                    let error_msg = if e.to_string().contains("timeout") {
                        "Connection timeout"
                    } else if e.to_string().contains("HTTP") {
                        "Server error"
                    } else {
                        "Network error"
                    };
                    
                    self.feeds[index].state = FeedState::Error(format!(
                        "{} (attempt {})", 
                        error_msg, 
                        self.feeds[index].retry_count
                    ));
                }
            }
        }
        
        self.last_refresh = Instant::now();
        self.calculate_unread_count();
        
        let successful = self.feeds.iter().filter(|f| matches!(f.state, FeedState::Loaded(_))).count();
        let failed = self.feeds.len() - successful;
        
        self.status_message = if failed == 0 {
            format!("[OK] All {} feeds updated successfully", successful)
        } else {
            format!("[!] {} updated, {} failed", successful, failed)
        };
    }
    
    // Legacy synchronous refresh for initial load
    async fn refresh_feeds(&mut self) {
        self.status_message = "Refreshing feeds...".to_string();
        
        // Parallel feed fetching with improved error handling
        let fetch_futures: Vec<_> = self.feeds.iter().enumerate().map(|(i, feed)| {
            let url = feed.url.clone();
            let name = feed.name.clone();
            async move {
                let result = match fetch_feed_with_retry(&url, 3).await {
                    Ok(articles) => Ok(articles),
                    Err(e) => Err(e.to_string()),
                };
                (i, name, result)
            }
        }).collect();
        
        let results = future::join_all(fetch_futures).await;
        self.process_refresh_results(results);
    }
    
    async fn mark_article_as_read(&mut self, article_id: &str) -> Result<()> {
        self.app_state.read_articles.insert(article_id.to_string());
        
        // Update the article in the current feed
        if let FeedState::Loaded(ref mut articles) = &mut self.feeds[self.current_feed_index].state {
            if let Some(article) = articles.iter_mut().find(|a| a.id == article_id) {
                article.read = true;
            }
        }
        
        self.calculate_unread_count();
        save_app_state(&self.app_state).await?;
        Ok(())
    }

    fn current_articles(&self) -> &[Article] {
        match &self.feeds[self.current_feed_index].state {
            FeedState::Loaded(articles) => articles,
            _ => &[],
        }
    }

    fn current_article(&self) -> Option<&Article> {
        let articles = self.current_articles();
        articles.get(self.current_article_index)
    }

    fn next_feed(&mut self) {
        if self.feeds.is_empty() {
            return;
        }
        let i = match self.feed_list_state.selected() {
            Some(i) => {
                if i >= self.feeds.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.feed_list_state.select(Some(i));
        self.current_feed_index = i;
        self.article_list_state.select(Some(0));
        self.current_article_index = 0;
    }

    fn previous_feed(&mut self) {
        if self.feeds.is_empty() {
            return;
        }
        let i = match self.feed_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.feeds.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.feed_list_state.select(Some(i));
        self.current_feed_index = i;
        self.article_list_state.select(Some(0));
        self.current_article_index = 0;
    }

    fn next_article(&mut self) {
        let articles = self.current_articles();
        if !articles.is_empty() {
            let i = match self.article_list_state.selected() {
                Some(i) => {
                    if i >= articles.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.article_list_state.select(Some(i));
            self.current_article_index = i;
        }
    }

    fn previous_article(&mut self) {
        let articles = self.current_articles();
        if !articles.is_empty() {
            let i = match self.article_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        articles.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.article_list_state.select(Some(i));
            self.current_article_index = i;
        }
    }
    
    fn calculate_unread_count(&mut self) {
        self.unread_count = self.feeds
            .iter()
            .filter_map(|feed| {
                if let FeedState::Loaded(articles) = &feed.state {
                    Some(articles.iter().filter(|a| !a.read).count())
                } else {
                    None
                }
            })
            .sum();
    }
    
    // Since we already have parallel fetching in refresh_feeds(),
    // the background refresh is effectively implemented through the existing mechanism.
    // The refresh_feeds() function already uses parallel async operations
    // and doesn't block the UI when called.
}

async fn fetch_feed_with_retry(url: &str, max_retries: u32) -> Result<Vec<Article>> {
    let mut last_error = None;
    
    for attempt in 0..=max_retries {
        match fetch_feed_single(url).await {
            Ok(articles) => return Ok(articles),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    // Exponential backoff: wait 2^attempt seconds
                    let delay_ms = 1000 * 2u64.pow(attempt);
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
    
    // Return the last error if all retries failed
    Err(last_error.unwrap())
}

async fn fetch_feed_single(url: &str) -> Result<Vec<Article>> {
    // Create client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Yomi/1.0")
        .build()?;
    
    // Add timeout to the entire operation
    let result = timeout(Duration::from_secs(45), async {
        let response = client.get(url).send().await?;
        
        // Check if response is successful
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP {} - {}", 
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown error")
            ));
        }
        
        let content = response.bytes().await?;
        let feed = feed_rs::parser::parse(content.as_ref())?;
        
        let articles: Vec<Article> = feed.entries.into_iter().take(20).map(|entry| {
            let pub_date = entry.published
                .or(entry.updated)
                .and_then(|dt| DateTime::parse_from_rfc3339(&dt.to_rfc3339()).ok())
                .map(|dt| dt.with_timezone(&Local));
            
            Article::new(
                entry.title.map(|t| t.content).unwrap_or_else(|| "No title".to_string()),
                entry.links.first().map(|l| l.href.clone()).unwrap_or_default(),
                entry.summary
                    .map(|t| t.content)
                    .or_else(|| entry.content.and_then(|c| c.body.map(|b| b)))
                    .unwrap_or_else(|| "No description available".to_string()),
                pub_date,
            )
        }).collect();
        
        Ok(articles)
    }).await;
    
    match result {
        Ok(articles) => articles,
        Err(_) => Err(anyhow::anyhow!("Request timed out after 45 seconds")),
    }
}

fn load_config() -> Result<Config> {
    let config_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("yomi")
        .join("config.toml");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        Ok(toml::from_str(&content)?)
    } else {
        let config = Config::default();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_string = toml::to_string_pretty(&config)?;
        std::fs::write(&config_path, toml_string)?;
        Ok(config)
    }
}

fn save_config(config: &Config) -> Result<()> {
    let config_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("yomi")
        .join("config.toml");

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let toml_string = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, toml_string)?;
    Ok(())
}

// App state persistence functions
async fn load_app_state() -> Result<AppState> {
    let state_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("yomi")
        .join("state.toml");

    if state_path.exists() {
        let content = tokio::fs::read_to_string(&state_path).await?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(AppState::default())
    }
}

async fn save_app_state(state: &AppState) -> Result<()> {
    let state_path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("yomi")
        .join("state.toml");

    if let Some(parent) = state_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    let mut updated_state = state.clone();
    updated_state.last_update = Some(Utc::now());
    
    let toml_string = toml::to_string_pretty(&updated_state)?;
    tokio::fs::write(&state_path, toml_string).await?;
    Ok(())
}

// Background task handler for async feed refreshing
async fn background_task_handler(
    mut command_rx: mpsc::UnboundedReceiver<BackgroundCommand>,
    message_tx: mpsc::UnboundedSender<BackgroundMessage>,
) {
    while let Some(command) = command_rx.recv().await {
        match command {
            BackgroundCommand::StartRefresh(feed_data) => {
                // Notify that refresh started
                let _ = message_tx.send(BackgroundMessage::RefreshStarted);
                
                // Perform the refresh in background
                let results = perform_background_refresh(feed_data).await;
                
                // Send results back to main thread
                let _ = message_tx.send(BackgroundMessage::RefreshComplete(results));
            }
        }
    }
}

async fn perform_background_refresh(
    feed_data: Vec<(usize, String, String)>
) -> Vec<(usize, String, Result<Vec<Article>, String>)> {
    // Parallel feed fetching with improved error handling
    let fetch_futures: Vec<_> = feed_data.into_iter().map(|(i, name, url)| {
        async move {
            let result = match fetch_feed_with_retry(&url, 3).await {
                Ok(articles) => Ok(articles),
                Err(e) => Err(e.to_string()),
            };
            (i, name, result)
        }
    }).collect();
    
    future::join_all(fetch_futures).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.as_ref().unwrap_or(&Commands::Read) {
        Commands::Read => {
            // Start the TUI
            start_tui().await
        }
        Commands::Add { url, name } => {
            handle_add_feed(url, name.as_deref()).await
        }
        Commands::Remove { target } => {
            handle_remove_feed(target).await
        }
        Commands::List => {
            handle_list_feeds().await
        }
        Commands::Refresh { feed } => {
            handle_refresh(feed.as_deref()).await
        }
    }
}

async fn start_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

// CLI command handlers
async fn handle_add_feed(url: &str, name: Option<&str>) -> Result<()> {
    let mut config = load_config()?;
    
    match cli::add_feed(&mut config, url, name.map(String::from)).await {
        Ok(()) => {
            save_config(&config)?;
            let feed_name = config.feeds.iter()
                .find(|(_, u)| u == &url)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
                
            println!("✓ Added feed '{}' successfully!", feed_name);
            println!("  URL: {}", url);
        }
        Err(e) => {
            eprintln!("Error adding feed: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn handle_remove_feed(target: &str) -> Result<()> {
    let mut config = load_config()?;
    
    // Get the name before removing
    let feed_name = if let Ok(index) = target.parse::<usize>() {
        config.feeds.keys().nth(index).cloned()
    } else {
        Some(target.to_string())
    };
    
    match cli::remove_feed(&mut config, target) {
        Ok(()) => {
            save_config(&config)?;
            if let Some(name) = feed_name {
                println!("✓ Removed feed '{}' successfully!", name);
            } else {
                println!("✓ Feed removed successfully!");
            }
        }
        Err(e) => {
            eprintln!("Error removing feed: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn handle_list_feeds() -> Result<()> {
    let config = load_config()?;
    
    match cli::list_feeds(&config) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("Error listing feeds: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn handle_refresh(feed_name: Option<&str>) -> Result<()> {
    let config = load_config()?;
    
    match cli::refresh_feeds(&config, feed_name).await {
        Ok(()) => {},
        Err(e) => {
            eprintln!("Error refreshing feeds: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut app = App::new().await?;
    app.refresh_feeds().await;

    loop {
        // Handle background messages first
        app.handle_background_messages();
        
        // Use enhanced UI instead of the basic one
        terminal.draw(|f| render_enhanced_ui(f, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match handle_key_event(key.code, key.modifiers, &mut app).await {
                        Ok(should_continue) => {
                            if !should_continue {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error handling key event: {}", e);
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }

        // Auto-refresh every 10 minutes using background refresh
        if app.last_refresh.elapsed() > Duration::from_secs(600) && !app.is_refreshing {
            app.start_background_refresh().await;
        }
    }

    // Clean up background task before exit
    if let Some(handle) = app.bg_task_handle.take() {
        handle.abort();
    }
    
    // Save state before exit
    let _ = save_app_state(&app.app_state).await;
    Ok(())
}

async fn handle_key_event(key: KeyCode, _modifiers: KeyModifiers, app: &mut App) -> Result<bool> {
    if app.popup != PopUp::None {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.popup = PopUp::None;
            }
            _ => {}
        }
        return Ok(true);
    }

    match key {
        KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(false);
        }
        KeyCode::Char('?') => {
            app.popup = PopUp::Help;
        }
        KeyCode::Char('r') | KeyCode::F(5) => {
            app.start_background_refresh().await;
        }
        // Browser integration with mark as read
        KeyCode::Char('o') => {
            if let Some(article) = app.current_article() {
                let article_id = article.id.clone();
                let article_title = article.title.clone();
                let article_link = article.link.clone();
                let _ = open::that(&article_link);
                let _ = app.mark_article_as_read(&article_id).await;
                app.status_message = format!("🌐 Opened '{}' in browser", truncate_string(&article_title, 30));
            }
        }
        // Mark current article as read/unread toggle
        KeyCode::Char('m') => {
            if let Some(article) = app.current_article() {
                let article_id = article.id.clone();
                if article.read {
                    // Unmark as read
                    app.app_state.read_articles.remove(&article_id);
                    // Update the article in the current feed
                    if let FeedState::Loaded(ref mut articles) = &mut app.feeds[app.current_feed_index].state {
                        if let Some(article) = articles.iter_mut().find(|a| a.id == article_id) {
                            article.read = false;
                        }
                    }
                    app.status_message = "📄 Article marked as unread".to_string();
                } else {
                    let _ = app.mark_article_as_read(&article_id).await;
                    app.status_message = "✓ Article marked as read".to_string();
                }
                app.calculate_unread_count();
            }
        }
        // Quick navigation shortcuts
        KeyCode::Char('g') => {
            // Go to top
            match app.current_screen {
                CurrentScreen::FeedList => {
                    app.feed_list_state.select(Some(0));
                    app.current_feed_index = 0;
                }
                CurrentScreen::ArticleList => {
                    app.article_list_state.select(Some(0));
                    app.current_article_index = 0;
                }
                CurrentScreen::ArticleView => {
                    app.scroll_offset = 0;
                }
            }
        }
        KeyCode::Char('G') => {
            // Go to bottom
            match app.current_screen {
                CurrentScreen::FeedList => {
                    let last_index = app.feeds.len().saturating_sub(1);
                    app.feed_list_state.select(Some(last_index));
                    app.current_feed_index = last_index;
                }
                CurrentScreen::ArticleList => {
                    let articles = app.current_articles();
                    if !articles.is_empty() {
                        let last_index = articles.len().saturating_sub(1);
                        app.article_list_state.select(Some(last_index));
                        app.current_article_index = last_index;
                    }
                }
                CurrentScreen::ArticleView => {
                    app.scroll_offset = 100; // Large scroll to bottom
                }
            }
        }
        KeyCode::Tab => {
            match app.current_screen {
                CurrentScreen::FeedList => {
                    app.current_screen = CurrentScreen::ArticleList;
                    if app.article_list_state.selected().is_none() && !app.current_articles().is_empty() {
                        app.article_list_state.select(Some(0));
                        app.current_article_index = 0;
                    }
                }
                CurrentScreen::ArticleList => {
                    app.current_screen = CurrentScreen::FeedList;
                }
                CurrentScreen::ArticleView => {
                    app.current_screen = CurrentScreen::ArticleList;
                }
            }
        }
        KeyCode::Enter => {
            match app.current_screen {
                CurrentScreen::ArticleList => {
                    // Mark article as read when opening full view
                    if let Some(article) = app.current_article() {
                        let article_id = article.id.clone();
                        let _ = app.mark_article_as_read(&article_id).await;
                    }
                    app.current_screen = CurrentScreen::ArticleView;
                    app.scroll_offset = 0;
                }
                _ => {}
            }
        }
        KeyCode::Esc => {
            match app.current_screen {
                CurrentScreen::ArticleView => {
                    app.current_screen = CurrentScreen::ArticleList;
                }
                CurrentScreen::ArticleList => {
                    app.current_screen = CurrentScreen::FeedList;
                }
                _ => {}
            }
        }
        // Enhanced navigation - Arrow keys and vim-style
        KeyCode::Up | KeyCode::Char('k') => {
            match app.current_screen {
                CurrentScreen::FeedList => app.previous_feed(),
                CurrentScreen::ArticleList => app.previous_article(),
                CurrentScreen::ArticleView => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(1);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            match app.current_screen {
                CurrentScreen::FeedList => app.next_feed(),
                CurrentScreen::ArticleList => app.next_article(),
                CurrentScreen::ArticleView => {
                    app.scroll_offset += 1;
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            // Move to left pane or go back
            match app.current_screen {
                CurrentScreen::ArticleList => {
                    app.current_screen = CurrentScreen::FeedList;
                }
                CurrentScreen::ArticleView => {
                    app.current_screen = CurrentScreen::ArticleList;
                }
                _ => {}
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            // Move to right pane
            match app.current_screen {
                CurrentScreen::FeedList => {
                    app.current_screen = CurrentScreen::ArticleList;
                    if app.article_list_state.selected().is_none() && !app.current_articles().is_empty() {
                        app.article_list_state.select(Some(0));
                        app.current_article_index = 0;
                    }
                }
                CurrentScreen::ArticleList => {
                    if let Some(article) = app.current_article() {
                        let article_id = article.id.clone();
                        let _ = app.mark_article_as_read(&article_id).await;
                    }
                    app.current_screen = CurrentScreen::ArticleView;
                    app.scroll_offset = 0;
                }
                _ => {}
            }
        }
        // Page navigation
        KeyCode::PageUp => {
            match app.current_screen {
                CurrentScreen::ArticleView => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                }
                CurrentScreen::FeedList => {
                    for _ in 0..5 { app.previous_feed(); }
                }
                CurrentScreen::ArticleList => {
                    for _ in 0..5 { app.previous_article(); }
                }
            }
        }
        KeyCode::PageDown => {
            match app.current_screen {
                CurrentScreen::ArticleView => {
                    app.scroll_offset += 10;
                }
                CurrentScreen::FeedList => {
                    for _ in 0..5 { app.next_feed(); }
                }
                CurrentScreen::ArticleList => {
                    for _ in 0..5 { app.next_article(); }
                }
            }
        }
        // Mark all articles in current feed as read
        KeyCode::Char('A') => {
            if let FeedState::Loaded(ref mut articles) = &mut app.feeds[app.current_feed_index].state {
                let mut marked_count = 0;
                for article in articles.iter_mut() {
                    if !article.read {
                        article.read = true;
                        app.app_state.read_articles.insert(article.id.clone());
                        marked_count += 1;
                    }
                }
                app.calculate_unread_count();
                let _ = save_app_state(&app.app_state).await;
                app.status_message = format!("✓ Marked {} articles as read in current feed", marked_count);
            }
        }
        _ => {}
    }

    Ok(true)
}
