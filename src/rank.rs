//! Scoring, and the explanation of it.
//!
//! Every term here is additive, bounded, and carries the sentence that will be
//! shown to the reader in the why-panel. That constraint is the design: a
//! ranking you cannot interrogate is one you end up resenting, and the whole
//! argument for doing this in a terminal is that the answer is just text.
//!
//! Deliberately not a model. Five or six readable numbers, all derived from
//! this reader's own history, none of it leaving the machine.

use chrono::{DateTime, Utc};

use crate::model::{Article, Rule};
use crate::store::FeedEngagement;

/// Where every article starts before anything is known about it.
pub const BASE: f64 = 0.50;

/// One line of the why-panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub label: String,
    pub delta: f64,
}

/// A score, together with the reason for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub total: f64,
    pub terms: Vec<Term>,
    /// A rule suppressed this outright; it should not appear at all.
    pub vetoed: bool,
    /// The rule that vetoed it, for the held-back explanation.
    pub veto_reason: Option<String>,
}

impl Score {
    /// The single strongest positive reason, for the one-line note the edition
    /// shows under a headline. Only reasons worth a reader's attention.
    pub fn headline_reason(&self) -> Option<&str> {
        self.terms
            .iter()
            // terms[0] is always the base; "it exists" is not a reason.
            .skip(1)
            .filter(|t| t.delta >= 0.08)
            .max_by(|a, b| {
                a.delta
                    .partial_cmp(&b.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|t| t.label.as_str())
    }
}

/// What we know about the feed an article came from.
#[derive(Debug, Clone)]
pub struct FeedSignals {
    pub engagement: FeedEngagement,
    /// The reader pressed `d` in the why-panel: stop boosting this feed.
    pub never_boost: bool,
}

/// Everything shared across one scoring pass.
#[derive(Debug, Clone)]
pub struct Context {
    pub now: DateTime<Utc>,
    /// Mean length of articles this reader finishes. `None` until there is
    /// enough history to be worth acting on.
    pub finished_mean: Option<f64>,
    pub rules: Vec<Rule>,
}

/// Score one article.
///
/// `crowd_index` is how many articles from the same feed have already been
/// placed in this edition, and `inbound_links` how many other articles in
/// today's pool point at this one.
pub fn score(
    article: &Article,
    feed: &FeedSignals,
    ctx: &Context,
    crowd_index: usize,
    inbound_links: usize,
) -> Score {
    let mut terms = vec![Term {
        label: format!("{}, base", article.feed_name),
        delta: BASE,
    }];

    // -- engagement ----------------------------------------------------
    // Needs a real sample before it says anything. Three opens is not much,
    // but it is enough to stop one lucky click from dominating.
    let e = feed.engagement;
    if e.opened >= 3 && !feed.never_boost {
        let delta = clamp((e.finish_rate() - 0.5) * 0.7, -0.25, 0.35);
        if delta.abs() >= 0.01 {
            terms.push(Term {
                label: format!("you finish {} of {} from this feed", e.finished, e.opened),
                delta,
            });
        }
    } else if feed.never_boost && e.opened >= 3 {
        terms.push(Term {
            label: "you asked not to rank this feed up".into(),
            delta: 0.0,
        });
    }

    // -- length preference ---------------------------------------------
    if let (Some(mean), true) = (ctx.finished_mean, article.word_count > 0) {
        if mean > 0.0 {
            let wc = article.word_count as f64;
            let ratio = wc.min(mean) / wc.max(mean);
            let delta = clamp((ratio - 0.5) * 0.24, -0.12, 0.12);
            let label = if wc >= mean {
                format!(
                    "{} min — you read long pieces to the end",
                    article.minutes()
                )
            } else {
                format!(
                    "{} min — shorter than you usually finish",
                    article.minutes()
                )
            };
            terms.push(Term { label, delta });
        }
    }

    // -- recency -------------------------------------------------------
    let hours = (ctx.now - article.appeared()).num_minutes() as f64 / 60.0;
    let delta = if hours <= 0.0 {
        0.08
    } else {
        clamp(0.08 * (1.0 - hours / 48.0), 0.0, 0.08)
    };
    if delta >= 0.005 {
        terms.push(Term {
            label: format!("posted {}", human_age(hours)),
            delta,
        });
    }

    // -- crowding ------------------------------------------------------
    // A feed that posts forty times a day should not get forty slots. The
    // first item is free; each subsequent one costs more.
    if crowd_index > 0 {
        terms.push(Term {
            label: format!("{} already in today's edition from this feed", crowd_index),
            delta: -(0.18 * crowd_index as f64).min(0.60),
        });
    } else {
        terms.push(Term {
            label: "1 post from this feed today, no crowding".into(),
            delta: 0.0,
        });
    }

    // -- other feeds pointing at it -------------------------------------
    if inbound_links > 0 {
        terms.push(Term {
            label: format!(
                "{} other {} you follow linked this",
                inbound_links,
                if inbound_links == 1 {
                    "article"
                } else {
                    "articles"
                }
            ),
            delta: (0.06 * inbound_links as f64).min(0.18),
        });
    }

    // -- the reader's own rules -----------------------------------------
    let mut vetoed = false;
    let mut veto_reason = None;
    for rule in &ctx.rules {
        if !rule.matches(article) {
            continue;
        }
        if rule.is_veto() {
            vetoed = true;
            veto_reason = Some(format!("your rule \"{}\"", rule.pattern));
            terms.push(Term {
                label: format!("your rule \"{}\" hides this", rule.pattern),
                delta: 0.0,
            });
            continue;
        }
        terms.push(Term {
            label: format!("matches your rule \"{}\"", rule.pattern),
            delta: rule.weight,
        });
    }

    let total = terms.iter().map(|t| t.delta).sum::<f64>().max(0.0);
    Score {
        total,
        terms,
        vetoed,
        veto_reason,
    }
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

fn human_age(hours: f64) -> String {
    if hours < 1.0 {
        "just now".to_string()
    } else if hours < 24.0 {
        format!("{}h ago", hours.round() as i64)
    } else {
        let days = (hours / 24.0).round() as i64;
        if days <= 1 {
            "1d ago".to_string()
        } else {
            format!("{days}d ago")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Field;
    use chrono::Duration;

    fn article(wc: i64, age_hours: i64) -> Article {
        let now = Utc::now();
        Article {
            id: "a".into(),
            feed_id: 1,
            feed_name: "lwn.net".into(),
            title: "Kubernetes 1.35 ships CNI".into(),
            author: None,
            link: "https://lwn.net/a".into(),
            summary: "a summary about containers".into(),
            body: None,
            published: Some(now - Duration::hours(age_hours)),
            first_seen: now,
            word_count: wc,
        }
    }

    fn ctx() -> Context {
        Context {
            now: Utc::now(),
            finished_mean: None,
            rules: vec![],
        }
    }

    fn feed(opened: i64, finished: i64) -> FeedSignals {
        FeedSignals {
            engagement: FeedEngagement { opened, finished },
            never_boost: false,
        }
    }

    fn label_containing<'a>(s: &'a Score, needle: &str) -> Option<&'a Term> {
        s.terms.iter().find(|t| t.label.contains(needle))
    }

    #[test]
    fn a_brand_new_feed_scores_near_base() {
        let s = score(&article(0, 30), &feed(0, 0), &ctx(), 0, 0);
        // base + a little recency, nothing else.
        assert!(s.total > BASE && s.total < BASE + 0.09, "was {}", s.total);
        assert!(label_containing(&s, "you finish").is_none());
    }

    #[test]
    fn engagement_needs_three_opens_before_it_speaks() {
        let quiet = score(&article(0, 4), &feed(2, 2), &ctx(), 0, 0);
        assert!(label_containing(&quiet, "you finish").is_none());
        let loud = score(&article(0, 4), &feed(3, 3), &ctx(), 0, 0);
        assert!(label_containing(&loud, "you finish").is_some());
    }

    #[test]
    fn a_feed_you_finish_is_promoted_and_says_so() {
        let s = score(&article(0, 4), &feed(10, 9), &ctx(), 0, 0);
        let t = label_containing(&s, "you finish").expect("term missing");
        assert_eq!(t.label, "you finish 9 of 10 from this feed");
        assert!(t.delta > 0.0);
    }

    #[test]
    fn a_feed_you_abandon_is_demoted() {
        let s = score(&article(0, 4), &feed(12, 1), &ctx(), 0, 0);
        let t = label_containing(&s, "you finish").expect("term missing");
        assert!(t.delta < 0.0, "delta was {}", t.delta);
    }

    #[test]
    fn never_boost_silences_engagement_without_hiding_the_fact() {
        let mut f = feed(10, 9);
        f.never_boost = true;
        let s = score(&article(0, 4), &f, &ctx(), 0, 0);
        assert!(label_containing(&s, "you finish").is_none());
        let t = label_containing(&s, "asked not to rank").expect("term missing");
        assert_eq!(t.delta, 0.0);
    }

    #[test]
    fn crowding_costs_more_for_each_extra_item_from_one_feed() {
        let first = score(&article(0, 4), &feed(0, 0), &ctx(), 0, 0);
        let second = score(&article(0, 4), &feed(0, 0), &ctx(), 1, 0);
        let third = score(&article(0, 4), &feed(0, 0), &ctx(), 2, 0);
        assert!(first.total > second.total);
        assert!(second.total > third.total);
        assert_eq!(
            label_containing(&first, "no crowding").map(|t| t.delta),
            Some(0.0)
        );
    }

    #[test]
    fn crowding_penalty_is_bounded() {
        let s = score(&article(0, 4), &feed(0, 0), &ctx(), 40, 0);
        let t = label_containing(&s, "already in today's").expect("term missing");
        assert!(t.delta >= -0.60, "penalty ran away: {}", t.delta);
    }

    #[test]
    fn recency_decays_and_never_goes_negative() {
        let fresh = score(&article(0, 0), &feed(0, 0), &ctx(), 0, 0);
        let stale = score(&article(0, 40), &feed(0, 0), &ctx(), 0, 0);
        let ancient = score(&article(0, 200), &feed(0, 0), &ctx(), 0, 0);
        assert!(fresh.total > stale.total);
        assert!(label_containing(&ancient, "posted").is_none_or(|t| t.delta >= 0.0));
    }

    #[test]
    fn length_preference_is_silent_without_history() {
        let s = score(&article(4000, 4), &feed(0, 0), &ctx(), 0, 0);
        assert!(label_containing(&s, "you read long").is_none());
        assert!(label_containing(&s, "shorter than").is_none());
    }

    #[test]
    fn a_long_read_matches_a_long_read_habit() {
        let mut c = ctx();
        c.finished_mean = Some(4400.0);
        let long = score(&article(4400, 4), &feed(0, 0), &c, 0, 0);
        let short = score(&article(150, 4), &feed(0, 0), &c, 0, 0);
        assert!(long.total > short.total);
        let t = label_containing(&long, "you read long pieces").expect("term missing");
        assert_eq!(t.label, "20 min — you read long pieces to the end");
    }

    #[test]
    fn rules_add_their_weight_and_name_themselves() {
        let mut c = ctx();
        c.rules = vec![Rule {
            id: 1,
            pattern: "kubernetes".into(),
            field: Field::Title,
            weight: 0.25,
        }];
        let s = score(&article(0, 4), &feed(0, 0), &c, 0, 0);
        let t = label_containing(&s, "matches your rule").expect("term missing");
        assert_eq!(t.label, "matches your rule \"kubernetes\"");
        assert_eq!(t.delta, 0.25);
        assert!(!s.vetoed);
    }

    #[test]
    fn a_veto_rule_suppresses_and_explains() {
        let mut c = ctx();
        c.rules = vec![Rule {
            id: 1,
            pattern: "kubernetes".into(),
            field: Field::Title,
            weight: -1.0,
        }];
        let s = score(&article(0, 4), &feed(0, 0), &c, 0, 0);
        assert!(s.vetoed);
        assert_eq!(s.veto_reason.as_deref(), Some("your rule \"kubernetes\""));
    }

    #[test]
    fn rules_that_do_not_match_contribute_nothing() {
        let mut c = ctx();
        c.rules = vec![Rule {
            id: 1,
            pattern: "gardening".into(),
            field: Field::Any,
            weight: 0.4,
        }];
        let s = score(&article(0, 4), &feed(0, 0), &c, 0, 0);
        assert!(label_containing(&s, "gardening").is_none());
    }

    #[test]
    fn inbound_links_promote_and_are_capped() {
        let none = score(&article(0, 4), &feed(0, 0), &ctx(), 0, 0);
        let some = score(&article(0, 4), &feed(0, 0), &ctx(), 0, 3);
        let many = score(&article(0, 4), &feed(0, 0), &ctx(), 0, 50);
        assert!(some.total > none.total);
        let t = label_containing(&some, "linked this").expect("term missing");
        assert_eq!(t.label, "3 other articles you follow linked this");
        assert!(label_containing(&many, "linked this").unwrap().delta <= 0.18);
    }

    #[test]
    fn one_inbound_link_is_singular() {
        let s = score(&article(0, 4), &feed(0, 0), &ctx(), 0, 1);
        assert_eq!(
            label_containing(&s, "linked this").unwrap().label,
            "1 other article you follow linked this"
        );
    }

    #[test]
    fn the_total_is_exactly_the_sum_of_the_terms_shown() {
        let mut c = ctx();
        c.finished_mean = Some(3000.0);
        c.rules = vec![Rule {
            id: 1,
            pattern: "kubernetes".into(),
            field: Field::Title,
            weight: 0.2,
        }];
        let s = score(&article(2800, 4), &feed(10, 8), &c, 1, 2);
        let sum: f64 = s.terms.iter().map(|t| t.delta).sum();
        assert!(
            (s.total - sum).abs() < 1e-9,
            "panel would not add up: {} vs {}",
            s.total,
            sum
        );
    }

    #[test]
    fn the_total_never_goes_below_zero() {
        let mut c = ctx();
        c.rules = vec![Rule {
            id: 1,
            pattern: "kubernetes".into(),
            field: Field::Title,
            weight: -0.9,
        }];
        let s = score(&article(0, 100), &feed(20, 0), &c, 5, 0);
        assert!(s.total >= 0.0, "was {}", s.total);
    }

    #[test]
    fn headline_reason_picks_the_strongest_positive() {
        let s = score(&article(0, 1), &feed(20, 19), &ctx(), 0, 0);
        assert_eq!(
            s.headline_reason(),
            Some("you finish 19 of 20 from this feed")
        );
    }

    #[test]
    fn headline_reason_stays_quiet_when_nothing_stands_out() {
        let s = score(&article(0, 30), &feed(0, 0), &ctx(), 0, 0);
        assert_eq!(s.headline_reason(), None);
    }

    #[test]
    fn ages_read_the_way_a_person_would_say_them() {
        assert_eq!(human_age(0.2), "just now");
        assert_eq!(human_age(4.0), "4h ago");
        assert_eq!(human_age(30.0), "1d ago");
        assert_eq!(human_age(80.0), "3d ago");
    }
}
