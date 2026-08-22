//! Turning article HTML into something worth reading.
//!
//! Feeds hand over HTML of wildly varying quality, and most of them truncate.
//! We reduce it to a small Markdown subset and store *that*, for two reasons:
//! it is the form the reading view lays out, and it is the form the vault
//! wants. One canonical body, two consumers.

use crate::model::Block;
use ego_tree::NodeRef;
use scraper::{Html, Node, Selector};

/// Elements whose contents are never article text.
const DROP: &[&str] = &[
    "script",
    "style",
    "noscript",
    "nav",
    "header",
    "footer",
    "aside",
    "form",
    "svg",
    "iframe",
    "figcaption",
    "button",
];

/// Reduce article HTML to the Markdown subset the reader understands.
pub fn html_to_markdown(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let mut out = Vec::new();
    walk(doc.tree.root(), &mut out, false);
    normalise(&out.join(""))
}

fn walk(node: NodeRef<'_, Node>, out: &mut Vec<String>, in_pre: bool) {
    match node.value() {
        Node::Text(t) => {
            if in_pre {
                out.push(t.to_string());
            } else {
                // Collapse feed whitespace; block structure comes from tags.
                let cleaned = t.replace(['\n', '\r', '\t'], " ");
                out.push(cleaned);
            }
        }
        Node::Element(el) => {
            let tag = el.name();
            if DROP.contains(&tag) {
                return;
            }
            match tag {
                "br" => out.push("\n".into()),
                "hr" => out.push("\n\n---\n\n".into()),
                "p" | "div" | "section" | "article" => {
                    out.push("\n\n".into());
                    for c in node.children() {
                        walk(c, out, in_pre);
                    }
                    out.push("\n\n".into());
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    out.push("\n\n# ".into());
                    for c in node.children() {
                        walk(c, out, in_pre);
                    }
                    out.push("\n\n".into());
                }
                "blockquote" => {
                    let mut inner = Vec::new();
                    for c in node.children() {
                        walk(c, &mut inner, in_pre);
                    }
                    let text = normalise(&inner.join(""));
                    out.push("\n\n".into());
                    for line in text.lines() {
                        out.push(format!("> {line}\n"));
                    }
                    out.push("\n".into());
                }
                "pre" => {
                    let mut inner = Vec::new();
                    for c in node.children() {
                        walk(c, &mut inner, true);
                    }
                    let text = inner.join("");
                    out.push("\n\n```\n".into());
                    out.push(text.trim_matches('\n').to_string());
                    out.push("\n```\n\n".into());
                }
                "li" => {
                    out.push("\n- ".into());
                    for c in node.children() {
                        walk(c, out, in_pre);
                    }
                }
                "ul" | "ol" => {
                    out.push("\n\n".into());
                    for c in node.children() {
                        walk(c, out, in_pre);
                    }
                    out.push("\n\n".into());
                }
                _ => {
                    for c in node.children() {
                        walk(c, out, in_pre);
                    }
                }
            }
        }
        _ => {
            for c in node.children() {
                walk(c, out, in_pre);
            }
        }
    }
}

/// Squeeze runs of blank lines and trailing spaces, but leave fenced code
/// blocks exactly as they were.
fn normalise(s: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_fence = false;
    for raw in s.lines() {
        if raw.trim_start().starts_with("```") {
            in_fence = !in_fence;
            lines.push(raw.trim().to_string());
            continue;
        }
        if in_fence {
            lines.push(raw.trim_end().to_string());
            continue;
        }
        let squeezed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if squeezed.is_empty() {
            if lines.last().map(|l| l.is_empty()) != Some(true) {
                lines.push(String::new());
            }
        } else {
            lines.push(squeezed);
        }
    }
    while lines.first().map(|l| l.is_empty()) == Some(true) {
        lines.remove(0);
    }
    while lines.last().map(|l| l.is_empty()) == Some(true) {
        lines.pop();
    }
    lines.join("\n")
}

/// Reduce a whole article *page* to Markdown.
///
/// Prefers the region a well-built page marks as the article, and only falls
/// back to the whole document when there is no such marker. This is not a full
/// Readability port — it is the eighty-percent version, and when it misses,
/// `o` still opens the real page in a browser.
pub fn article_markdown(html: &str) -> String {
    for selector in [
        "article",
        "main",
        "[role=main]",
        "#content",
        ".post-content",
    ] {
        let Ok(sel) = Selector::parse(selector) else {
            continue;
        };
        let doc = Html::parse_document(html);
        if let Some(region) = doc.select(&sel).next() {
            let mut out = Vec::new();
            for child in region.children() {
                walk(child, &mut out, false);
            }
            let md = normalise(&out.join(""));
            // A nav-only <main> is worse than nothing; require real prose.
            if word_count(&md) >= 40 {
                return md;
            }
        }
    }
    html_to_markdown(html)
}

/// Words, for reading-time and the length-preference term.
pub fn word_count(markdown: &str) -> i64 {
    let mut in_fence = false;
    let mut words = 0i64;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        words += line
            .split_whitespace()
            .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
            .count() as i64;
    }
    words
}

/// Parse the stored Markdown back into blocks for layout.
pub fn markdown_to_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let mut quote: Vec<String> = Vec::new();
    let mut list: Vec<String> = Vec::new();
    let mut code: Vec<String> = Vec::new();
    let mut in_code = false;

    macro_rules! flush {
        () => {
            if !para.is_empty() {
                blocks.push(Block::Para(para.join(" ")));
                para.clear();
            }
            if !quote.is_empty() {
                blocks.push(Block::Quote(std::mem::take(&mut quote)));
            }
            if !list.is_empty() {
                blocks.push(Block::List(std::mem::take(&mut list)));
            }
        };
    }

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code {
                blocks.push(Block::Code(std::mem::take(&mut code)));
                in_code = false;
            } else {
                flush!();
                in_code = true;
            }
            continue;
        }
        if in_code {
            code.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush!();
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            flush!();
            blocks.push(Block::Heading(rest.trim().to_string()));
        } else if trimmed == "---" {
            flush!();
            blocks.push(Block::Rule);
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            if !para.is_empty() || !list.is_empty() {
                flush!();
            }
            quote.push(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            if !para.is_empty() || !quote.is_empty() {
                flush!();
            }
            list.push(rest.trim().to_string());
        } else {
            if !quote.is_empty() || !list.is_empty() {
                flush!();
            }
            para.push(trimmed.to_string());
        }
    }
    if in_code && !code.is_empty() {
        blocks.push(Block::Code(code));
    }
    flush!();
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_survive_the_round_trip() {
        let md = html_to_markdown("<p>First para.</p><p>Second para.</p>");
        let blocks = markdown_to_blocks(&md);
        assert_eq!(
            blocks,
            vec![
                Block::Para("First para.".into()),
                Block::Para("Second para.".into())
            ]
        );
    }

    #[test]
    fn scripts_and_navigation_are_dropped() {
        let md = html_to_markdown(
            "<nav>Home About</nav><script>alert('x')</script><p>Real text.</p><footer>(c) 2026</footer>",
        );
        assert_eq!(md, "Real text.");
    }

    #[test]
    fn headings_become_blocks() {
        let blocks = markdown_to_blocks(&html_to_markdown("<h2>The Heading</h2><p>Body.</p>"));
        assert_eq!(
            blocks,
            vec![
                Block::Heading("The Heading".into()),
                Block::Para("Body.".into())
            ]
        );
    }

    #[test]
    fn blockquotes_keep_their_lines() {
        let blocks = markdown_to_blocks(&html_to_markdown(
            "<blockquote><p>Soundness holes are not bugs.</p></blockquote>",
        ));
        assert_eq!(
            blocks,
            vec![Block::Quote(vec!["Soundness holes are not bugs.".into()])]
        );
    }

    #[test]
    fn code_keeps_its_whitespace() {
        let html = "<pre><code>fn main() {\n    println!(\"hi\");\n}</code></pre>";
        let md = html_to_markdown(html);
        let blocks = markdown_to_blocks(&md);
        assert_eq!(
            blocks,
            vec![Block::Code(vec![
                "fn main() {".into(),
                "    println!(\"hi\");".into(),
                "}".into(),
            ])],
            "indentation inside a code block must not be collapsed"
        );
    }

    #[test]
    fn lists_are_kept_as_items() {
        let blocks = markdown_to_blocks(&html_to_markdown("<ul><li>one</li><li>two</li></ul>"));
        assert_eq!(blocks, vec![Block::List(vec!["one".into(), "two".into()])]);
    }

    #[test]
    fn horizontal_rules_survive() {
        let blocks = markdown_to_blocks(&html_to_markdown("<p>a</p><hr><p>b</p>"));
        assert_eq!(
            blocks,
            vec![
                Block::Para("a".into()),
                Block::Rule,
                Block::Para("b".into())
            ]
        );
    }

    #[test]
    fn feed_whitespace_is_collapsed() {
        let md = html_to_markdown("<p>lots\n   of\t\tspace   here</p>");
        assert_eq!(md, "lots of space here");
    }

    #[test]
    fn entities_are_decoded() {
        let md = html_to_markdown("<p>Tom &amp; Jerry &lt;3</p>");
        assert_eq!(md, "Tom & Jerry <3");
    }

    #[test]
    fn empty_html_yields_nothing() {
        assert_eq!(html_to_markdown(""), "");
        assert!(markdown_to_blocks("").is_empty());
    }

    #[test]
    fn word_count_ignores_code_and_punctuation() {
        let md = "One two three.\n\n```\nfn a() {}\nfn b() {}\n```\n\nfour five.";
        assert_eq!(word_count(md), 5);
    }

    #[test]
    fn a_quote_directly_after_a_paragraph_does_not_swallow_it() {
        let blocks = markdown_to_blocks("Some text.\n> quoted line\nmore text.");
        assert_eq!(
            blocks,
            vec![
                Block::Para("Some text.".into()),
                Block::Quote(vec!["quoted line".into()]),
                Block::Para("more text.".into()),
            ]
        );
    }

    #[test]
    fn an_unterminated_code_fence_still_produces_a_block() {
        let blocks = markdown_to_blocks("```\nlet x = 1;\n");
        assert_eq!(blocks, vec![Block::Code(vec!["let x = 1;".into()])]);
    }

    #[test]
    fn the_article_region_is_preferred_over_page_furniture() {
        let prose = "word ".repeat(60);
        let html = format!(
            "<html><body><nav>Home Blog About Contact</nav>\
             <article><p>{prose}</p></article>\
             <footer>Copyright 2026 someone</footer></body></html>"
        );
        let md = article_markdown(&html);
        assert!(md.starts_with("word word"));
        assert!(!md.contains("Copyright"));
    }

    #[test]
    fn a_page_with_no_article_marker_falls_back_to_everything() {
        let prose = "sentence ".repeat(60);
        let html = format!("<html><body><div><p>{prose}</p></div></body></html>");
        assert!(article_markdown(&html).contains("sentence sentence"));
    }

    #[test]
    fn a_navigation_only_main_is_rejected_as_the_article() {
        let prose = "actual article text ".repeat(30);
        let html = format!(
            "<html><body><main><p>Skip to content</p></main>\
             <div><p>{prose}</p></div></body></html>"
        );
        let md = article_markdown(&html);
        assert!(
            md.contains("actual article text"),
            "a three-word <main> must not win over the real body"
        );
    }

    #[test]
    fn nested_markup_inside_a_paragraph_is_flattened() {
        let md = html_to_markdown("<p>a <strong>bold</strong> and <em>italic</em> word</p>");
        assert_eq!(md, "a bold and italic word");
    }
}
