//! Colour, used sparingly and borrowed from the terminal.
//!
//! The old build hardcoded Tokyo Night as RGB triples, which overrode whatever
//! palette the reader had already chosen for their terminal. This one uses the
//! sixteen ANSI slots, so Yomi looks like the rest of your session by default.
//!
//! There is exactly one accent, and it only ever carries meaning: why an
//! article is on the front page, and what is being held back. Nothing is
//! coloured for decoration.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The single accent. Reasons, and nothing else.
    pub accent: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent: Color::Yellow,
        }
    }
}

impl Theme {
    /// Resolve an accent named in the config file. Unknown names fall back to
    /// the default rather than failing to start.
    pub fn from_accent_name(name: &str) -> Theme {
        Theme {
            accent: parse_ansi_colour(name).unwrap_or(Color::Yellow),
        }
    }

    /// Ordinary text: the terminal's own foreground.
    pub fn body(&self) -> Style {
        Style::default()
    }

    /// A headline.
    pub fn headline(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    /// A headline the cursor is on.
    pub fn headline_selected(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Bylines, timestamps, counts.
    pub fn meta(&self) -> Style {
        Style::default().fg(Color::Gray)
    }

    /// Rules, box edges, key hints.
    pub fn chrome(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }

    /// A reason. The only place the accent appears.
    pub fn reason(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// A quoted passage.
    pub fn quote(&self) -> Style {
        Style::default().add_modifier(Modifier::ITALIC)
    }

    /// Code, held slightly back from prose.
    pub fn code(&self) -> Style {
        Style::default().fg(Color::Cyan)
    }
}

fn parse_ansi_colour(name: &str) -> Option<Color> {
    match name.trim().to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_accent_is_an_ansi_slot_not_an_rgb_triple() {
        let t = Theme::default();
        assert!(
            !matches!(t.accent, Color::Rgb(..)),
            "hardcoding RGB overrides the reader's own terminal palette"
        );
    }

    #[test]
    fn body_text_does_not_set_a_colour_at_all() {
        // Prose should be whatever the terminal's foreground already is.
        assert_eq!(Theme::default().body().fg, None);
        assert_eq!(Theme::default().body().bg, None);
    }

    #[test]
    fn nothing_sets_a_background() {
        let t = Theme::default();
        for style in [
            t.body(),
            t.headline(),
            t.headline_selected(),
            t.meta(),
            t.chrome(),
            t.reason(),
            t.quote(),
            t.code(),
        ] {
            assert_eq!(
                style.bg, None,
                "a background would fight the terminal's own"
            );
        }
    }

    #[test]
    fn the_accent_is_reserved_for_reasons_and_selection() {
        let t = Theme::from_accent_name("magenta");
        assert_eq!(t.reason().fg, Some(Color::Magenta));
        assert_eq!(t.headline_selected().fg, Some(Color::Magenta));
        // Everything else must stay off the accent.
        assert_ne!(t.meta().fg, Some(Color::Magenta));
        assert_ne!(t.chrome().fg, Some(Color::Magenta));
        assert_ne!(t.headline().fg, Some(Color::Magenta));
    }

    #[test]
    fn accent_names_are_case_insensitive() {
        assert_eq!(Theme::from_accent_name("Cyan").accent, Color::Cyan);
        assert_eq!(Theme::from_accent_name("  BLUE ").accent, Color::Blue);
    }

    #[test]
    fn an_unknown_accent_falls_back_rather_than_failing() {
        assert_eq!(Theme::from_accent_name("puce").accent, Color::Yellow);
        assert_eq!(Theme::from_accent_name("").accent, Color::Yellow);
    }
}
