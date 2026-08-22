//! Small shared helpers.

/// Shorten `s` to at most `max_len` *characters*, marking the cut with `...`.
///
/// Counting and slicing are both done in characters rather than bytes. RSS
/// titles are full of em dashes, curly quotes, accents and CJK, and a byte
/// index lands mid-codepoint on any of them.
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    // No room for the ellipsis and anything else; just hard-cut.
    if max_len <= 3 {
        return s.chars().take(max_len).collect();
    }
    let head: String = s.chars().take(max_len - 3).collect();
    format!("{head}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_short_strings_alone() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn exact_length_is_not_truncated() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }

    #[test]
    fn output_never_exceeds_the_budget() {
        for len in 0..40 {
            let out = truncate_string("a rather long headline about things", len);
            assert!(
                out.chars().count() <= len,
                "len {len} produced {} chars",
                out.chars().count()
            );
        }
    }

    /// The bug this module exists for: slicing by byte index used to abort the
    /// process when the cut fell inside a multi-byte character. Inside the TUI
    /// that panic left the terminal in raw mode.
    #[test]
    fn does_not_panic_on_multibyte_boundaries() {
        // An em dash is three bytes, so sweeping the budget walks the cut
        // through the middle of one at some point.
        let title = "Rust 1.94 released — what is new for embedded developers";
        for len in 0..title.chars().count() + 5 {
            let _ = truncate_string(title, len);
        }
    }

    /// The exact input that aborted the old implementation: pressing `o` on
    /// this headline called truncate_string(title, 30), which sliced at byte
    /// 27 -- inside the em dash at bytes 26..29.
    #[test]
    fn regression_em_dash_at_the_cut() {
        let title = "Kubernetes 1.35 ships CNI \u{2014} and what it breaks";
        assert!(!title.is_char_boundary(27));
        assert_eq!(
            truncate_string(title, 30),
            "Kubernetes 1.35 ships CNI \u{2014}..."
        );
    }

    #[test]
    fn counts_characters_not_bytes() {
        // Twelve characters, thirty-six bytes. The byte-based check used to
        // truncate this even though it comfortably fits.
        let cjk = "読み読み読み読み読み読み";
        assert_eq!(cjk.chars().count(), 12);
        assert_eq!(truncate_string(cjk, 12), cjk);
    }

    #[test]
    fn truncates_multibyte_on_character_boundaries() {
        assert_eq!(truncate_string("読み読み読み読み", 5), "読み...");
    }

    #[test]
    fn handles_emoji() {
        let s = "🎌🎌🎌🎌🎌🎌";
        assert_eq!(truncate_string(s, 4), "🎌...");
    }

    #[test]
    fn tiny_budgets_drop_the_ellipsis() {
        assert_eq!(truncate_string("hello", 3), "hel");
        assert_eq!(truncate_string("hello", 0), "");
    }
}
