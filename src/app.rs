//! Application state and key handling.
//!
//! The edition is built once and then held still. Opening an article records
//! that you opened it, but the front page does not rearrange itself underneath
//! you — a page that reflows while you are reading it is the inbox behaviour
//! this design exists to get rid of.

use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::config::Config;
use crate::edition::{self, Edition, Placed};
use crate::extract;
use crate::layout::{self, Laid};
use crate::model::{Article, Event};
use crate::rank::{self, Context, FeedSignals};
use crate::store::Store;
use crate::theme::Theme;
use crate::vault;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Edition,
    Reading,
}

/// Work the app wants done that needs the network or the async runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Refresh,
    FullText { id: String, link: String },
}

pub struct App {
    pub store: Store,
    pub config: Config,
    pub theme: Theme,
    pub edition: Edition,

    pub view: View,
    pub cursor: usize,
    pub list_scroll: usize,
    pub scroll: u16,
    pub reading: Vec<Laid>,
    /// Height of the reading pane at the last draw, for paging and progress.
    pub reading_height: usize,

    pub status: String,
    pub refreshing: bool,
    pub show_why: bool,
    pub show_help: bool,
    pub quit: bool,
    pub now: DateTime<Utc>,

    /// Requests for the runtime to pick up.
    pub pending: Vec<Request>,
    /// Articles already counted as finished this session.
    finished: std::collections::HashSet<String>,
}

impl App {
    pub fn new(store: Store, config: Config) -> Result<App> {
        let theme = Theme::from_accent_name(&config.appearance.accent);
        let edition = edition::build(&store, config.window(), config.edition.size)?;
        Ok(App {
            store,
            config,
            theme,
            edition,
            view: View::Edition,
            cursor: 0,
            list_scroll: 0,
            scroll: 0,
            reading: Vec::new(),
            reading_height: 20,
            status: String::new(),
            refreshing: false,
            show_why: false,
            show_help: false,
            quit: false,
            now: Utc::now(),
            pending: Vec::new(),
            finished: std::collections::HashSet::new(),
        })
    }

    pub fn rebuild(&mut self) -> Result<()> {
        self.edition = edition::build(&self.store, self.config.window(), self.config.edition.size)?;
        self.cursor = self.cursor.min(self.rows().saturating_sub(1));
        self.list_scroll = 0;
        Ok(())
    }

    /// Selectable rows: the picked articles, then the held-back feeds.
    pub fn rows(&self) -> usize {
        self.edition.picked.len() + self.edition.held.len()
    }

    pub fn current_placed(&self) -> Option<&Placed> {
        self.edition.picked.get(self.cursor)
    }

    pub fn current_article(&self) -> Option<&Article> {
        self.current_placed().map(|p| &p.article)
    }

    fn on_held_row(&self) -> Option<usize> {
        let picked = self.edition.picked.len();
        if self.cursor >= picked {
            Some(self.cursor - picked)
        } else {
            None
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let rows = self.rows();
        if rows == 0 {
            return;
        }
        let next = self.cursor as isize + delta;
        self.cursor = next.clamp(0, rows as isize - 1) as usize;
        self.status.clear();
    }

    // ---- reading -----------------------------------------------------

    pub fn open_current(&mut self) -> Result<()> {
        let Some(article) = self.current_article().cloned() else {
            return Ok(());
        };
        self.store.record(&article, Event::Opened)?;
        self.view = View::Reading;
        self.scroll = 0;
        self.load_reading(&article);

        if article.body.is_none() && self.config.edition.full_text && !article.link.is_empty() {
            self.pending.push(Request::FullText {
                id: article.id.clone(),
                link: article.link.clone(),
            });
            self.status = "fetching the full text…".into();
        }
        Ok(())
    }

    fn load_reading(&mut self, article: &Article) {
        let source = article
            .body
            .clone()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or_else(|| article.summary.clone());
        let blocks = extract::markdown_to_blocks(&source);
        self.reading = layout::lay_out(&blocks, layout::MAX_MEASURE);
    }

    /// Called when the full text arrives for an article being read.
    pub fn full_text_arrived(&mut self, id: &str, markdown: &str) -> Result<()> {
        let words = extract::word_count(markdown);
        self.store.set_body(id, markdown, words)?;
        // Re-read rather than patching the in-memory copy, so the store stays
        // the only place an article is defined.
        if let Some(fresh) = self.store.article(id)? {
            if let Some(placed) = self.edition.picked.iter_mut().find(|p| p.article.id == id) {
                placed.article = fresh;
            }
        }
        if self.view == View::Reading {
            if let Some(a) = self.current_article().cloned() {
                if a.id == id {
                    self.load_reading(&a);
                    self.status.clear();
                }
            }
        }
        Ok(())
    }

    pub fn scroll_by(&mut self, delta: i32) -> Result<()> {
        let max = self.max_scroll();
        let next = (self.scroll as i32 + delta).clamp(0, max as i32);
        self.scroll = next as u16;
        self.check_finished()
    }

    pub fn max_scroll(&self) -> u16 {
        // The header block above the body is four lines of title and byline.
        let total = self.reading.len() + 5;
        total.saturating_sub(self.reading_height) as u16
    }

    /// Reaching the end is the signal the ranker learns from.
    fn check_finished(&mut self) -> Result<()> {
        if self.scroll < self.max_scroll() {
            return Ok(());
        }
        let Some(article) = self.current_article().cloned() else {
            return Ok(());
        };
        if self.finished.contains(&article.id) {
            return Ok(());
        }
        self.finished.insert(article.id.clone());
        self.store.record(&article, Event::Finished)?;
        Ok(())
    }

    pub fn next_article(&mut self) -> Result<()> {
        if self.cursor + 1 < self.edition.picked.len() {
            self.cursor += 1;
            self.open_current()?;
        } else {
            self.view = View::Edition;
            self.status = "that was the last one".into();
        }
        Ok(())
    }

    // ---- actions -----------------------------------------------------

    pub fn open_in_browser(&mut self) -> Result<()> {
        let Some(article) = self.current_article().cloned() else {
            return Ok(());
        };
        if article.link.is_empty() {
            self.status = "this item has no link".into();
            return Ok(());
        }
        match open::that(&article.link) {
            Ok(()) => {
                self.store.record(&article, Event::Browser)?;
                self.status = format!(
                    "opened {} in your browser",
                    crate::util::truncate_string(&article.title, 40)
                );
            }
            Err(e) => self.status = format!("could not open a browser: {e}"),
        }
        Ok(())
    }

    pub fn save_to_vault(&mut self) -> Result<()> {
        let Some(article) = self.current_article().cloned() else {
            return Ok(());
        };
        let Some(dir) = self.config.vault_dir() else {
            self.status = format!(
                "no vault set — add  [vault]  path = \"{}\"  to your config",
                crate::config::suggest_vault()
            );
            return Ok(());
        };
        match vault::save(&dir, &article, Utc::now()) {
            Ok(path) => {
                self.store.record(&article, Event::Saved)?;
                let name: String = path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.status = format!("saved {name}");
            }
            Err(e) => self.status = format!("could not save: {e}"),
        }
        Ok(())
    }

    pub fn mute_current_feed(&mut self) -> Result<()> {
        let feed_id = match self.on_held_row() {
            Some(i) => self.edition.held.get(i).map(|h| h.feed_id),
            None => self.current_article().map(|a| a.feed_id),
        };
        let Some(feed_id) = feed_id else {
            return Ok(());
        };
        self.store.set_muted(feed_id, true)?;
        self.rebuild()?;
        self.status = "muted — it stays subscribed but out of the edition".into();
        Ok(())
    }

    pub fn stop_boosting_current_feed(&mut self) -> Result<()> {
        let Some(feed_id) = self.current_article().map(|a| a.feed_id) else {
            return Ok(());
        };
        self.store.set_never_boost(feed_id, true)?;
        self.rebuild()?;
        self.show_why = false;
        self.status = "engagement will no longer push this feed up".into();
        Ok(())
    }

    /// Pull a held-back feed's articles into the edition.
    ///
    /// The held-back list is only trustworthy if it is one keypress from being
    /// undone, so this is deliberately cheap.
    pub fn reveal_held(&mut self, index: usize) -> Result<()> {
        let Some(held) = self.edition.held.get(index).cloned() else {
            return Ok(());
        };
        let candidates = self.store.candidates(self.config.window())?;
        let already: std::collections::HashSet<String> = self
            .edition
            .picked
            .iter()
            .map(|p| p.article.id.clone())
            .collect();

        let signals = FeedSignals {
            engagement: self.store.engagement(held.feed_id)?,
            never_boost: self
                .store
                .feeds()?
                .iter()
                .find(|f| f.id == held.feed_id)
                .map(|f| f.never_boost)
                .unwrap_or(false),
        };
        let ctx = Context {
            now: Utc::now(),
            finished_mean: self.store.finished_length_mean()?,
            rules: self.store.rules()?,
        };

        let mut added = 0;
        for article in candidates
            .into_iter()
            .filter(|a| a.feed_id == held.feed_id && !already.contains(&a.id))
        {
            let score = rank::score(&article, &signals, &ctx, 0, 0);
            if score.vetoed {
                continue;
            }
            self.edition.picked.push(Placed { article, score });
            added += 1;
        }

        self.edition.held.remove(index);
        self.cursor = self.cursor.min(self.rows().saturating_sub(1));
        self.status = if added == 0 {
            format!("nothing left to show from {}", held.feed_name)
        } else {
            format!("added {added} from {}", held.feed_name)
        };
        Ok(())
    }

    // ---- keys --------------------------------------------------------

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        // Overlays swallow keys, but only the ones they actually use.
        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return Ok(());
        }
        if self.show_why {
            match key.code {
                KeyCode::Esc | KeyCode::Char('w') | KeyCode::Char('q') => self.show_why = false,
                KeyCode::Char('d') => self.stop_boosting_current_feed()?,
                _ => {}
            }
            return Ok(());
        }

        match self.view {
            View::Edition => self.key_edition(key.code),
            View::Reading => self.key_reading(key.code),
        }
    }

    fn key_edition(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_cursor(-1),
            KeyCode::Char('g') | KeyCode::Home => {
                self.cursor = 0;
                self.list_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.cursor = self.rows().saturating_sub(1);
            }
            KeyCode::Enter => match self.on_held_row() {
                Some(i) => self.reveal_held(i)?,
                None => self.open_current()?,
            },
            KeyCode::Char('o') => self.open_in_browser()?,
            KeyCode::Char('s') => self.save_to_vault()?,
            KeyCode::Char('w') if self.current_placed().is_some() => self.show_why = true,
            KeyCode::Char('m') => self.mute_current_feed()?,
            KeyCode::Char('r') => {
                if !self.refreshing {
                    self.refreshing = true;
                    self.status = "refreshing…".into();
                    self.pending.push(Request::Refresh);
                }
            }
            KeyCode::Char('?') => self.show_help = true,
            _ => {}
        }
        Ok(())
    }

    fn key_reading(&mut self, code: KeyCode) -> Result<()> {
        let page = self.reading_height.saturating_sub(2).max(1) as i32;
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.view = View::Edition;
                self.status.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => self.scroll_by(1)?,
            KeyCode::Char('k') | KeyCode::Up => self.scroll_by(-1)?,
            KeyCode::Char(' ') | KeyCode::PageDown => self.scroll_by(page)?,
            KeyCode::PageUp => self.scroll_by(-page)?,
            KeyCode::Char('g') | KeyCode::Home => self.scroll = 0,
            KeyCode::Char('G') | KeyCode::End => {
                self.scroll = self.max_scroll();
                self.check_finished()?;
            }
            KeyCode::Char('o') => self.open_in_browser()?,
            KeyCode::Char('s') => self.save_to_vault()?,
            KeyCode::Char('n') => self.next_article()?,
            KeyCode::Char('w') if self.current_placed().is_some() => self.show_why = true,
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::IncomingArticle;
    use chrono::Duration;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn code(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn incoming(id: &str, title: &str) -> IncomingArticle {
        IncomingArticle {
            id: id.into(),
            title: title.into(),
            author: None,
            link: format!("https://example.invalid/{id}"),
            summary: "the summary".into(),
            body: Some("Some body text that goes on for a while.".into()),
            published: Some(Utc::now() - Duration::hours(2)),
            word_count: 300,
        }
    }

    fn app_with(n: usize) -> App {
        let mut store = Store::in_memory().unwrap();
        let f = store
            .add_feed("example", "https://example.invalid/feed")
            .unwrap();
        let batch: Vec<IncomingArticle> = (0..n)
            .map(|i| incoming(&format!("a{i}"), &format!("Story number {i}")))
            .collect();
        store.ingest(f, &batch).unwrap();
        App::new(store, Config::default()).unwrap()
    }

    #[test]
    fn the_cursor_cannot_leave_the_list() {
        let mut app = app_with(3);
        app.move_cursor(-5);
        assert_eq!(app.cursor, 0);
        app.move_cursor(99);
        assert_eq!(app.cursor, app.rows() - 1);
    }

    #[test]
    fn moving_the_cursor_on_an_empty_edition_is_harmless() {
        let mut app = app_with(0);
        assert_eq!(app.rows(), 0);
        app.move_cursor(1);
        app.move_cursor(-1);
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn q_quits_from_the_edition() {
        let mut app = app_with(2);
        app.handle_key(key('q')).unwrap();
        assert!(app.quit);
    }

    #[test]
    fn q_leaves_the_article_rather_than_the_app() {
        let mut app = app_with(2);
        app.open_current().unwrap();
        assert_eq!(app.view, View::Reading);
        app.handle_key(key('q')).unwrap();
        assert_eq!(app.view, View::Edition);
        assert!(!app.quit, "q in an article must not quit yomi");
    }

    #[test]
    fn opening_an_article_records_it() {
        let mut app = app_with(2);
        let id = app.current_article().unwrap().id.clone();
        app.open_current().unwrap();
        let article = app.store.article(&id).unwrap().unwrap();
        let e = app.store.engagement(article.feed_id).unwrap();
        assert_eq!(e.opened, 1);
    }

    #[test]
    fn the_edition_does_not_reflow_while_you_read_it() {
        let mut app = app_with(4);
        let before: Vec<String> = app
            .edition
            .picked
            .iter()
            .map(|p| p.article.id.clone())
            .collect();
        app.open_current().unwrap();
        app.handle_key(key('q')).unwrap();
        let after: Vec<String> = app
            .edition
            .picked
            .iter()
            .map(|p| p.article.id.clone())
            .collect();
        assert_eq!(before, after, "reading must not rearrange the front page");
    }

    #[test]
    fn reaching_the_end_counts_as_finished_once() {
        let mut app = app_with(1);
        app.reading_height = 3;
        app.open_current().unwrap();
        let feed_id = app.current_article().unwrap().feed_id;

        app.handle_key(key('G')).unwrap();
        app.handle_key(key('G')).unwrap();
        app.handle_key(key('j')).unwrap();

        assert_eq!(app.store.engagement(feed_id).unwrap().finished, 1);
    }

    #[test]
    fn scrolling_partway_does_not_count_as_finished() {
        let mut app = app_with(1);
        app.reading_height = 2;
        app.open_current().unwrap();
        let feed_id = app.current_article().unwrap().feed_id;
        // Give it something long enough that one line is not the end.
        app.reading = vec![
            Laid {
                role: crate::layout::Role::Body,
                text: "x".into()
            };
            80
        ];
        app.scroll_by(1).unwrap();
        assert_eq!(app.store.engagement(feed_id).unwrap().finished, 0);
    }

    #[test]
    fn scrolling_stops_at_the_ends() {
        let mut app = app_with(1);
        app.reading_height = 5;
        app.open_current().unwrap();
        app.scroll_by(-100).unwrap();
        assert_eq!(app.scroll, 0);
        app.scroll_by(10_000).unwrap();
        assert_eq!(app.scroll, app.max_scroll());
    }

    #[test]
    fn the_why_panel_only_opens_on_a_real_article() {
        let mut app = app_with(0);
        app.handle_key(key('w')).unwrap();
        assert!(!app.show_why);

        let mut app = app_with(2);
        app.handle_key(key('w')).unwrap();
        assert!(app.show_why);
    }

    #[test]
    fn the_why_panel_swallows_navigation_but_not_its_own_keys() {
        let mut app = app_with(3);
        app.handle_key(key('w')).unwrap();
        app.handle_key(key('j')).unwrap();
        assert_eq!(app.cursor, 0, "j must not move the list behind the panel");
        app.handle_key(code(KeyCode::Esc)).unwrap();
        assert!(!app.show_why);
    }

    #[test]
    fn d_in_the_why_panel_stops_the_feed_being_boosted() {
        let mut app = app_with(2);
        let feed_id = app.current_article().unwrap().feed_id;
        app.handle_key(key('w')).unwrap();
        app.handle_key(key('d')).unwrap();
        assert!(
            app.store
                .feeds()
                .unwrap()
                .iter()
                .find(|f| f.id == feed_id)
                .unwrap()
                .never_boost
        );
        assert!(!app.show_why, "acting on the panel should close it");
    }

    #[test]
    fn help_swallows_keys_until_dismissed() {
        let mut app = app_with(3);
        app.handle_key(key('?')).unwrap();
        app.handle_key(key('q')).unwrap();
        assert!(!app.quit, "q should close help, not quit");
        assert!(!app.show_help);
    }

    #[test]
    fn muting_a_feed_removes_it_from_the_edition() {
        let mut app = app_with(3);
        assert!(!app.edition.picked.is_empty());
        app.handle_key(key('m')).unwrap();
        assert!(app.edition.picked.is_empty());
        assert!(
            app.edition.held.is_empty(),
            "a muted feed is not held back, it is gone"
        );
    }

    #[test]
    fn revealing_a_held_feed_adds_its_articles() {
        // Edition of 1, three arrivals: two are held back.
        let mut store = Store::in_memory().unwrap();
        let f = store
            .add_feed("example", "https://example.invalid/feed")
            .unwrap();
        let batch: Vec<IncomingArticle> = (0..3)
            .map(|i| incoming(&format!("a{i}"), &format!("Story {i}")))
            .collect();
        store.ingest(f, &batch).unwrap();
        let mut config = Config::default();
        config.edition.size = 1;
        let mut app = App::new(store, config).unwrap();

        assert_eq!(app.edition.picked.len(), 1);
        assert_eq!(app.edition.held.len(), 1);

        app.cursor = 1; // the held row
        app.handle_key(code(KeyCode::Enter)).unwrap();

        assert_eq!(app.edition.picked.len(), 3);
        assert!(app.edition.held.is_empty());
    }

    #[test]
    fn enter_on_an_article_reads_it_rather_than_revealing() {
        let mut app = app_with(2);
        app.handle_key(code(KeyCode::Enter)).unwrap();
        assert_eq!(app.view, View::Reading);
    }

    #[test]
    fn refresh_is_requested_once_not_queued_repeatedly() {
        let mut app = app_with(1);
        app.handle_key(key('r')).unwrap();
        app.handle_key(key('r')).unwrap();
        assert_eq!(
            app.pending
                .iter()
                .filter(|r| **r == Request::Refresh)
                .count(),
            1
        );
    }

    #[test]
    fn saving_without_a_vault_explains_rather_than_failing() {
        let mut app = app_with(1);
        app.handle_key(key('s')).unwrap();
        assert!(app.status.contains("no vault set"), "{}", app.status);
    }

    #[test]
    fn saving_writes_a_note_and_records_it() {
        let dir = std::env::temp_dir().join(format!("yomi-app-vault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = app_with(1);
        app.config.vault.path = Some(dir.to_string_lossy().to_string());

        let feed_id = app.current_article().unwrap().feed_id;
        app.handle_key(key('s')).unwrap();

        assert!(app.status.starts_with("saved "), "{}", app.status);
        assert_eq!(app.store.saves_by_feed(feed_id).unwrap(), 1);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn full_text_replaces_the_summary_in_place() {
        let mut app = app_with(1);
        let id = app.current_article().unwrap().id.clone();
        app.open_current().unwrap();
        app.full_text_arrived(&id, "# Real heading\n\nThe genuine article text.")
            .unwrap();

        let stored = app.store.article(&id).unwrap().unwrap();
        assert!(stored.body.unwrap().contains("genuine article text"));
        assert!(app
            .reading
            .iter()
            .any(|l| l.text.contains("genuine article text")));
    }

    #[test]
    fn an_article_with_no_body_asks_for_the_full_text() {
        let mut store = Store::in_memory().unwrap();
        let f = store
            .add_feed("example", "https://example.invalid/feed")
            .unwrap();
        let mut a = incoming("a0", "Truncated");
        a.body = None;
        store.ingest(f, &[a]).unwrap();
        let mut app = App::new(store, Config::default()).unwrap();

        app.open_current().unwrap();
        assert!(matches!(
            app.pending.first(),
            Some(Request::FullText { .. })
        ));
    }

    #[test]
    fn an_article_that_already_has_a_body_asks_for_nothing() {
        let mut app = app_with(1);
        app.open_current().unwrap();
        assert!(app.pending.is_empty());
    }

    #[test]
    fn n_moves_to_the_next_article_and_stops_at_the_end() {
        let mut app = app_with(2);
        app.open_current().unwrap();
        app.handle_key(key('n')).unwrap();
        assert_eq!(app.cursor, 1);
        assert_eq!(app.view, View::Reading);
        app.handle_key(key('n')).unwrap();
        assert_eq!(app.view, View::Edition);
        assert_eq!(app.status, "that was the last one");
    }
}
