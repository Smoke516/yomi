//! Configuration.
//!
//! Small on purpose. The feeds live in the store now, not in a TOML file, so
//! this only holds the handful of things a person actually wants to set.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub edition: EditionConfig,
    pub appearance: Appearance,
    pub vault: Vault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditionConfig {
    /// How many articles a day's edition holds. The point is that it is small.
    pub size: usize,
    /// How far back an article can have appeared and still be today's news.
    pub window_hours: i64,
    /// Fetch the full article text, not just the feed's summary.
    pub full_text: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// An ANSI colour name. Yomi borrows the rest of its palette from the
    /// terminal, so this is the only colour it chooses.
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Vault {
    /// Where `s` writes notes. `~` is expanded.
    pub path: Option<String>,
}

impl Default for EditionConfig {
    fn default() -> Self {
        EditionConfig {
            size: 12,
            window_hours: 36,
            full_text: true,
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            accent: "yellow".into(),
        }
    }
}

impl Config {
    /// Read the config, falling back to defaults when it does not exist.
    ///
    /// A malformed config is an error rather than a silent reset: quietly
    /// ignoring someone's settings is worse than telling them the file is
    /// broken.
    pub fn load() -> Result<Config> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("{} is not valid TOML", path.display()))
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = config_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(path)
    }

    /// The vault directory, with `~` expanded. `None` means `s` is unbound.
    pub fn vault_dir(&self) -> Option<PathBuf> {
        self.vault.path.as_deref().map(expand_tilde)
    }

    pub fn window(&self) -> chrono::Duration {
        chrono::Duration::hours(self.edition.window_hours.max(1))
    }
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("could not find a config directory")?
        .join("yomi"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// The one file that holds everything Yomi knows.
pub fn store_path() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("YOMI_STORE") {
        return Ok(PathBuf::from(explicit));
    }
    Ok(dirs::data_dir()
        .context("could not find a data directory")?
        .join("yomi")
        .join("yomi.db"))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

/// Where the default vault would be, for the message shown when `s` is unbound.
pub fn suggest_vault() -> &'static str {
    "~/vault/clippings"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_a_small_bounded_edition() {
        let c = Config::default();
        assert_eq!(c.edition.size, 12);
        assert!(
            c.edition.size <= 20,
            "an edition has to stay small to mean anything"
        );
        assert_eq!(c.edition.window_hours, 36);
    }

    #[test]
    fn a_partial_config_keeps_the_other_defaults() {
        let c: Config = toml::from_str("[edition]\nsize = 5\n").unwrap();
        assert_eq!(c.edition.size, 5);
        assert_eq!(
            c.edition.window_hours, 36,
            "unset fields keep their default"
        );
        assert_eq!(c.appearance.accent, "yellow");
    }

    #[test]
    fn an_empty_config_is_the_default_config() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.edition.size, Config::default().edition.size);
    }

    #[test]
    fn config_round_trips_through_toml() {
        let mut c = Config::default();
        c.vault.path = Some("~/notes/clippings".into());
        c.appearance.accent = "cyan".into();
        let text = toml::to_string_pretty(&c).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.vault.path.as_deref(), Some("~/notes/clippings"));
        assert_eq!(back.appearance.accent, "cyan");
    }

    #[test]
    fn a_broken_config_is_an_error_not_a_silent_reset() {
        assert!(toml::from_str::<Config>("edition = [[[").is_err());
    }

    #[test]
    fn no_vault_configured_means_no_vault_dir() {
        assert_eq!(Config::default().vault_dir(), None);
    }

    #[test]
    fn tilde_is_expanded_against_home() {
        let home = dirs::home_dir().expect("test needs a home directory");
        assert_eq!(expand_tilde("~/notes"), home.join("notes"));
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn absolute_and_relative_paths_are_left_alone() {
        assert_eq!(expand_tilde("/srv/vault"), PathBuf::from("/srv/vault"));
        assert_eq!(expand_tilde("notes"), PathBuf::from("notes"));
        // A tilde that is not a home reference must not be mangled.
        assert_eq!(expand_tilde("~backup/x"), PathBuf::from("~backup/x"));
    }

    #[test]
    fn the_window_is_never_zero() {
        let mut c = Config::default();
        c.edition.window_hours = 0;
        assert_eq!(c.window(), chrono::Duration::hours(1));
        c.edition.window_hours = -5;
        assert_eq!(c.window(), chrono::Duration::hours(1));
    }

    #[test]
    fn the_store_path_can_be_overridden_for_testing() {
        // Serial-unsafe if run in parallel with another env test, so it owns
        // its own variable name.
        std::env::set_var("YOMI_STORE", "/tmp/yomi-test.db");
        assert_eq!(store_path().unwrap(), PathBuf::from("/tmp/yomi-test.db"));
        std::env::remove_var("YOMI_STORE");
    }
}
