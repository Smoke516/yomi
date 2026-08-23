//! Painting the three screens.
//!
//! There are no panes and no boxes, apart from the one panel that is genuinely
//! a dialog. A front page is a column of type; a reading view is a measure of
//! prose. Chrome would only take space away from both.

use chrono::Local;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block as Panel, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, View};
use crate::edition::Placed;
use crate::fetch;
use crate::layout::{self, Role};

/// Vertical gutter marking the row the cursor is on.
const CURSOR: &str = "▌";

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    match app.view {
        View::Edition => draw_edition(f, area, app),
        View::Reading => draw_reading(f, area, app),
    }
    if app.show_why {
        draw_why(f, area, app);
    }
    if app.show_help {
        draw_help(f, area, app);
    }
}

// ---------------------------------------------------------------- edition

/// Build the edition as lines, remembering where each selectable row starts so
/// the view can keep the cursor on screen.
fn edition_lines(app: &App) -> (Vec<Line<'static>>, Vec<usize>) {
    let t = &app.theme;
    let ed = &app.edition;
    let mut lines: Vec<Line> = Vec::new();
    let mut starts: Vec<usize> = Vec::new();

    let date = Local::now().format("%A %-d %B").to_string();
    lines.push(Line::from(vec![
        Span::styled(" yomi", t.headline()),
        Span::raw("  "),
        Span::styled(date, t.meta()),
    ]));
    lines.push(Line::styled(format!(" {}", "─".repeat(66)), t.chrome()));

    let masthead = if app.refreshing {
        " refreshing…".to_string()
    } else if ed.picked.is_empty() && ed.considered == 0 {
        " no feeds have posted anything yet".to_string()
    } else if ed.picked.is_empty() {
        format!(" nothing made the edition from {} arrivals", ed.considered)
    } else {
        format!(
            " {} picked from {} · nothing is waiting for you",
            ed.picked.len(),
            ed.considered
        )
    };
    lines.push(Line::styled(masthead, t.meta()));
    lines.push(Line::raw(""));

    if ed.picked.is_empty() && ed.considered == 0 {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "   Add a feed to get started:".to_string(),
            t.body(),
        ));
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "     yomi add https://lwn.net/headlines/newrss".to_string(),
            t.reason(),
        ));
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "   then press r to fetch.".to_string(),
            t.meta(),
        ));
    }

    for (i, placed) in ed.picked.iter().enumerate() {
        starts.push(lines.len());
        let selected = app.cursor == i;
        let lead = i == 0;
        push_story(&mut lines, app, placed, selected, lead);
        lines.push(Line::raw(""));
    }

    if !ed.held.is_empty() {
        lines.push(Line::styled(
            format!(" ─ held back {}", "─".repeat(54)),
            t.chrome(),
        ));
        lines.push(Line::raw(""));
        for (j, held) in ed.held.iter().enumerate() {
            let idx = ed.picked.len() + j;
            starts.push(lines.len());
            let selected = app.cursor == idx;
            let gutter = if selected { CURSOR } else { " " };
            lines.push(Line::from(vec![
                Span::styled(format!(" {gutter} "), t.reason()),
                Span::styled(
                    format!("{:<18}", crate::util::truncate_string(&held.feed_name, 18)),
                    if selected {
                        t.headline_selected()
                    } else {
                        t.meta()
                    },
                ),
                Span::styled(format!("{:<40}", held.reason), t.chrome()),
                Span::styled("show", t.reason()),
            ]));
        }
        lines.push(Line::raw(""));
    }

    (lines, starts)
}

fn push_story(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    placed: &Placed,
    selected: bool,
    lead: bool,
) {
    let t = &app.theme;
    let a = &placed.article;
    let gutter = if selected { CURSOR } else { " " };
    let prefix = format!(" {gutter} ");

    let title_style = if selected {
        t.headline_selected()
    } else {
        t.headline()
    };

    for (n, line) in layout::wrap(&a.title, 62).into_iter().enumerate() {
        lines.push(Line::from(vec![
            Span::styled(
                if n == 0 { prefix.clone() } else { "   ".into() },
                t.reason(),
            ),
            Span::styled(line, title_style),
        ]));
    }

    let mut meta = vec![a.feed_name.clone()];
    if let Some(author) = &a.author {
        meta.push(author.clone());
    }
    meta.push(fetch::age(app.now, a.appeared()));
    if a.minutes() > 0 {
        meta.push(format!("{} min", a.minutes()));
    }
    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(meta.join(" · "), t.meta()),
    ]));

    if lead && !a.summary.trim().is_empty() {
        lines.push(Line::raw(""));
        for line in layout::wrap(&a.summary, 62).into_iter().take(3) {
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(line, t.body()),
            ]));
        }
    }

    if let Some(reason) = placed.score.headline_reason() {
        if lead {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(reason.to_string(), t.reason()),
        ]));
    }
}

fn draw_edition(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let (lines, starts) = edition_lines(app);
    let height = chunks[0].height as usize;

    // Keep the row under the cursor on screen.
    let cursor_line = starts.get(app.cursor).copied().unwrap_or(0);
    let next_line = starts.get(app.cursor + 1).copied().unwrap_or(lines.len());
    if cursor_line < app.list_scroll {
        app.list_scroll = cursor_line;
    } else if next_line > app.list_scroll + height {
        app.list_scroll = next_line.saturating_sub(height);
    }
    let max_scroll = lines.len().saturating_sub(height);
    app.list_scroll = app.list_scroll.min(max_scroll);

    f.render_widget(
        Paragraph::new(lines).scroll((app.list_scroll as u16, 0)),
        chunks[0],
    );
    f.render_widget(keybar(app, EDITION_KEYS), chunks[1]);
}

// ---------------------------------------------------------------- reading

fn draw_reading(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let t = &app.theme;
    let Some(article) = app.current_article().cloned() else {
        f.render_widget(Paragraph::new("nothing to read").style(t.meta()), chunks[1]);
        return;
    };

    let measure = layout::measure_for(chunks[1].width as usize);
    let indent = ((chunks[1].width as usize).saturating_sub(measure)) / 2;
    let pad = " ".repeat(indent);

    // -- header: source on the left, progress on the right ---------------
    let body_lines = app.reading.len();
    let visible = chunks[1].height as usize;
    let progress = if body_lines <= visible {
        100
    } else {
        let end = (app.scroll as usize + visible).min(body_lines);
        (end * 100 / body_lines).min(100)
    };
    let head_w = chunks[0].width as usize;
    let left = format!(" {}", article.feed_name);
    let right = format!("{progress}% ");
    let gap = head_w.saturating_sub(left.chars().count() + right.chars().count());
    f.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!(" {}", "─".repeat(head_w.saturating_sub(2))),
                t.chrome(),
            ),
            Line::from(vec![
                Span::styled(left, t.meta()),
                Span::raw(" ".repeat(gap)),
                Span::styled(right, t.reason()),
            ]),
        ]),
        chunks[0],
    );

    // -- body -------------------------------------------------------------
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::raw(""));
    for l in layout::wrap(&article.title, measure) {
        lines.push(Line::styled(format!("{pad}{l}"), t.headline()).alignment(Alignment::Left));
    }
    let mut byline = Vec::new();
    if let Some(author) = &article.author {
        byline.push(author.clone());
    }
    byline.push(article.feed_name.clone());
    if article.minutes() > 0 {
        byline.push(format!("{} min", article.minutes()));
    }
    if article.body.is_none() {
        byline.push("summary only".into());
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!("{pad}{}", byline.join(" · ")),
        t.meta(),
    ));
    lines.push(Line::raw(""));
    lines.push(Line::raw(""));

    for laid in &app.reading {
        let style = match laid.role {
            Role::Heading => t.headline(),
            Role::Body | Role::Blank => t.body(),
            Role::Quote => t.quote(),
            Role::Code => t.code(),
            Role::ListItem => t.body(),
            Role::Rule => t.chrome(),
        };
        let text = match laid.role {
            Role::Quote => format!("{pad}  ┃ {}", laid.text),
            Role::Code => format!("{pad}  {}", laid.text),
            Role::Rule => format!("{pad}{}", "·".repeat(measure.min(20))),
            _ => format!("{pad}{}", laid.text),
        };
        lines.push(Line::styled(text, style));
    }

    f.render_widget(Paragraph::new(lines).scroll((app.scroll, 0)), chunks[1]);
    f.render_widget(keybar(app, READING_KEYS), chunks[2]);
}

// ------------------------------------------------------------------- why

fn draw_why(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let Some(placed) = app.current_placed() else {
        return;
    };

    let mut lines: Vec<Line> = vec![Line::raw("")];
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            crate::util::truncate_string(&placed.article.title, 60),
            t.headline(),
        ),
    ]));
    lines.push(Line::raw(""));

    for term in &placed.score.terms {
        let delta = if term.delta > 0.0 {
            format!("+ {:.2}", term.delta)
        } else if term.delta < 0.0 {
            format!("- {:.2}", term.delta.abs())
        } else {
            "  0.00".to_string()
        };
        let label = crate::util::truncate_string(&term.label, 48);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{label:<50}"), t.meta()),
            Span::styled(
                format!("{delta:>6}"),
                if term.delta.abs() >= 0.005 {
                    t.reason()
                } else {
                    t.meta()
                },
            ),
        ]));
    }

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(" ".repeat(50)),
        Span::styled("──────", t.chrome()),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(" ".repeat(50)),
        Span::styled(format!("{:>6.2}", placed.score.total), t.headline()),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "every number came from your own reading history.".to_string(),
            t.meta(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("nothing left this machine.".to_string(), t.meta()),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("d", t.reason()),
        Span::styled(" stop ranking this feed up    ", t.meta()),
        Span::styled("esc", t.reason()),
        Span::styled(" close", t.meta()),
    ]));

    let height = (lines.len() as u16 + 2).min(area.height);
    let rect = centred(area, 72, height);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines).block(
            Panel::default()
                .borders(Borders::ALL)
                .border_style(t.chrome())
                .title(Span::styled(" why is this on your front page? ", t.meta())),
        ),
        rect,
    );
}

// ------------------------------------------------------------------ help

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let rows: &[(&str, &str)] = &[
        ("j k ↑ ↓", "move"),
        ("g G", "first / last"),
        ("↵", "read, or reveal a held-back feed"),
        ("o", "open in browser"),
        ("s", "save to the vault"),
        ("w", "why is this here?"),
        ("r", "refresh feeds"),
        ("m", "mute this feed — `yomi unmute` undoes it"),
        ("?", "this"),
        ("q", "quit, or leave the article"),
    ];
    let mut lines: Vec<Line> = vec![Line::raw("")];
    for (keys, what) in rows {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{keys:<10}"), t.reason()),
            Span::styled(what.to_string(), t.body()),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "yomi keeps no unread count. yesterday's edition is gone.".to_string(),
            t.meta(),
        ),
    ]));

    let height = (lines.len() as u16 + 2).min(area.height);
    let rect = centred(area, 62, height);
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines).block(
            Panel::default()
                .borders(Borders::ALL)
                .border_style(t.chrome())
                .title(Span::styled(" keys ", t.meta())),
        ),
        rect,
    );
}

// --------------------------------------------------------------- shared

const EDITION_KEYS: &[(&str, &str)] = &[
    ("↵", "read"),
    ("o", "browser"),
    ("s", "→ vault"),
    ("w", "why"),
    ("r", "refresh"),
    ("?", "keys"),
    ("q", "quit"),
];

const READING_KEYS: &[(&str, &str)] = &[
    ("j/k", "scroll"),
    ("s", "→ vault"),
    ("o", "browser"),
    ("n", "next"),
    ("esc", "back"),
];

fn keybar<'a>(app: &App, keys: &'a [(&'a str, &'a str)]) -> Paragraph<'a> {
    let t = &app.theme;
    if !app.status.is_empty() {
        return Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(app.status.clone(), t.reason()),
        ]));
    }
    let mut spans = vec![Span::raw(" ")];
    for (key, what) in keys {
        spans.push(Span::styled(*key, t.chrome()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*what, t.meta()));
        spans.push(Span::raw("   "));
    }
    Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false })
}

fn centred(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centring_never_leaves_the_area() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 10,
        };
        for (w, h) in [(72, 20), (10, 4), (40, 10), (1, 1)] {
            let r = centred(area, w, h);
            assert!(r.x + r.width <= area.width, "{w}x{h} overflowed width");
            assert!(r.y + r.height <= area.height, "{w}x{h} overflowed height");
        }
    }

    #[test]
    fn a_panel_wider_than_the_terminal_is_clamped() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 8,
        };
        let r = centred(area, 72, 20);
        assert_eq!(r.width, 30);
        assert_eq!(r.height, 8);
    }
}
