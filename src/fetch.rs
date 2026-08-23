//! Talking to the network.
//!
//! Two jobs: pull a feed and turn it into rows for the store, and pull the
//! article page behind an item so the reading view has real text rather than a
//! truncated summary.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use sha2::{Digest, Sha256};

use crate::extract;
use crate::store::IncomingArticle;

const TIMEOUT: Duration = Duration::from_secs(20);
const MAX_ITEMS_PER_FETCH: usize = 60;
const USER_AGENT: &str = concat!("yomi/", env!("CARGO_PKG_VERSION"));

pub fn client() -> Result<Client> {
    Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .context("could not build an HTTP client")
}

/// A stable id for an article, so re-reading a feed does not duplicate it.
///
/// Prefers the feed's own guid; falls back to the link. Hashed so the id is a
/// fixed width and safe as a primary key whatever the feed put in there.
pub fn article_id(guid: Option<&str>, link: &str) -> String {
    let basis = guid.filter(|g| !g.trim().is_empty()).unwrap_or(link);
    let mut hasher = Sha256::new();
    hasher.update(basis.trim().as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}

/// Fetch and parse one feed.
pub async fn fetch_feed(client: &Client, url: &str) -> Result<Vec<IncomingArticle>> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("could not reach {url}"))?;

    if !response.status().is_success() {
        return Err(anyhow!("{url} returned {}", response.status()));
    }

    let bytes = response.bytes().await?;
    let parsed = feed_rs::parser::parse(bytes.as_ref())
        .with_context(|| format!("{url} is not a valid RSS or Atom feed"))?;

    Ok(parsed
        .entries
        .into_iter()
        .take(MAX_ITEMS_PER_FETCH)
        .map(|entry| {
            let link = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default();

            let title = entry
                .title
                .map(|t| t.content)
                .unwrap_or_else(|| "Untitled".to_string());

            // Some feeds put the whole article in <content>. When they do,
            // that is better than anything we would scrape.
            let content_html = entry.content.and_then(|c| c.body);
            let summary_html = entry.summary.map(|t| t.content);

            let body = content_html
                .as_deref()
                .map(extract::html_to_markdown)
                .filter(|m| !m.trim().is_empty());

            let summary = summary_html
                .as_deref()
                .map(extract::html_to_markdown)
                .or_else(|| body.clone())
                .unwrap_or_default();

            let word_count = body.as_deref().map(extract::word_count).unwrap_or(0);

            IncomingArticle {
                id: article_id(Some(&entry.id), &link),
                title: extract::html_to_markdown(&title),
                author: entry.authors.first().and_then(|p| real_author(&p.name)),
                link,
                summary: first_paragraph(&summary, 400),
                body,
                published: entry
                    .published
                    .or(entry.updated)
                    .map(|d| d.with_timezone(&Utc)),
                word_count,
            }
        })
        .filter(|a| !a.link.is_empty())
        .collect())
}

/// Fetch the article page and reduce it to readable Markdown.
pub async fn fetch_full_text(client: &Client, link: &str) -> Result<String> {
    let response = client
        .get(link)
        .send()
        .await
        .with_context(|| format!("could not reach {link}"))?;
    if !response.status().is_success() {
        return Err(anyhow!("{link} returned {}", response.status()));
    }
    let html = response.text().await?;
    let markdown = extract::article_markdown(&html);
    if markdown.trim().is_empty() {
        return Err(anyhow!("no article text found at {link}"));
    }
    Ok(markdown)
}

/// Drop bylines that are not names.
///
/// Plenty of feeds fill the author field with a placeholder — The Hacker News
/// publishes the literal string "author" — and rendering that as a byline
/// makes the edition look broken when it is the feed that is.
pub fn real_author(name: &str) -> Option<String> {
    let trimmed = name.trim();
    const PLACEHOLDERS: &[&str] = &[
        "author",
        "admin",
        "administrator",
        "editor",
        "unknown",
        "none",
        "n/a",
        "staff",
        "no author",
        "-",
    ];
    if trimmed.is_empty() || PLACEHOLDERS.contains(&trimmed.to_ascii_lowercase().as_str()) {
        return None;
    }
    Some(trimmed.to_string())
}

/// The opening of a summary, cut on a character boundary and at a word.
pub fn first_paragraph(markdown: &str, max_chars: usize) -> String {
    let text = markdown
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with(['#', '>', '-', '`']))
        .unwrap_or_else(|| markdown.lines().next().unwrap_or(""))
        .trim();

    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let cut: String = text.chars().take(max_chars).collect();
    match cut.rsplit_once(' ') {
        Some((head, _)) if head.chars().count() > max_chars / 2 => format!("{head}…"),
        _ => format!("{cut}…"),
    }
}

/// Best-effort title for a URL, used when adding a feed without a name.
pub async fn feed_title(client: &Client, url: &str) -> Result<String> {
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!("{url} returned {}", response.status()));
    }
    let bytes = response.bytes().await?;
    let parsed = feed_rs::parser::parse(bytes.as_ref())
        .with_context(|| format!("{url} is not a valid RSS or Atom feed"))?;
    Ok(parsed
        .title
        .map(|t| extract::html_to_markdown(&t.content))
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| host_of(url)))
}

pub fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or(url)
        .trim_start_matches("www.")
        .to_string()
}

/// A timestamp only for display; kept here so the views do not each invent one.
pub fn age(now: DateTime<Utc>, then: DateTime<Utc>) -> String {
    let minutes = (now - then).num_minutes();
    if minutes < 1 {
        "just now".into()
    } else if minutes < 60 {
        format!("{minutes}m")
    } else if minutes < 60 * 24 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}d", minutes / (60 * 24))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn article_ids_are_stable_for_the_same_guid() {
        let a = article_id(Some("tag:lwn.net,2026:12345"), "https://lwn.net/a");
        let b = article_id(Some("tag:lwn.net,2026:12345"), "https://lwn.net/moved");
        assert_eq!(a, b, "a moved link must not create a duplicate article");
    }

    #[test]
    fn article_ids_fall_back_to_the_link() {
        let a = article_id(None, "https://lwn.net/a");
        let b = article_id(Some("   "), "https://lwn.net/a");
        assert_eq!(a, b);
        assert_ne!(a, article_id(None, "https://lwn.net/b"));
    }

    #[test]
    fn article_ids_are_fixed_width_hex() {
        let id = article_id(None, "https://example.invalid/a");
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn summaries_are_cut_on_character_boundaries() {
        // An em dash straddling the cut used to abort the process.
        let text = "Kubernetes 1.35 ships CNI — and what it breaks for everyone else";
        for n in 1..text.chars().count() + 5 {
            let out = first_paragraph(text, n);
            assert!(out.chars().count() <= n + 1, "n={n} gave {out:?}");
        }
    }

    #[test]
    fn short_summaries_are_left_alone() {
        assert_eq!(first_paragraph("a short one", 400), "a short one");
    }

    #[test]
    fn summaries_prefer_the_first_real_paragraph() {
        let md = "# A heading\n\n> a quote\n\nThe actual opening sentence.";
        assert_eq!(first_paragraph(md, 400), "The actual opening sentence.");
    }

    #[test]
    fn summaries_break_at_a_word_when_they_can() {
        let out = first_paragraph("alpha beta gamma delta epsilon", 20);
        assert!(out.ends_with('…'));
        assert!(!out.contains("delt…"), "should not cut mid-word: {out}");
    }

    #[test]
    fn an_empty_body_gives_an_empty_summary() {
        assert_eq!(first_paragraph("", 400), "");
    }

    #[test]
    fn placeholder_bylines_are_dropped() {
        // The Hacker News really does publish "author" as the author.
        for junk in [
            "author", "Author", " admin ", "unknown", "N/A", "", "   ", "-",
        ] {
            assert_eq!(real_author(junk), None, "{junk:?} is not a byline");
        }
    }

    #[test]
    fn real_bylines_are_kept_and_trimmed() {
        assert_eq!(real_author("Bruce Schneier"), Some("Bruce Schneier".into()));
        assert_eq!(real_author("  Bill Toulas  "), Some("Bill Toulas".into()));
        // A sponsor credit is real information, even if it is not a person.
        assert_eq!(
            real_author("Sponsored by ThreatLocker"),
            Some("Sponsored by ThreatLocker".into())
        );
    }

    #[test]
    fn hosts_are_extracted_for_display() {
        assert_eq!(host_of("https://www.lwn.net/rss"), "lwn.net");
        assert_eq!(
            host_of("https://fasterthanli.me/index.xml"),
            "fasterthanli.me"
        );
        assert_eq!(host_of("not a url"), "not a url");
    }

    #[test]
    fn ages_read_compactly() {
        let now = Utc::now();
        assert_eq!(age(now, now), "just now");
        assert_eq!(age(now, now - ChronoDuration::minutes(20)), "20m");
        assert_eq!(age(now, now - ChronoDuration::hours(6)), "6h");
        assert_eq!(age(now, now - ChronoDuration::days(3)), "3d");
    }
}
