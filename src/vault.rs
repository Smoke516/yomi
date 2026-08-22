//! Writing an article into a markdown vault.
//!
//! This is the half of Yomi that only exists because Scribble exists. Reading
//! and note-keeping are the same activity interrupted by a tool boundary; `s`
//! removes the boundary. The file lands in the vault Scribble already edits,
//! in the form Scribble already understands.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::model::Article;

/// Turn an article into a vault note.
pub fn note_for(article: &Article, saved_at: DateTime<Utc>) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("title: {}\n", yaml_scalar(&article.title)));
    out.push_str(&format!("source: {}\n", yaml_scalar(&article.link)));
    out.push_str(&format!("feed: {}\n", yaml_scalar(&article.feed_name)));
    if let Some(author) = &article.author {
        out.push_str(&format!("author: {}\n", yaml_scalar(author)));
    }
    if let Some(published) = article.published {
        out.push_str(&format!("published: {}\n", published.to_rfc3339()));
    }
    out.push_str(&format!("saved: {}\n", saved_at.to_rfc3339()));
    out.push_str("tags: [clipping]\n");
    out.push_str("---\n\n");

    out.push_str(&format!("# {}\n\n", article.title));

    match article.body.as_deref() {
        Some(body) if !body.trim().is_empty() => {
            out.push_str(body.trim());
            out.push('\n');
        }
        _ => {
            let summary = article.summary.trim();
            if summary.is_empty() {
                out.push_str("*No text was available for this article.*\n");
            } else {
                out.push_str(summary);
                out.push_str("\n\n*Only the feed summary was available.*\n");
            }
        }
    }

    out.push_str(&format!("\n[Read the original]({})\n", article.link));
    out
}

/// A filename that is safe on every platform Yomi builds for, and still
/// recognisable in a vault listing.
pub fn filename_for(article: &Article) -> String {
    let mut slug = String::new();
    let mut last_dash = true;
    for c in article.title.chars() {
        // Windows forbids <>:"/\|?* and control characters; a vault is often
        // synced across platforms, so be strict everywhere.
        let ok = c.is_alphanumeric() || c == ' ' || c == '-' || c == '_';
        if ok && !c.is_whitespace() && c != '-' {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
        if slug.chars().count() >= 60 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        format!("yomi-{}.md", &article.id[..article.id.len().min(12)])
    } else {
        format!("{slug}.md")
    }
}

/// Write the note, without ever clobbering something already in the vault.
///
/// Scribble's own rule is that it never overwrites work it did not write, and
/// a reader dropping files into someone's notes has no business being less
/// careful.
pub fn save(vault: &Path, article: &Article, saved_at: DateTime<Utc>) -> Result<PathBuf> {
    std::fs::create_dir_all(vault)
        .with_context(|| format!("could not create vault directory {}", vault.display()))?;

    let base = filename_for(article);
    let path = unique_path(vault, &base);
    std::fs::write(&path, note_for(article, saved_at))
        .with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

fn unique_path(dir: &Path, filename: &str) -> PathBuf {
    let candidate = dir.join(filename);
    if !candidate.exists() {
        return candidate;
    }
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    for n in 2..1000 {
        let next = dir.join(format!("{stem} {n}.md"));
        if !next.exists() {
            return next;
        }
    }
    dir.join(format!("{stem} {}.md", Utc::now().timestamp()))
}

/// Quote a YAML scalar only when it needs it, so the frontmatter stays
/// readable in an editor.
fn yaml_scalar(s: &str) -> String {
    let needs_quotes = s.is_empty()
        || s.starts_with([
            '&', '*', '!', '|', '>', '%', '@', '`', '"', '\'', '[', '{', '#', '-', '?',
        ])
        || s.contains(": ")
        || s.ends_with(':')
        || s.contains('\n');
    if needs_quotes {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article() -> Article {
        Article {
            id: "abc123def456".into(),
            feed_id: 1,
            feed_name: "fasterthanli.me".into(),
            title: "A history of the borrow checker".into(),
            author: Some("Amos Wenger".into()),
            link: "https://fasterthanli.me/borrow".into(),
            summary: "a summary".into(),
            body: Some("The borrow checker is not a theorem prover.".into()),
            published: Some(
                DateTime::parse_from_rfc3339("2026-08-22T09:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            first_seen: Utc::now(),
            word_count: 4400,
        }
    }

    fn tmpdir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("yomi-vault-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_note_opens_with_frontmatter() {
        let note = note_for(&article(), Utc::now());
        assert!(note.starts_with("---\n"));
        assert!(note.contains("title: A history of the borrow checker\n"));
        assert!(note.contains("source: https://fasterthanli.me/borrow\n"));
        assert!(note.contains("author: Amos Wenger\n"));
        assert!(note.contains("tags: [clipping]\n"));
    }

    #[test]
    fn the_body_is_the_stored_markdown() {
        let note = note_for(&article(), Utc::now());
        assert!(note.contains("The borrow checker is not a theorem prover."));
        assert!(note.contains("[Read the original](https://fasterthanli.me/borrow)"));
    }

    #[test]
    fn a_summary_only_article_says_so() {
        let mut a = article();
        a.body = None;
        let note = note_for(&a, Utc::now());
        assert!(note.contains("a summary"));
        assert!(note.contains("*Only the feed summary was available.*"));
    }

    #[test]
    fn an_empty_article_still_produces_a_valid_note() {
        let mut a = article();
        a.body = None;
        a.summary = "   ".into();
        let note = note_for(&a, Utc::now());
        assert!(note.contains("*No text was available for this article.*"));
        assert_eq!(note.matches("---\n").count(), 2, "frontmatter must close");
    }

    #[test]
    fn titles_with_yaml_metacharacters_are_quoted() {
        let mut a = article();
        a.title = "Rust: what changed".into();
        let note = note_for(&a, Utc::now());
        assert!(note.contains("title: \"Rust: what changed\""), "{note}");
    }

    #[test]
    fn a_title_with_a_quote_is_escaped_not_broken() {
        let mut a = article();
        a.title = "The \"safe\" subset: a review".into();
        let note = note_for(&a, Utc::now());
        assert!(
            note.contains(r#"title: "The \"safe\" subset: a review""#),
            "{note}"
        );
    }

    #[test]
    fn a_plain_title_is_left_unquoted() {
        let note = note_for(&article(), Utc::now());
        assert!(note.contains("title: A history of the borrow checker\n"));
    }

    #[test]
    fn filenames_are_slugged() {
        assert_eq!(
            filename_for(&article()),
            "A-history-of-the-borrow-checker.md"
        );
    }

    #[test]
    fn filenames_strip_characters_that_break_on_windows() {
        let mut a = article();
        a.title = "What is a *slash* / backslash \\ colon: pipe|?".into();
        let name = filename_for(&a);
        assert!(
            !name.contains(['/', '\\', ':', '*', '?', '|', '<', '>', '"']),
            "unsafe filename: {name}"
        );
        assert!(name.ends_with(".md"));
    }

    #[test]
    fn non_ascii_titles_keep_their_characters() {
        let mut a = article();
        a.title = "読みについて — a note".into();
        let name = filename_for(&a);
        assert!(name.starts_with("読みについて"), "{name}");
        assert!(name.ends_with(".md"));
    }

    #[test]
    fn a_title_of_only_punctuation_still_gets_a_filename() {
        let mut a = article();
        a.title = "!!! ???".into();
        assert_eq!(filename_for(&a), "yomi-abc123def456.md");
    }

    #[test]
    fn filenames_are_bounded_in_length() {
        let mut a = article();
        a.title = "word ".repeat(80);
        assert!(filename_for(&a).chars().count() <= 64);
    }

    #[test]
    fn saving_writes_the_file() {
        let dir = tmpdir("write");
        let path = save(&dir, &article(), Utc::now()).unwrap();
        assert!(path.exists());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# A history of the borrow checker"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn saving_twice_never_clobbers_the_first_note() {
        let dir = tmpdir("clobber");
        let a = article();
        let first = save(&dir, &a, Utc::now()).unwrap();
        std::fs::write(&first, "notes I typed myself").unwrap();

        let second = save(&dir, &a, Utc::now()).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            std::fs::read_to_string(&first).unwrap(),
            "notes I typed myself",
            "an existing note must survive a second save"
        );
        assert!(second.to_string_lossy().contains(" 2.md"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn saving_creates_the_vault_directory() {
        let dir = tmpdir("mkdir").join("nested").join("clippings");
        assert!(!dir.exists());
        save(&dir, &article(), Utc::now()).unwrap();
        assert!(dir.exists());
        std::fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).unwrap();
    }
}
