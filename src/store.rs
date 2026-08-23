//! The local article store.
//!
//! Everything Yomi knows lives in one SQLite file: the feeds, every article it
//! has ever seen, what you did with them, and your rules. Nothing leaves the
//! machine.
//!
//! Keeping articles rather than refetching them is what makes the rest of the
//! app possible — the ranker needs a history to learn from, the edition needs
//! to know what it showed you yesterday, and reading on a train needs the text
//! to already be here.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::model::{Article, Event, Feed, Field, Rule};

/// An article as parsed from a feed, before it has a place in the store.
#[derive(Debug, Clone)]
pub struct IncomingArticle {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub link: String,
    pub summary: String,
    pub body: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub word_count: i64,
}

/// How a feed has been doing with this reader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeedEngagement {
    pub opened: i64,
    pub finished: i64,
}

impl FeedEngagement {
    /// Smoothed finish rate. With no history this is exactly 0.5, so a new
    /// feed is neither promoted nor punished.
    pub fn finish_rate(&self) -> f64 {
        (self.finished as f64 + 1.0) / (self.opened as f64 + 2.0)
    }
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &std::path::Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("could not create {}", dir.display()))?;
        }
        let conn =
            Connection::open(path).with_context(|| format!("could not open {}", path.display()))?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Store> {
        let store = Store {
            conn: Connection::open_in_memory()?,
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS feeds (
                id            INTEGER PRIMARY KEY,
                name          TEXT NOT NULL UNIQUE,
                url           TEXT NOT NULL UNIQUE,
                muted         INTEGER NOT NULL DEFAULT 0,
                never_boost   INTEGER NOT NULL DEFAULT 0,
                etag          TEXT,
                last_modified TEXT,
                last_fetch    TEXT
            );

            CREATE TABLE IF NOT EXISTS articles (
                id         TEXT PRIMARY KEY,
                feed_id    INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                title      TEXT NOT NULL,
                author     TEXT,
                link       TEXT NOT NULL,
                summary    TEXT NOT NULL,
                body       TEXT,
                published  TEXT,
                first_seen TEXT NOT NULL,
                word_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_articles_feed ON articles(feed_id);
            CREATE INDEX IF NOT EXISTS idx_articles_seen ON articles(first_seen);

            CREATE TABLE IF NOT EXISTS events (
                id         INTEGER PRIMARY KEY,
                article_id TEXT NOT NULL,
                feed_id    INTEGER NOT NULL,
                kind       TEXT NOT NULL,
                at         TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_article ON events(article_id);
            CREATE INDEX IF NOT EXISTS idx_events_feed ON events(feed_id);

            CREATE TABLE IF NOT EXISTS rules (
                id      INTEGER PRIMARY KEY,
                pattern TEXT NOT NULL,
                field   TEXT NOT NULL,
                weight  REAL NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    // ---- feeds -------------------------------------------------------

    /// Add a feed, or return the existing one's id if the URL is already known.
    pub fn add_feed(&self, name: &str, url: &str) -> Result<i64> {
        if let Some(id) = self.feed_id_by_url(url)? {
            return Ok(id);
        }
        self.conn
            .execute(
                "INSERT INTO feeds (name, url) VALUES (?1, ?2)",
                params![name, url],
            )
            .context("could not add feed")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn feed_id_by_url(&self, url: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT id FROM feeds WHERE url = ?1", params![url], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn feeds(&self) -> Result<Vec<Feed>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, url, muted, never_boost FROM feeds ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Feed {
                id: r.get(0)?,
                name: r.get(1)?,
                url: r.get(2)?,
                muted: r.get::<_, i64>(3)? != 0,
                never_boost: r.get::<_, i64>(4)? != 0,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Look a feed up by name (case-insensitive) or by its index in `feeds()`.
    ///
    /// Both forms exist because names are stable and indices are convenient;
    /// indices renumber after a removal, so scripts should prefer names.
    pub fn find_feed(&self, target: &str) -> Result<Option<Feed>> {
        let feeds = self.feeds()?;
        if let Ok(idx) = target.parse::<usize>() {
            return Ok(feeds.get(idx).cloned());
        }
        let needle = target.to_lowercase();
        Ok(feeds
            .iter()
            .find(|f| f.name.to_lowercase() == needle)
            .cloned())
    }

    /// Remove a feed by exact name, or by its index in `feeds()`.
    pub fn remove_feed(&self, target: &str) -> Result<Option<String>> {
        let Some(feed) = self.find_feed(target)? else {
            return Ok(None);
        };
        self.conn
            .execute("DELETE FROM feeds WHERE id = ?1", params![feed.id])?;
        Ok(Some(feed.name))
    }

    pub fn set_never_boost(&self, feed_id: i64, value: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE feeds SET never_boost = ?2 WHERE id = ?1",
            params![feed_id, value as i64],
        )?;
        Ok(())
    }

    pub fn set_muted(&self, feed_id: i64, value: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE feeds SET muted = ?2 WHERE id = ?1",
            params![feed_id, value as i64],
        )?;
        Ok(())
    }

    pub fn mark_fetched(&self, feed_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE feeds SET last_fetch = ?2 WHERE id = ?1",
            params![feed_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    // ---- articles ----------------------------------------------------

    /// Insert everything we have not seen before. Returns how many were new.
    ///
    /// Existing rows are left alone rather than overwritten: `first_seen` is
    /// what the edition sorts on, and a feed that re-publishes an item should
    /// not be able to push it back to the top.
    pub fn ingest(&mut self, feed_id: i64, incoming: &[IncomingArticle]) -> Result<usize> {
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction()?;
        let mut new = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO articles
                   (id, feed_id, title, author, link, summary, body, published, first_seen, word_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for a in incoming {
                let changed = stmt.execute(params![
                    a.id,
                    feed_id,
                    a.title,
                    a.author,
                    a.link,
                    a.summary,
                    a.body,
                    a.published.map(|d| d.to_rfc3339()),
                    now,
                    a.word_count,
                ])?;
                new += changed;
            }
        }
        tx.commit()?;
        Ok(new)
    }

    pub fn set_body(&self, article_id: &str, body: &str, word_count: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE articles SET body = ?2, word_count = ?3 WHERE id = ?1",
            params![article_id, body, word_count],
        )?;
        Ok(())
    }

    /// Articles that have never been opened and appeared inside `window`.
    ///
    /// This is the pool the edition is built from. Muted feeds are excluded
    /// here rather than filtered later, so a muted feed cannot even appear in
    /// the held-back list.
    pub fn candidates(&self, window: Duration) -> Result<Vec<Article>> {
        let cutoff = (Utc::now() - window).to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.feed_id, f.name, a.title, a.author, a.link, a.summary,
                    a.body, a.published, a.first_seen, a.word_count
               FROM articles a
               JOIN feeds f ON f.id = a.feed_id
              WHERE f.muted = 0
                AND COALESCE(a.published, a.first_seen) >= ?1
                AND NOT EXISTS (
                      SELECT 1 FROM events e
                       WHERE e.article_id = a.id AND e.kind IN ('opened','browser','saved')
                    )
              ORDER BY COALESCE(a.published, a.first_seen) DESC",
        )?;
        let rows = stmt.query_map(params![cutoff], row_to_article)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn article(&self, id: &str) -> Result<Option<Article>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.feed_id, f.name, a.title, a.author, a.link, a.summary,
                    a.body, a.published, a.first_seen, a.word_count
               FROM articles a JOIN feeds f ON f.id = a.feed_id
              WHERE a.id = ?1",
        )?;
        Ok(stmt.query_row(params![id], row_to_article).optional()?)
    }

    pub fn article_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM articles", [], |r| r.get(0))?)
    }

    /// How many articles arrived from each feed inside `window`, regardless of
    /// whether they made the edition. This is what the held-back list counts.
    pub fn arrivals_by_feed(&self, window: Duration) -> Result<Vec<(i64, String, i64)>> {
        let cutoff = (Utc::now() - window).to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.name, COUNT(*)
               FROM articles a JOIN feeds f ON f.id = a.feed_id
              WHERE f.muted = 0 AND COALESCE(a.published, a.first_seen) >= ?1
              GROUP BY f.id, f.name",
        )?;
        let rows = stmt.query_map(params![cutoff], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    // ---- events ------------------------------------------------------

    pub fn record(&self, article: &Article, event: Event) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (article_id, feed_id, kind, at) VALUES (?1, ?2, ?3, ?4)",
            params![
                article.id,
                article.feed_id,
                event.as_str(),
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn engagement(&self, feed_id: i64) -> Result<FeedEngagement> {
        let opened: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT article_id) FROM events WHERE feed_id = ?1 AND kind = 'opened'",
            params![feed_id],
            |r| r.get(0),
        )?;
        let finished: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT article_id) FROM events WHERE feed_id = ?1 AND kind = 'finished'",
            params![feed_id],
            |r| r.get(0),
        )?;
        Ok(FeedEngagement { opened, finished })
    }

    /// Mean length of the articles this reader actually finished. `None` until
    /// there is enough history for the number to mean anything.
    pub fn finished_length_mean(&self) -> Result<Option<f64>> {
        let (count, total): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(a.word_count), 0)
               FROM events e JOIN articles a ON a.id = e.article_id
              WHERE e.kind = 'finished' AND a.word_count > 0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if count < 5 {
            return Ok(None);
        }
        Ok(Some(total as f64 / count as f64))
    }

    /// Feeds this reader has saved into the vault, and how often.
    pub fn saves_by_feed(&self, feed_id: i64) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM events WHERE feed_id = ?1 AND kind = 'saved'",
            params![feed_id],
            |r| r.get(0),
        )?)
    }

    // ---- rules -------------------------------------------------------

    pub fn rules(&self) -> Result<Vec<Rule>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, pattern, field, weight FROM rules ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            let field: String = r.get(2)?;
            Ok(Rule {
                id: r.get(0)?,
                pattern: r.get(1)?,
                field: Field::parse(&field).unwrap_or(Field::Any),
                weight: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn add_rule(&self, pattern: &str, field: Field, weight: f64) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO rules (pattern, field, weight) VALUES (?1, ?2, ?3)",
            params![pattern, field.as_str(), weight],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_rule(&self, id: i64) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM rules WHERE id = ?1", params![id])?
            > 0)
    }
}

fn row_to_article(r: &rusqlite::Row) -> rusqlite::Result<Article> {
    let published: Option<String> = r.get(8)?;
    let first_seen: String = r.get(9)?;
    Ok(Article {
        id: r.get(0)?,
        feed_id: r.get(1)?,
        feed_name: r.get(2)?,
        title: r.get(3)?,
        author: r.get(4)?,
        link: r.get(5)?,
        summary: r.get(6)?,
        body: r.get(7)?,
        published: published.and_then(|s| parse_ts(&s)),
        first_seen: parse_ts(&first_seen).unwrap_or_else(Utc::now),
        word_count: r.get(10)?,
    })
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incoming(id: &str, title: &str) -> IncomingArticle {
        IncomingArticle {
            id: id.into(),
            title: title.into(),
            author: None,
            link: format!("https://example.invalid/{id}"),
            summary: "summary".into(),
            body: None,
            published: Some(Utc::now()),
            word_count: 500,
        }
    }

    fn store_with_feed() -> (Store, i64) {
        let mut s = Store::in_memory().unwrap();
        let id = s.add_feed("lwn", "https://lwn.net/feed").unwrap();
        s.ingest(id, &[incoming("a1", "one")]).unwrap();
        (s, id)
    }

    #[test]
    fn adding_the_same_url_twice_returns_the_same_feed() {
        let s = Store::in_memory().unwrap();
        let a = s.add_feed("lwn", "https://lwn.net/feed").unwrap();
        let b = s.add_feed("lwn again", "https://lwn.net/feed").unwrap();
        assert_eq!(a, b);
        assert_eq!(s.feeds().unwrap().len(), 1);
    }

    #[test]
    fn ingest_is_idempotent() {
        let mut s = Store::in_memory().unwrap();
        let f = s.add_feed("lwn", "https://lwn.net/feed").unwrap();
        let batch = vec![incoming("a1", "one"), incoming("a2", "two")];
        assert_eq!(s.ingest(f, &batch).unwrap(), 2);
        assert_eq!(
            s.ingest(f, &batch).unwrap(),
            0,
            "re-ingest must add nothing"
        );
        assert_eq!(s.article_count().unwrap(), 2);
    }

    #[test]
    fn reingesting_does_not_move_first_seen() {
        let mut s = Store::in_memory().unwrap();
        let f = s.add_feed("lwn", "https://lwn.net/feed").unwrap();
        s.ingest(f, &[incoming("a1", "one")]).unwrap();
        let before = s.article("a1").unwrap().unwrap().first_seen;
        s.ingest(f, &[incoming("a1", "one, revised")]).unwrap();
        let after = s.article("a1").unwrap().unwrap();
        assert_eq!(after.first_seen, before);
        assert_eq!(after.title, "one", "existing rows are not overwritten");
    }

    #[test]
    fn removing_a_feed_takes_its_articles_with_it() {
        let (s, _) = store_with_feed();
        assert_eq!(s.article_count().unwrap(), 1);
        assert_eq!(s.remove_feed("lwn").unwrap().as_deref(), Some("lwn"));
        assert_eq!(s.article_count().unwrap(), 0);
    }

    #[test]
    fn feeds_can_be_removed_by_index() {
        let s = Store::in_memory().unwrap();
        s.add_feed("alpha", "https://a.invalid/f").unwrap();
        s.add_feed("beta", "https://b.invalid/f").unwrap();
        assert_eq!(s.remove_feed("1").unwrap().as_deref(), Some("beta"));
        assert_eq!(s.feeds().unwrap().len(), 1);
    }

    #[test]
    fn feeds_are_found_by_name_or_index() {
        let s = Store::in_memory().unwrap();
        s.add_feed("alpha", "https://a.invalid/f").unwrap();
        s.add_feed("beta", "https://b.invalid/f").unwrap();
        assert_eq!(s.find_feed("beta").unwrap().unwrap().name, "beta");
        assert_eq!(s.find_feed("1").unwrap().unwrap().name, "beta");
        assert!(s.find_feed("gamma").unwrap().is_none());
        assert!(s.find_feed("99").unwrap().is_none());
    }

    #[test]
    fn feed_lookup_is_case_insensitive_including_non_ascii() {
        let s = Store::in_memory().unwrap();
        s.add_feed("Krebs on Security", "https://k.invalid/f")
            .unwrap();
        s.add_feed("Élan Café", "https://e.invalid/f").unwrap();
        assert!(s.find_feed("krebs on security").unwrap().is_some());
        assert!(s.find_feed("KREBS ON SECURITY").unwrap().is_some());
        // eq_ignore_ascii_case would miss this one.
        assert!(s.find_feed("élan café").unwrap().is_some());
    }

    #[test]
    fn muting_is_reversible() {
        let (s, feed) = store_with_feed();
        assert!(!s.feeds().unwrap()[0].muted);
        s.set_muted(feed, true).unwrap();
        assert!(s.feeds().unwrap()[0].muted);
        s.set_muted(feed, false).unwrap();
        assert!(!s.feeds().unwrap()[0].muted);
        assert_eq!(
            s.candidates(Duration::hours(48)).unwrap().len(),
            1,
            "unmuting must put the feed's articles back in the pool"
        );
    }

    #[test]
    fn removing_an_unknown_feed_reports_it() {
        let s = Store::in_memory().unwrap();
        assert_eq!(s.remove_feed("nope").unwrap(), None);
    }

    #[test]
    fn opened_articles_drop_out_of_the_candidate_pool() {
        let (s, _) = store_with_feed();
        assert_eq!(s.candidates(Duration::hours(48)).unwrap().len(), 1);
        let a = s.article("a1").unwrap().unwrap();
        s.record(&a, Event::Opened).unwrap();
        assert!(s.candidates(Duration::hours(48)).unwrap().is_empty());
    }

    #[test]
    fn articles_outside_the_window_are_not_candidates() {
        let mut s = Store::in_memory().unwrap();
        let f = s.add_feed("lwn", "https://lwn.net/feed").unwrap();
        let mut old = incoming("old", "ancient");
        old.published = Some(Utc::now() - Duration::days(9));
        s.ingest(f, &[old]).unwrap();
        assert!(s.candidates(Duration::hours(36)).unwrap().is_empty());
        assert_eq!(s.candidates(Duration::days(30)).unwrap().len(), 1);
    }

    #[test]
    fn muted_feeds_never_reach_the_edition() {
        let (s, feed) = store_with_feed();
        s.set_muted(feed, true).unwrap();
        assert!(s.candidates(Duration::hours(48)).unwrap().is_empty());
        assert!(s.arrivals_by_feed(Duration::hours(48)).unwrap().is_empty());
    }

    #[test]
    fn engagement_starts_neutral() {
        let (s, feed) = store_with_feed();
        let e = s.engagement(feed).unwrap();
        assert_eq!(e.opened, 0);
        assert!((e.finish_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn finishing_articles_lifts_the_finish_rate() {
        let (s, feed) = store_with_feed();
        let a = s.article("a1").unwrap().unwrap();
        s.record(&a, Event::Opened).unwrap();
        s.record(&a, Event::Finished).unwrap();
        let e = s.engagement(feed).unwrap();
        assert_eq!((e.opened, e.finished), (1, 1));
        assert!(e.finish_rate() > 0.5);
    }

    #[test]
    fn repeat_events_on_one_article_are_counted_once() {
        let (s, feed) = store_with_feed();
        let a = s.article("a1").unwrap().unwrap();
        for _ in 0..4 {
            s.record(&a, Event::Opened).unwrap();
        }
        assert_eq!(s.engagement(feed).unwrap().opened, 1);
    }

    #[test]
    fn length_preference_needs_a_real_sample() {
        let mut s = Store::in_memory().unwrap();
        let f = s.add_feed("lwn", "https://lwn.net/feed").unwrap();
        for i in 0..4 {
            let a = incoming(&format!("a{i}"), "t");
            s.ingest(f, &[a]).unwrap();
            let stored = s.article(&format!("a{i}")).unwrap().unwrap();
            s.record(&stored, Event::Finished).unwrap();
        }
        assert_eq!(s.finished_length_mean().unwrap(), None, "4 is not enough");

        let a = incoming("a9", "t");
        s.ingest(f, &[a]).unwrap();
        let stored = s.article("a9").unwrap().unwrap();
        s.record(&stored, Event::Finished).unwrap();
        assert_eq!(s.finished_length_mean().unwrap(), Some(500.0));
    }

    #[test]
    fn arrivals_count_everything_not_just_candidates() {
        let (s, _) = store_with_feed();
        let a = s.article("a1").unwrap().unwrap();
        s.record(&a, Event::Opened).unwrap();
        let arrivals = s.arrivals_by_feed(Duration::hours(48)).unwrap();
        assert_eq!(arrivals.len(), 1);
        assert_eq!(arrivals[0].2, 1, "already-read articles still arrived");
    }

    #[test]
    fn rules_round_trip() {
        let s = Store::in_memory().unwrap();
        let id = s.add_rule("crypto", Field::Title, -1.0).unwrap();
        let rules = s.rules().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].pattern, "crypto");
        assert_eq!(rules[0].field, Field::Title);
        assert!(rules[0].is_veto());
        assert!(s.remove_rule(id).unwrap());
        assert!(s.rules().unwrap().is_empty());
        assert!(
            !s.remove_rule(id).unwrap(),
            "removing twice is not an error"
        );
    }

    #[test]
    fn bodies_can_be_filled_in_later() {
        let (s, _) = store_with_feed();
        assert!(s.article("a1").unwrap().unwrap().body.is_none());
        s.set_body("a1", "the full text", 1234).unwrap();
        let a = s.article("a1").unwrap().unwrap();
        assert_eq!(a.body.as_deref(), Some("the full text"));
        assert_eq!(a.word_count, 1234);
    }

    #[test]
    fn never_boost_persists() {
        let (s, feed) = store_with_feed();
        assert!(!s.feeds().unwrap()[0].never_boost);
        s.set_never_boost(feed, true).unwrap();
        assert!(s.feeds().unwrap()[0].never_boost);
    }
}
