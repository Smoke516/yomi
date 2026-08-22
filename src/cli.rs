//! The command line.
//!
//! Everything the TUI can show, the shell can too. A reader that only exists
//! as a full-screen app is a reader you cannot pipe, cron, or grep.

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::edition;
use crate::fetch;
use crate::model::Field;
use crate::store::Store;

#[derive(Parser, Debug)]
#[command(
    name = "yomi",
    version,
    about = "A daily edition for your feeds — ranked on your machine, with the reason shown."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open the edition (the default)
    #[command(alias = "tui")]
    Read,

    /// Print today's edition
    Today {
        /// Machine-readable output
        #[arg(long)]
        json: bool,
    },

    /// Explain why an article is in today's edition
    Why {
        /// Its position in `yomi today`, starting at 1
        position: usize,
    },

    /// Subscribe to a feed
    Add {
        url: String,
        /// A name for it; taken from the feed when omitted
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Unsubscribe, by name or by index in `yomi list`
    Remove { target: String },

    /// Show subscribed feeds
    List,

    /// Fetch every feed
    Refresh,

    /// Where Yomi keeps things, and how much it is holding
    Status,

    /// Write a config file with the current settings, to edit by hand
    Config,

    /// Keyword rules that promote or suppress articles
    Rule {
        #[command(subcommand)]
        action: RuleAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum RuleAction {
    /// Show your rules
    List,
    /// Promote matching articles
    Boost {
        pattern: String,
        /// How much, 0.0 to 1.0
        #[arg(short, long, default_value = "0.3")]
        weight: f64,
    },
    /// Push matching articles down
    Demote {
        pattern: String,
        #[arg(short, long, default_value = "0.3")]
        weight: f64,
    },
    /// Keep matching articles out of the edition entirely
    Hide { pattern: String },
    /// Delete a rule by its id in `yomi rule list`
    Remove { id: i64 },
}

pub async fn add(store: &Store, url: &str, name: Option<String>) -> Result<()> {
    let client = fetch::client()?;
    let name = match name {
        Some(n) => n,
        None => fetch::feed_title(&client, url).await?,
    };
    if store.feed_id_by_url(url)?.is_some() {
        println!("already subscribed to {url}");
        return Ok(());
    }
    store.add_feed(&name, url)?;
    println!("added {name}");
    println!("  {url}");
    println!("\nrun `yomi refresh` to pull it in.");
    Ok(())
}

pub fn remove(store: &Store, target: &str) -> Result<()> {
    match store.remove_feed(target)? {
        Some(name) => println!("removed {name}"),
        None => println!("no feed called {target:?} — try `yomi list`"),
    }
    Ok(())
}

pub fn list(store: &Store) -> Result<()> {
    let feeds = store.feeds()?;
    if feeds.is_empty() {
        println!("no feeds yet. add one:");
        println!("  yomi add https://lwn.net/headlines/newrss");
        return Ok(());
    }
    for (i, feed) in feeds.iter().enumerate() {
        let flag = if feed.muted { " (muted)" } else { "" };
        println!(
            "{i:<3} {:<32} {}{}",
            crate::util::truncate_string(&feed.name, 32),
            feed.url,
            flag
        );
    }
    Ok(())
}

pub async fn refresh(store: &mut Store) -> Result<()> {
    let feeds = store.feeds()?;
    if feeds.is_empty() {
        println!("no feeds to refresh");
        return Ok(());
    }
    let client = fetch::client()?;
    let mut added = 0usize;
    let mut failed = 0usize;

    let results = futures::future::join_all(
        feeds
            .iter()
            .map(|f| {
                let client = client.clone();
                let url = f.url.clone();
                async move { (f.id, f.name.clone(), fetch::fetch_feed(&client, &url).await) }
            })
            .collect::<Vec<_>>(),
    )
    .await;

    for (feed_id, name, result) in results {
        match result {
            Ok(items) => {
                let n = store.ingest(feed_id, &items)?;
                store.mark_fetched(feed_id)?;
                added += n;
                println!("{name}: {n} new");
            }
            Err(e) => {
                failed += 1;
                eprintln!("{name}: {e}");
            }
        }
    }
    println!(
        "\n{added} new article(s){}",
        if failed > 0 {
            format!(", {failed} feed(s) failed")
        } else {
            String::new()
        }
    );
    Ok(())
}

pub fn today(store: &Store, config: &Config, json: bool) -> Result<()> {
    let ed = edition::build(store, config.window(), config.edition.size)?;

    if json {
        println!("{}", edition_json(&ed));
        return Ok(());
    }

    if ed.is_empty() {
        println!("nothing in today's edition.");
        if ed.considered > 0 {
            println!("{} arrived but none scored high enough.", ed.considered);
        } else {
            println!("run `yomi refresh` to pull your feeds.");
        }
        return Ok(());
    }

    println!("{} picked from {}\n", ed.picked.len(), ed.considered);
    let now = Utc::now();
    for (i, p) in ed.picked.iter().enumerate() {
        let a = &p.article;
        println!("{:>2}. {}", i + 1, a.title);
        let mut meta = vec![a.feed_name.clone(), fetch::age(now, a.appeared())];
        if a.minutes() > 0 {
            meta.push(format!("{} min", a.minutes()));
        }
        println!("    {}", meta.join(" · "));
        if let Some(reason) = p.score.headline_reason() {
            println!("    {reason}");
        }
        println!("    {}", a.link);
        println!();
    }

    if !ed.held.is_empty() {
        println!("held back");
        for h in &ed.held {
            println!(
                "  {:<20} {}",
                crate::util::truncate_string(&h.feed_name, 20),
                h.reason
            );
        }
    }
    Ok(())
}

pub fn why(store: &Store, config: &Config, position: usize) -> Result<()> {
    let ed = edition::build(store, config.window(), config.edition.size)?;
    if position == 0 || position > ed.picked.len() {
        return Err(anyhow!(
            "there is no article {position} — today's edition has {}",
            ed.picked.len()
        ));
    }
    let p = &ed.picked[position - 1];
    println!("{}\n", p.article.title);
    for term in &p.score.terms {
        let delta = if term.delta > 0.0 {
            format!("+ {:.2}", term.delta)
        } else if term.delta < 0.0 {
            format!("- {:.2}", term.delta.abs())
        } else {
            "  0.00".into()
        };
        println!("  {:<50}{:>7}", term.label, delta);
    }
    println!("  {:<50}{:>7}", "", "──────");
    println!("  {:<50}{:>7.2}", "", p.score.total);
    println!("\nevery number came from your own reading history.");
    Ok(())
}

pub fn rule(store: &Store, action: &RuleAction) -> Result<()> {
    match action {
        RuleAction::List => {
            let rules = store.rules()?;
            if rules.is_empty() {
                println!("no rules. try:");
                println!("  yomi rule hide \"crypto\"");
                println!("  yomi rule boost \"rust\" --weight 0.4");
                return Ok(());
            }
            for r in rules {
                let what = if r.is_veto() {
                    "hidden".to_string()
                } else if r.weight > 0.0 {
                    format!("+{:.2}", r.weight)
                } else {
                    format!("{:.2}", r.weight)
                };
                println!(
                    "{:<4} {:<10} {:<28} {}",
                    r.id,
                    r.field.as_str(),
                    r.pattern,
                    what
                );
            }
        }
        RuleAction::Boost { pattern, weight } => {
            let w = weight.clamp(0.0, 1.0);
            store.add_rule(pattern, Field::Any, w)?;
            println!("articles matching {pattern:?} now score +{w:.2}");
        }
        RuleAction::Demote { pattern, weight } => {
            // A demotion must never silently become a veto.
            let w = -weight.clamp(0.0, 0.9);
            store.add_rule(pattern, Field::Any, w)?;
            println!("articles matching {pattern:?} now score {w:.2}");
        }
        RuleAction::Hide { pattern } => {
            store.add_rule(pattern, Field::Any, -1.0)?;
            println!("articles matching {pattern:?} will be kept out of the edition");
        }
        RuleAction::Remove { id } => {
            if store.remove_rule(*id)? {
                println!("removed rule {id}");
            } else {
                println!("no rule {id} — try `yomi rule list`");
            }
        }
    }
    Ok(())
}

pub fn status(store: &Store, config: &Config) -> Result<()> {
    let feeds = store.feeds()?;
    let muted = feeds.iter().filter(|f| f.muted).count();

    println!("store    {}", crate::config::store_path()?.display());
    println!("config   {}", crate::config::config_path()?.display());
    match config.vault_dir() {
        Some(v) => println!("vault    {}", v.display()),
        None => println!("vault    not set — `s` will tell you how"),
    }
    println!();
    println!(
        "{} feed(s){}, {} article(s) kept",
        feeds.len(),
        if muted > 0 {
            format!(" ({muted} muted)")
        } else {
            String::new()
        },
        store.article_count()?
    );
    println!(
        "edition of {} over the last {}h",
        config.edition.size, config.edition.window_hours
    );

    let mut saved: Vec<(String, i64)> = Vec::new();
    for feed in &feeds {
        let n = store.saves_by_feed(feed.id)?;
        if n > 0 {
            saved.push((feed.name.clone(), n));
        }
    }
    if !saved.is_empty() {
        saved.sort_by(|a, b| b.1.cmp(&a.1));
        println!("\nsaved to the vault");
        for (name, n) in saved.iter().take(5) {
            println!("  {:<28} {n}", crate::util::truncate_string(name, 28));
        }
    }
    Ok(())
}

pub fn write_config(config: &Config) -> Result<()> {
    let path = crate::config::config_path()?;
    if path.exists() {
        println!("{} already exists — edit it in place", path.display());
        return Ok(());
    }
    let written = config.save()?;
    println!("wrote {}", written.display());
    println!("\nset  [vault] path  to send `s` into your notes vault.");
    Ok(())
}

fn edition_json(ed: &edition::Edition) -> String {
    let now = Utc::now();
    let items: Vec<String> = ed
        .picked
        .iter()
        .map(|p| {
            let a = &p.article;
            let reasons: Vec<String> = p
                .score
                .terms
                .iter()
                .map(|t| {
                    format!(
                        "{{\"label\":{},\"delta\":{:.4}}}",
                        json_string(&t.label),
                        t.delta
                    )
                })
                .collect();
            format!(
                "{{\"title\":{},\"feed\":{},\"link\":{},\"age\":{},\"minutes\":{},\"score\":{:.4},\"why\":[{}]}}",
                json_string(&a.title),
                json_string(&a.feed_name),
                json_string(&a.link),
                json_string(&fetch::age(now, a.appeared())),
                a.minutes(),
                p.score.total,
                reasons.join(",")
            )
        })
        .collect();

    let held: Vec<String> = ed
        .held
        .iter()
        .map(|h| {
            format!(
                "{{\"feed\":{},\"count\":{},\"reason\":{}}}",
                json_string(&h.feed_name),
                h.count,
                json_string(&h.reason)
            )
        })
        .collect();

    format!(
        "{{\"built_at\":{},\"considered\":{},\"picked\":[{}],\"held\":[{}]}}",
        json_string(&ed.built_at.to_rfc3339()),
        ed.considered,
        items.join(","),
        held.join(",")
    )
}

/// Minimal JSON string escaping — enough to be correct, no dependency.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_strings_escape_what_they_must() {
        assert_eq!(json_string("plain"), "\"plain\"");
        assert_eq!(json_string("a \"quote\""), "\"a \\\"quote\\\"\"");
        assert_eq!(json_string("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(json_string("two\nlines"), "\"two\\nlines\"");
        assert_eq!(json_string("tab\there"), "\"tab\\there\"");
    }

    #[test]
    fn json_strings_escape_control_characters() {
        assert_eq!(json_string("\u{1}"), "\"\\u0001\"");
    }

    #[test]
    fn json_strings_leave_unicode_alone() {
        assert_eq!(json_string("読み — ok"), "\"読み — ok\"");
    }

    #[test]
    fn an_empty_edition_is_still_valid_json() {
        let ed = edition::Edition {
            built_at: Utc::now(),
            picked: vec![],
            held: vec![],
            considered: 0,
        };
        let json = edition_json(&ed);
        assert!(json.starts_with("{\"built_at\":\""));
        assert!(json.ends_with("\"considered\":0,\"picked\":[],\"held\":[]}"));
    }

    #[test]
    fn demote_can_never_become_a_veto() {
        // A user typing --weight 5 wants a strong demotion, not silent
        // deletion; only `hide` removes things.
        let store = Store::in_memory().unwrap();
        rule(
            &store,
            &RuleAction::Demote {
                pattern: "sport".into(),
                weight: 5.0,
            },
        )
        .unwrap();
        let r = &store.rules().unwrap()[0];
        assert!(!r.is_veto(), "weight was {}", r.weight);
        assert!(r.weight < 0.0);
    }

    #[test]
    fn hide_is_a_veto() {
        let store = Store::in_memory().unwrap();
        rule(
            &store,
            &RuleAction::Hide {
                pattern: "crypto".into(),
            },
        )
        .unwrap();
        assert!(store.rules().unwrap()[0].is_veto());
    }

    #[test]
    fn boost_is_clamped_to_something_sane() {
        let store = Store::in_memory().unwrap();
        rule(
            &store,
            &RuleAction::Boost {
                pattern: "rust".into(),
                weight: 99.0,
            },
        )
        .unwrap();
        assert_eq!(store.rules().unwrap()[0].weight, 1.0);
    }

    #[test]
    fn why_rejects_a_position_that_is_not_there() {
        let store = Store::in_memory().unwrap();
        let config = Config::default();
        assert!(why(&store, &config, 0).is_err());
        assert!(why(&store, &config, 99).is_err());
    }
}
