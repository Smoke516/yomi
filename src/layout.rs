//! Laying article text out at a readable measure.
//!
//! The reading view abandons the three-pane split and gives the text a column
//! of about sixty-five characters, because that is what prose wants. This
//! module turns blocks into positioned lines; the view only has to paint them.

use crate::model::Block;

/// What a laid-out line is, so the view can style it without re-parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Heading,
    Body,
    Quote,
    Code,
    ListItem,
    Rule,
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laid {
    pub role: Role,
    pub text: String,
}

impl Laid {
    fn new(role: Role, text: impl Into<String>) -> Laid {
        Laid {
            role,
            text: text.into(),
        }
    }
}

/// The widest measure we will set prose at, however wide the terminal is.
pub const MAX_MEASURE: usize = 66;

/// Choose a measure for the available width.
pub fn measure_for(width: usize) -> usize {
    width.saturating_sub(8).clamp(20, MAX_MEASURE)
}

/// Lay blocks out into lines at `measure` characters.
pub fn lay_out(blocks: &[Block], measure: usize) -> Vec<Laid> {
    let measure = measure.max(8);
    let mut out: Vec<Laid> = Vec::new();

    for block in blocks {
        if !out.is_empty() {
            out.push(Laid::new(Role::Blank, ""));
        }
        match block {
            Block::Heading(t) => {
                for line in wrap(t, measure) {
                    out.push(Laid::new(Role::Heading, line));
                }
            }
            Block::Para(t) => {
                for line in wrap(t, measure) {
                    out.push(Laid::new(Role::Body, line));
                }
            }
            Block::Quote(lines) => {
                let inner = measure.saturating_sub(3).max(8);
                for l in lines {
                    for line in wrap(l, inner) {
                        out.push(Laid::new(Role::Quote, line));
                    }
                }
            }
            Block::Code(lines) => {
                // Code is never re-wrapped; it is shown as written and the
                // view lets it scroll if it must.
                for l in lines {
                    out.push(Laid::new(Role::Code, l.clone()));
                }
            }
            Block::List(items) => {
                let inner = measure.saturating_sub(2).max(8);
                for item in items {
                    let wrapped = wrap(item, inner);
                    for (i, line) in wrapped.iter().enumerate() {
                        let prefix = if i == 0 { "· " } else { "  " };
                        out.push(Laid::new(Role::ListItem, format!("{prefix}{line}")));
                    }
                }
            }
            Block::Rule => out.push(Laid::new(Role::Rule, "")),
        }
    }

    // A leading blank from the loop above is never wanted.
    while out.first().map(|l| l.role) == Some(Role::Blank) {
        out.remove(0);
    }
    out
}

/// Greedy word wrap, counting characters rather than bytes.
///
/// Byte-indexed wrapping is the same bug that used to abort the process on
/// em dashes; there is no reason to reintroduce it here.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    for word in text.split_whitespace() {
        let wlen = word.chars().count();

        // A word longer than the measure is hard-split rather than allowed to
        // run off the edge.
        if wlen > width {
            if current_len > 0 {
                lines.push(std::mem::take(&mut current));
                current_len = 0;
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.chars().count() == width {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(c);
            }
            if !chunk.is_empty() {
                current = chunk;
                current_len = current.chars().count();
            }
            continue;
        }

        let needed = if current_len == 0 {
            wlen
        } else {
            current_len + 1 + wlen
        };
        if needed > width {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
            current_len = wlen;
        } else {
            if current_len > 0 {
                current.push(' ');
            }
            current.push_str(word);
            current_len = needed;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_breaks_on_words() {
        let out = wrap("the quick brown fox jumps", 11);
        assert_eq!(out, vec!["the quick", "brown fox", "jumps"]);
    }

    #[test]
    fn no_wrapped_line_exceeds_the_measure() {
        let text = "The borrow checker is not a theorem prover, it is a set of rules \
                    that happen to be sound most of the time.";
        for width in 8..40 {
            for line in wrap(text, width) {
                assert!(
                    line.chars().count() <= width,
                    "width {width} produced a {}-char line: {line:?}",
                    line.chars().count()
                );
            }
        }
    }

    #[test]
    fn wrapping_counts_characters_not_bytes() {
        // Ten CJK characters are thirty bytes. A byte-based wrap would break
        // this into three lines and probably panic doing it.
        let out = wrap("読み読み読み読み読み", 12);
        assert_eq!(out, vec!["読み読み読み読み読み"]);
    }

    #[test]
    fn a_word_longer_than_the_measure_is_split_not_dropped() {
        let out = wrap("https://example.invalid/a/very/long/path/indeed", 10);
        assert!(out.iter().all(|l| l.chars().count() <= 10));
        assert_eq!(
            out.concat(),
            "https://example.invalid/a/very/long/path/indeed"
        );
    }

    #[test]
    fn a_long_multibyte_word_splits_on_character_boundaries() {
        let out = wrap("読読読読読読読読読読読読", 5);
        assert!(out.iter().all(|l| l.chars().count() <= 5));
        assert_eq!(out.concat(), "読読読読読読読読読読読読");
    }

    #[test]
    fn empty_text_still_gives_one_line() {
        assert_eq!(wrap("", 20), vec![""]);
    }

    #[test]
    fn paragraphs_are_separated_by_one_blank() {
        let blocks = vec![Block::Para("first".into()), Block::Para("second".into())];
        let laid = lay_out(&blocks, 40);
        assert_eq!(
            laid,
            vec![
                Laid::new(Role::Body, "first"),
                Laid::new(Role::Blank, ""),
                Laid::new(Role::Body, "second"),
            ]
        );
    }

    #[test]
    fn no_leading_blank_line() {
        let laid = lay_out(&[Block::Para("only".into())], 40);
        assert_eq!(laid.first().map(|l| l.role), Some(Role::Body));
    }

    #[test]
    fn code_is_never_rewrapped() {
        let long = "    let x = some_extremely_long_function_name(with, arguments);";
        let laid = lay_out(&[Block::Code(vec![long.into()])], 20);
        assert_eq!(laid.len(), 1);
        assert_eq!(laid[0].text, long, "code must keep its exact form");
        assert_eq!(laid[0].role, Role::Code);
    }

    #[test]
    fn list_items_get_a_marker_and_hanging_indent() {
        let laid = lay_out(
            &[Block::List(vec!["a fairly long list item here".into()])],
            18,
        );
        assert!(laid[0].text.starts_with("· "));
        assert!(laid[1].text.starts_with("  "));
        assert!(laid.iter().all(|l| l.role == Role::ListItem));
    }

    #[test]
    fn quotes_are_laid_out_narrower_than_body() {
        // The view indents quotes behind a marker, so their text has to be
        // wrapped to leave room for it.
        let text = "a b c d e f g h i j k l m n o p q r s t u v w x y z";
        let quote = lay_out(&[Block::Quote(vec![text.into()])], 30);
        assert!(
            quote.iter().all(|l| l.text.chars().count() <= 27),
            "quote text must leave room for the marker: {:?}",
            quote
                .iter()
                .map(|l| l.text.chars().count())
                .collect::<Vec<_>>()
        );
        assert!(quote.iter().all(|l| l.role == Role::Quote));
    }

    #[test]
    fn headings_and_rules_keep_their_roles() {
        let laid = lay_out(&[Block::Heading("Title".into()), Block::Rule], 40);
        assert_eq!(laid[0].role, Role::Heading);
        assert_eq!(laid.last().unwrap().role, Role::Rule);
    }

    #[test]
    fn the_measure_is_capped_however_wide_the_terminal() {
        assert_eq!(measure_for(500), MAX_MEASURE);
        assert_eq!(measure_for(48), 40);
    }

    #[test]
    fn a_tiny_terminal_still_gets_a_usable_measure() {
        assert_eq!(measure_for(10), 20);
        assert_eq!(measure_for(0), 20);
    }

    #[test]
    fn laying_out_nothing_produces_nothing() {
        assert!(lay_out(&[], 40).is_empty());
    }
}
