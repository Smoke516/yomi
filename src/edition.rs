//! Building the day's edition.
//!
//! A bounded, ranked set of articles, plus an honest account of what was left
//! out. The held-back list is not a courtesy — a filter you cannot see is a
//! filter you cannot trust, and the whole design rests on the reader believing
//! this one.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

use crate::model::Article;
use crate::rank::{self, Context, FeedSignals, Score};
use crate::store::Store;

/// An article that made the edition, with the reasoning that put it there.
#[derive(Debug, Clone)]
pub struct Placed {
    pub article: Article,
    pub score: Score,
}

/// A feed that had arrivals which did not make it.
#[derive(Debug, Clone, PartialEq)]
pub struct HeldBack {
    pub feed_id: i64,
    pub feed_name: String,
    pub count: i64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct Edition {
    pub built_at: DateTime<Utc>,
    pub picked: Vec<Placed>,
    pub held: Vec<HeldBack>,
    /// Everything that arrived in the window, whether shown or not.
    pub considered: i64,
}

impl Edition {
    pub fn is_empty(&self) -> bool {
        self.picked.is_empty()
    }
}

/// How many held-back feeds to list before it stops being informative.
const HELD_ROWS: usize = 6;

/// Assemble an edition from an already-gathered pool.
///
/// Placement is greedy and re-scores after every pick, because the crowding
/// term depends on what has already been placed. The pool is small enough
/// (a day of feeds) that the quadratic pass is not worth optimising away.
pub fn assemble(
    candidates: Vec<Article>,
    arrivals: Vec<(i64, String, i64)>,
    signals: &HashMap<i64, FeedSignals>,
    ctx: &Context,
    limit: usize,
) -> Edition {
    let considered: i64 = arrivals.iter().map(|(_, _, n)| *n).sum();
    let inbound = inbound_links(&candidates);

    let default_signals = FeedSignals {
        engagement: crate::store::FeedEngagement {
            opened: 0,
            finished: 0,
        },
        never_boost: false,
    };

    // Vetoed articles are removed up front so they can never be picked, but
    // they are counted so the held-back line can say why.
    let mut vetoed_per_feed: HashMap<i64, i64> = HashMap::new();
    let mut pool: Vec<Article> = Vec::new();
    for a in candidates {
        let sig = signals.get(&a.feed_id).unwrap_or(&default_signals);
        let s = rank::score(&a, sig, ctx, 0, 0);
        if s.vetoed {
            *vetoed_per_feed.entry(a.feed_id).or_insert(0) += 1;
        } else {
            pool.push(a);
        }
    }

    let mut picked: Vec<Placed> = Vec::new();
    let mut crowd: HashMap<i64, usize> = HashMap::new();

    while picked.len() < limit && !pool.is_empty() {
        let mut best: Option<(usize, Score)> = None;
        for (i, a) in pool.iter().enumerate() {
            let sig = signals.get(&a.feed_id).unwrap_or(&default_signals);
            let s = rank::score(
                a,
                sig,
                ctx,
                *crowd.get(&a.feed_id).unwrap_or(&0),
                *inbound.get(&a.id).unwrap_or(&0),
            );
            let better = match &best {
                None => true,
                Some((_, bs)) => s.total > bs.total,
            };
            if better {
                best = Some((i, s));
            }
        }
        let Some((idx, score)) = best else { break };
        let article = pool.remove(idx);
        *crowd.entry(article.feed_id).or_insert(0) += 1;
        picked.push(Placed { article, score });
    }

    // -- what was left out ----------------------------------------------
    let mut picked_per_feed: HashMap<i64, i64> = HashMap::new();
    for p in &picked {
        *picked_per_feed.entry(p.article.feed_id).or_insert(0) += 1;
    }

    let mut held: Vec<HeldBack> = arrivals
        .into_iter()
        .filter_map(|(feed_id, feed_name, total)| {
            let shown = *picked_per_feed.get(&feed_id).unwrap_or(&0);
            let count = total - shown;
            if count <= 0 {
                return None;
            }
            let vetoed = *vetoed_per_feed.get(&feed_id).unwrap_or(&0);
            let reason = if vetoed > 0 {
                format!(
                    "{count} {}, {vetoed} stopped by your rules",
                    plural(count, "post")
                )
            } else if shown == 0 {
                format!(
                    "{count} {}, none scored above the fold",
                    plural(count, "post")
                )
            } else {
                format!("{count} more {}", plural(count, "post"))
            };
            Some(HeldBack {
                feed_id,
                feed_name,
                count,
                reason,
            })
        })
        .collect();

    held.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.feed_name.cmp(&b.feed_name))
    });
    held.truncate(HELD_ROWS);

    Edition {
        built_at: ctx.now,
        picked,
        held,
        considered,
    }
}

fn plural(n: i64, word: &str) -> String {
    if n == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

/// How many other articles in the pool point at each article's link.
///
/// Cheap and approximate: a substring search over the text we already have.
/// It exists to surface the thing three of your feeds all wrote about today.
fn inbound_links(pool: &[Article]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let texts: Vec<(String, String)> = pool
        .iter()
        .map(|a| {
            let mut t = a.summary.clone();
            if let Some(b) = &a.body {
                t.push('\n');
                t.push_str(b);
            }
            (a.id.clone(), t)
        })
        .collect();

    for target in pool {
        if target.link.is_empty() {
            continue;
        }
        let n = texts
            .iter()
            .filter(|(id, text)| id != &target.id && text.contains(&target.link))
            .count();
        if n > 0 {
            counts.insert(target.id.clone(), n);
        }
    }
    counts
}

/// Build today's edition from the store.
pub fn build(store: &Store, window: Duration, limit: usize) -> Result<Edition> {
    let candidates = store.candidates(window)?;
    let arrivals = store.arrivals_by_feed(window)?;

    let mut signals = HashMap::new();
    for feed in store.feeds()? {
        signals.insert(
            feed.id,
            FeedSignals {
                engagement: store.engagement(feed.id)?,
                never_boost: feed.never_boost,
            },
        );
    }

    let ctx = Context {
        now: Utc::now(),
        finished_mean: store.finished_length_mean()?,
        rules: store.rules()?,
    };

    Ok(assemble(candidates, arrivals, &signals, &ctx, limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, Rule};
    use crate::store::FeedEngagement;

    fn art(id: &str, feed_id: i64, feed: &str, title: &str, age_h: i64) -> Article {
        Article {
            id: id.into(),
            feed_id,
            feed_name: feed.into(),
            title: title.into(),
            author: None,
            link: format!("https://{feed}/{id}"),
            summary: String::new(),
            body: None,
            published: Some(Utc::now() - Duration::hours(age_h)),
            first_seen: Utc::now(),
            word_count: 600,
        }
    }

    fn ctx() -> Context {
        Context {
            now: Utc::now(),
            finished_mean: None,
            rules: vec![],
        }
    }

    fn no_signals() -> HashMap<i64, FeedSignals> {
        HashMap::new()
    }

    fn signals_for(feed_id: i64, opened: i64, finished: i64) -> HashMap<i64, FeedSignals> {
        let mut m = HashMap::new();
        m.insert(
            feed_id,
            FeedSignals {
                engagement: FeedEngagement { opened, finished },
                never_boost: false,
            },
        );
        m
    }

    #[test]
    fn an_empty_pool_gives_an_empty_edition() {
        let e = assemble(vec![], vec![], &no_signals(), &ctx(), 12);
        assert!(e.is_empty());
        assert_eq!(e.considered, 0);
        assert!(e.held.is_empty());
    }

    #[test]
    fn the_edition_is_bounded_by_the_limit() {
        let pool: Vec<Article> = (0..40)
            .map(|i| art(&format!("a{i}"), (i % 8) as i64, "feed", "t", 3))
            .collect();
        let arrivals = vec![(0, "feed".into(), 40)];
        let e = assemble(pool, arrivals, &no_signals(), &ctx(), 12);
        assert_eq!(e.picked.len(), 12);
    }

    #[test]
    fn one_firehose_feed_cannot_take_every_slot() {
        // 30 posts from one feed, 1 each from three others.
        let mut pool: Vec<Article> = (0..30)
            .map(|i| art(&format!("hose{i}"), 1, "firehose", "noise", 3))
            .collect();
        pool.push(art("q1", 2, "quiet-a", "a good one", 3));
        pool.push(art("q2", 3, "quiet-b", "another", 3));
        pool.push(art("q3", 4, "quiet-c", "a third", 3));

        let e = assemble(pool, vec![], &no_signals(), &ctx(), 8);
        let from_hose = e.picked.iter().filter(|p| p.article.feed_id == 1).count();
        assert!(
            from_hose < 8,
            "firehose took {from_hose} of 8 slots — crowding is not working"
        );
        for feed in [2, 3, 4] {
            assert!(
                e.picked.iter().any(|p| p.article.feed_id == feed),
                "quiet feed {feed} was crowded out entirely"
            );
        }
    }

    #[test]
    fn a_feed_you_finish_wins_the_lead() {
        let pool = vec![
            art("loved", 1, "beloved", "from the feed you read", 6),
            art("meh", 2, "whatever", "from the feed you ignore", 6),
        ];
        let mut signals = signals_for(1, 20, 19);
        signals.insert(
            2,
            FeedSignals {
                engagement: FeedEngagement {
                    opened: 20,
                    finished: 1,
                },
                never_boost: false,
            },
        );
        let e = assemble(pool, vec![], &signals, &ctx(), 12);
        assert_eq!(e.picked[0].article.id, "loved");
    }

    #[test]
    fn every_picked_article_carries_its_reasoning() {
        let pool = vec![art("a", 1, "feed", "t", 2)];
        let e = assemble(pool, vec![], &no_signals(), &ctx(), 12);
        let s = &e.picked[0].score;
        assert!(!s.terms.is_empty());
        let sum: f64 = s.terms.iter().map(|t| t.delta).sum();
        assert!((s.total - sum).abs() < 1e-9);
    }

    #[test]
    fn vetoed_articles_never_appear() {
        let pool = vec![
            art("keep", 1, "feed", "a normal headline", 2),
            art("drop", 1, "feed", "another crypto rug pull", 2),
        ];
        let mut c = ctx();
        c.rules = vec![Rule {
            id: 1,
            pattern: "crypto".into(),
            field: Field::Title,
            weight: -1.0,
        }];
        let e = assemble(pool, vec![(1, "feed".into(), 2)], &no_signals(), &c, 12);
        assert_eq!(e.picked.len(), 1);
        assert_eq!(e.picked[0].article.id, "keep");
    }

    #[test]
    fn the_held_back_line_names_the_rule_that_did_it() {
        let pool = vec![art("drop", 1, "feed", "another crypto rug pull", 2)];
        let mut c = ctx();
        c.rules = vec![Rule {
            id: 1,
            pattern: "crypto".into(),
            field: Field::Title,
            weight: -1.0,
        }];
        let e = assemble(pool, vec![(1, "feed".into(), 1)], &no_signals(), &c, 12);
        assert_eq!(e.held.len(), 1);
        assert_eq!(e.held[0].reason, "1 post, 1 stopped by your rules");
    }

    #[test]
    fn a_feed_with_nothing_shown_says_so() {
        let pool: Vec<Article> = (0..3)
            .map(|i| art(&format!("a{i}"), 1, "loud", "t", 2))
            .collect();
        // Limit of 1 means two are held back from the same feed.
        let e = assemble(pool, vec![(1, "loud".into(), 3)], &no_signals(), &ctx(), 1);
        assert_eq!(e.held[0].reason, "2 more posts");
    }

    #[test]
    fn a_feed_that_lost_every_slot_gets_the_above_the_fold_line() {
        let pool = vec![art("a", 1, "quiet", "t", 2)];
        let arrivals = vec![(1, "quiet".into(), 1), (2, "silent".into(), 4)];
        let e = assemble(pool, arrivals, &no_signals(), &ctx(), 12);
        let silent = e.held.iter().find(|h| h.feed_name == "silent").unwrap();
        assert_eq!(silent.reason, "4 posts, none scored above the fold");
    }

    #[test]
    fn feeds_with_everything_shown_are_not_listed_as_held() {
        let pool = vec![art("a", 1, "quiet", "t", 2)];
        let e = assemble(
            pool,
            vec![(1, "quiet".into(), 1)],
            &no_signals(),
            &ctx(),
            12,
        );
        assert!(e.held.is_empty());
    }

    #[test]
    fn the_held_list_is_capped_and_ordered_by_size() {
        let arrivals: Vec<(i64, String, i64)> = (0..12)
            .map(|i| (i as i64, format!("feed{i}"), (i + 1) as i64))
            .collect();
        let e = assemble(vec![], arrivals, &no_signals(), &ctx(), 12);
        assert_eq!(e.held.len(), HELD_ROWS);
        assert_eq!(e.held[0].count, 12, "largest first");
        assert!(e.held.windows(2).all(|w| w[0].count >= w[1].count));
    }

    #[test]
    fn considered_counts_everything_that_arrived() {
        let arrivals = vec![(1, "a".into(), 41), (2, "b".into(), 128)];
        let e = assemble(vec![], arrivals, &no_signals(), &ctx(), 12);
        assert_eq!(e.considered, 169);
    }

    #[test]
    fn an_article_other_feeds_link_to_is_promoted() {
        let mut linker_a = art("la", 2, "blog-a", "commentary", 3);
        let mut linker_b = art("lb", 3, "blog-b", "more commentary", 3);
        let target = art("t", 1, "source", "the original", 3);
        linker_a.summary = format!("see {} for details", target.link);
        linker_b.summary = format!("as covered at {}", target.link);

        let pool = vec![linker_a, linker_b, target];
        let e = assemble(pool, vec![], &no_signals(), &ctx(), 12);
        assert_eq!(e.picked[0].article.id, "t");
        assert!(e.picked[0]
            .score
            .terms
            .iter()
            .any(|t| t.label.contains("linked this")));
    }

    #[test]
    fn an_article_does_not_link_to_itself() {
        let mut a = art("self", 1, "feed", "t", 3);
        a.summary = format!("my own url is {}", a.link);
        let e = assemble(vec![a], vec![], &no_signals(), &ctx(), 12);
        assert!(!e.picked[0]
            .score
            .terms
            .iter()
            .any(|t| t.label.contains("linked this")));
    }

    #[test]
    fn picks_are_ordered_by_score() {
        let pool: Vec<Article> = (0..6)
            .map(|i| art(&format!("a{i}"), i, "feed", "t", i * 6))
            .collect();
        let e = assemble(pool, vec![], &no_signals(), &ctx(), 6);
        let totals: Vec<f64> = e.picked.iter().map(|p| p.score.total).collect();
        assert!(
            totals.windows(2).all(|w| w[0] >= w[1]),
            "not in descending score order: {totals:?}"
        );
    }

    #[test]
    fn build_runs_against_a_real_store() {
        use crate::store::IncomingArticle;
        let mut store = Store::in_memory().unwrap();
        let f = store.add_feed("lwn", "https://lwn.net/feed").unwrap();
        store
            .ingest(
                f,
                &[IncomingArticle {
                    id: "x1".into(),
                    title: "Something happened".into(),
                    author: None,
                    link: "https://lwn.net/x1".into(),
                    summary: "a summary".into(),
                    body: None,
                    published: Some(Utc::now()),
                    word_count: 400,
                }],
            )
            .unwrap();

        let e = build(&store, Duration::hours(36), 12).unwrap();
        assert_eq!(e.picked.len(), 1);
        assert_eq!(e.considered, 1);
        assert_eq!(e.picked[0].article.title, "Something happened");
    }
}
