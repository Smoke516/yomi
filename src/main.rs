//! yomi — a daily edition for your feeds.

use std::io;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

mod app;
mod cli;
mod config;
mod edition;
mod extract;
mod fetch;
mod layout;
mod model;
mod rank;
mod store;
mod theme;
mod ui;
mod util;
mod vault;

use app::{App, Request};
use cli::{Cli, Command};
use config::Config;
use store::{IncomingArticle, Store};

/// Something a background task finished doing.
enum Message {
    Fetched {
        feed_id: i64,
        name: String,
        result: Result<Vec<IncomingArticle>, String>,
    },
    RefreshDone,
    FullText {
        id: String,
        result: Result<String, String>,
    },
}

/// Die quietly when a pipe closes.
///
/// Rust masks SIGPIPE at startup, which turns `yomi today | head` into a panic
/// on a broken pipe instead of the silent exit every other Unix tool manages.
/// A reader that advertises itself as pipeable has to actually be pipeable.
#[cfg(unix)]
fn restore_sigpipe() {
    // SAFETY: setting a signal disposition to the default before any threads
    // are spawned is the documented way to undo Rust's SIGPIPE masking.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_sigpipe() {}

#[tokio::main]
async fn main() -> Result<()> {
    restore_sigpipe();
    let args = Cli::parse();
    let config = Config::load()?;
    let mut store = Store::open(&config::store_path()?)?;

    match args.command {
        Some(Command::Add { url, name }) => cli::add(&store, &url, name).await,
        Some(Command::Remove { target }) => cli::remove(&store, &target),
        Some(Command::List) => cli::list(&store),
        Some(Command::Refresh) => cli::refresh(&mut store).await,
        Some(Command::Today { json }) => cli::today(&store, &config, json),
        Some(Command::Why { position }) => cli::why(&store, &config, position),
        Some(Command::Status) => cli::status(&store, &config),
        Some(Command::Config) => cli::write_config(&config),
        Some(Command::Rule { action }) => cli::rule(&store, &action),
        Some(Command::Read) | None => run(store, config).await,
    }
}

async fn run(store: Store, config: Config) -> Result<()> {
    let mut app = App::new(store, config)?;
    let client = fetch::client()?;
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let mut terminal = enter_terminal()?;
    let result = event_loop(&mut terminal, &mut app, &client, &tx, &mut rx).await;
    leave_terminal(&mut terminal)?;
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    client: &reqwest::Client,
    tx: &mpsc::UnboundedSender<Message>,
    rx: &mut mpsc::UnboundedReceiver<Message>,
) -> Result<()> {
    let mut outstanding_fetches = 0usize;
    // Only paint when something actually changed. A reader that sits open all
    // day has no business waking up eight times a second to redraw the same
    // frame.
    let mut dirty = true;
    let mut last_paint = std::time::Instant::now();

    loop {
        // Relative ages ("4h") go stale on their own, so repaint occasionally
        // even when nothing happened.
        if last_paint.elapsed() >= Duration::from_secs(60) {
            dirty = true;
        }
        if dirty {
            terminal.draw(|f| {
                // The reading view needs to know its own height for paging.
                app.reading_height = f.area().height.saturating_sub(3) as usize;
                ui::draw(f, app);
            })?;
            dirty = false;
            last_paint = std::time::Instant::now();
        }

        if app.quit {
            return Ok(());
        }

        // Hand any queued work to the runtime.
        for request in std::mem::take(&mut app.pending) {
            match request {
                Request::Refresh => {
                    let feeds = app.store.feeds()?;
                    outstanding_fetches = feeds.len();
                    if feeds.is_empty() {
                        app.refreshing = false;
                        app.status = "no feeds to refresh".into();
                        continue;
                    }
                    for feed in feeds {
                        let client = client.clone();
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            let result = fetch::fetch_feed(&client, &feed.url)
                                .await
                                .map_err(|e| e.to_string());
                            let _ = tx.send(Message::Fetched {
                                feed_id: feed.id,
                                name: feed.name,
                                result,
                            });
                        });
                    }
                }
                Request::FullText { id, link } => {
                    let client = client.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let result = fetch::fetch_full_text(&client, &link)
                            .await
                            .map_err(|e| e.to_string());
                        let _ = tx.send(Message::FullText { id, result });
                    });
                }
            }
        }

        // Drain anything that finished.
        while let Ok(message) = rx.try_recv() {
            dirty = true;
            match message {
                Message::Fetched {
                    feed_id,
                    name,
                    result,
                } => {
                    match result {
                        Ok(items) => {
                            let n = app.store.ingest(feed_id, &items)?;
                            app.store.mark_fetched(feed_id)?;
                            if n > 0 {
                                app.status = format!("{name}: {n} new");
                            }
                        }
                        Err(e) => app.status = format!("{name}: {e}"),
                    }
                    outstanding_fetches = outstanding_fetches.saturating_sub(1);
                    if outstanding_fetches == 0 {
                        let _ = tx.send(Message::RefreshDone);
                    }
                }
                Message::RefreshDone => {
                    app.refreshing = false;
                    app.rebuild()?;
                    app.status = format!(
                        "{} picked from {}",
                        app.edition.picked.len(),
                        app.edition.considered
                    );
                }
                Message::FullText { id, result } => match result {
                    Ok(markdown) => app.full_text_arrived(&id, &markdown)?,
                    Err(_) => {
                        if app.status.starts_with("fetching") {
                            app.status = "only the feed summary was available".into();
                        }
                    }
                },
            }
        }

        // Poll rather than block, so background messages land promptly.
        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key(key)?;
                    dirty = true;
                }
                Event::Resize(..) => dirty = true,
                _ => {}
            }
        }
        app.now = chrono::Utc::now();
    }
}

fn enter_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    // A panic with the terminal in raw mode leaves the shell without echo.
    // Restore it first, then let the default hook print the message.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn leave_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
