//! Core types shared across the store, the ranker and the views.

use chrono::{DateTime, Utc};

/// A subscribed feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    pub id: i64,
    pub name: String,
    pub url: String,
    /// Set by the user to keep a feed subscribed but out of the edition.
    pub muted: bool,
    /// Set from the why-panel: stop letting engagement push this feed up.
    pub never_boost: bool,
}

/// One item from a feed, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub id: String,
    pub feed_id: i64,
    pub feed_name: String,
    pub title: String,
    pub author: Option<String>,
    pub link: String,
    /// The summary the feed gave us. Often truncated.
    pub summary: String,
    /// Full text, extracted from the article page. `None` until fetched.
    pub body: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub first_seen: DateTime<Utc>,
    pub word_count: i64,
}

impl Article {
    /// Reading time in minutes, never zero for a non-empty article.
    pub fn minutes(&self) -> i64 {
        if self.word_count == 0 {
            return 0;
        }
        (self.word_count / 220).max(1)
    }

    /// When this article should be treated as having appeared.
    pub fn appeared(&self) -> DateTime<Utc> {
        self.published.unwrap_or(self.first_seen)
    }
}

/// Something the reader did with an article. The ranker learns from these,
/// and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Opened in the reading view.
    Opened,
    /// Scrolled to the end of the reading view.
    Finished,
    /// Handed off to the browser.
    Browser,
    /// Written into the vault.
    Saved,
}

impl Event {
    pub fn as_str(self) -> &'static str {
        match self {
            Event::Opened => "opened",
            Event::Finished => "finished",
            Event::Browser => "browser",
            Event::Saved => "saved",
        }
    }
}

/// Which part of an article a rule looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Summary,
    Any,
}

impl Field {
    pub fn as_str(self) -> &'static str {
        match self {
            Field::Title => "title",
            Field::Summary => "summary",
            Field::Any => "any",
        }
    }

    pub fn parse(s: &str) -> Option<Field> {
        match s {
            "title" => Some(Field::Title),
            "summary" => Some(Field::Summary),
            "any" => Some(Field::Any),
            _ => None,
        }
    }
}

/// A user-written scoring rule. Positive weights promote, negative suppress.
/// A rule at or below -1.0 removes an article from the edition entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: i64,
    pub pattern: String,
    pub field: Field,
    pub weight: f64,
}

impl Rule {
    /// Case-insensitive substring match. Deliberately not a regex: a rule you
    /// cannot read at a glance is a rule you cannot trust.
    pub fn matches(&self, article: &Article) -> bool {
        let needle = self.pattern.to_lowercase();
        if needle.is_empty() {
            return false;
        }
        let hit = |s: &str| s.to_lowercase().contains(&needle);
        match self.field {
            Field::Title => hit(&article.title),
            Field::Summary => hit(&article.summary),
            Field::Any => hit(&article.title) || hit(&article.summary),
        }
    }

    /// A rule this negative suppresses outright rather than just demoting.
    pub fn is_veto(&self) -> bool {
        self.weight <= -1.0
    }
}

/// A structural piece of an article body, ready to be laid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(String),
    Para(String),
    Quote(Vec<String>),
    Code(Vec<String>),
    List(Vec<String>),
    Rule,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article(title: &str, summary: &str) -> Article {
        Article {
            id: "x".into(),
            feed_id: 1,
            feed_name: "f".into(),
            title: title.into(),
            author: None,
            link: "https://example.invalid/a".into(),
            summary: summary.into(),
            body: None,
            published: None,
            first_seen: Utc::now(),
            word_count: 0,
        }
    }

    fn rule(pattern: &str, field: Field, weight: f64) -> Rule {
        Rule {
            id: 1,
            pattern: pattern.into(),
            field,
            weight,
        }
    }

    #[test]
    fn minutes_rounds_up_for_short_articles() {
        let mut a = article("t", "s");
        a.word_count = 30;
        assert_eq!(a.minutes(), 1);
        a.word_count = 4400;
        assert_eq!(a.minutes(), 20);
    }

    #[test]
    fn empty_article_has_no_reading_time() {
        assert_eq!(article("t", "s").minutes(), 0);
    }

    #[test]
    fn rules_match_case_insensitively() {
        let a = article("Rust 1.94 Released", "a release post");
        assert!(rule("rust", Field::Title, 0.2).matches(&a));
        assert!(rule("RUST", Field::Title, 0.2).matches(&a));
    }

    #[test]
    fn rules_respect_their_field() {
        let a = article("Rust 1.94 Released", "a release post");
        assert!(!rule("release post", Field::Title, 0.2).matches(&a));
        assert!(rule("release post", Field::Summary, 0.2).matches(&a));
        assert!(rule("release post", Field::Any, 0.2).matches(&a));
    }

    #[test]
    fn an_empty_pattern_matches_nothing() {
        let a = article("anything", "at all");
        assert!(!rule("", Field::Any, 0.2).matches(&a));
    }

    #[test]
    fn strongly_negative_rules_are_vetoes() {
        assert!(rule("crypto", Field::Any, -1.0).is_veto());
        assert!(rule("crypto", Field::Any, -2.5).is_veto());
        assert!(!rule("crypto", Field::Any, -0.4).is_veto());
    }

    #[test]
    fn appeared_prefers_published_over_first_seen() {
        let mut a = article("t", "s");
        let pub_at = Utc::now() - chrono::Duration::hours(9);
        a.published = Some(pub_at);
        assert_eq!(a.appeared(), pub_at);
        a.published = None;
        assert_eq!(a.appeared(), a.first_seen);
    }

    #[test]
    fn event_names_are_distinct_and_stable() {
        let names: Vec<&str> = [Event::Opened, Event::Finished, Event::Browser, Event::Saved]
            .iter()
            .map(|e| e.as_str())
            .collect();
        assert_eq!(names, vec!["opened", "finished", "browser", "saved"]);
    }

    #[test]
    fn field_names_round_trip() {
        for f in [Field::Title, Field::Summary, Field::Any] {
            assert_eq!(Field::parse(f.as_str()), Some(f));
        }
        assert_eq!(Field::parse("nonsense"), None);
    }
}
