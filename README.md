<div align="center">

# 🎌 Yomi (読み)

### A daily edition for your feeds

*Twelve things worth reading, ranked on your machine, with the reason shown.*
*No unread count, ever.*

[![CI](https://github.com/Smoke516/yomi/actions/workflows/ci.yml/badge.svg)](https://github.com/Smoke516/yomi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)]()

</div>

## Why this is not another RSS reader

Every terminal feed reader is the same shape: folders, messages, a preview
pane, and a number that climbs until you declare bankruptcy. That is an email
client, and treating writing you *chose* to follow like mail you *didn't* is
what makes RSS feel like a chore.

Yomi is a **daily edition** instead. Bounded, ranked, and honest about what it
held back. You open it, read what is there, and close it. Yesterday's edition
is gone — that is the feature.

```
 yomi  Saturday 22 August
 ──────────────────────────────────────────────────────────────────
 6 picked from 25 · nothing is waiting for you

 ▌ Enabling the next-generation trait solver on nightly
   Rust Blog · lcnr · 1d · 4 min

   After nearly 4 years of active development, the next-generation
   trait solver is close to stabilization. We are enabling it by
   default on nightly to surface any remaining issues.

   matches your rule "Rust"

   Supply chain attack on arrayref (Rust blog)
   LWN.net · corbet · 2d

   [$] Considering the OpenMDW license
   LWN.net · corbet · 1d

 ─ held back ──────────────────────────────────────────────────────

   LWN.net           13 posts, 3 stopped by your rules       show
   Rust Blog         6 more posts                            show

 ↵ read   o browser   s → vault   w why   r refresh   ? keys   q quit
```

**There is no unread count anywhere in the interface.** The header says how
much Yomi discarded, not how much you owe.

## The three ideas

### 1. A bounded edition, not an inbox

Twelve articles by default. A feed that posts forty times a day gets one slot,
not forty — each additional item from the same feed costs more than the last,
so a firehose cannot crowd out the blog that posts monthly.

### 2. Ranking you can interrogate

Press `w` on anything:

```
┌ why is this on your front page? ─────────────────────────────────────┐
│                                                                      │
│  Announcing Rust 1.98.0                                              │
│                                                                      │
│  Rust Blog, base                                   + 0.50            │
│  1 already in today's edition from this feed       - 0.18            │
│  matches your rule "Rust"                          + 0.40            │
│                                                    ──────            │
│                                                      0.72            │
│                                                                      │
│  every number came from your own reading history.                    │
│  nothing left this machine.                                          │
│                                                                      │
│  d stop ranking this feed up    esc close                            │
└──────────────────────────────────────────────────────────────────────┘
```

Five or six additive terms you can read in three seconds. Not a model — no
training, no opacity, no vendor. Every algorithmic feed people resent is one
they cannot ask this question of. `d` corrects it on the spot.

### 3. Reading that is actually reading

`↵` drops the panes and gives the text a measure of about sixty-five
characters, centred, with real paragraph rhythm, blockquotes and code. Most
feeds truncate, so Yomi fetches the article page and extracts it — you get the
piece, not the teaser.

`s` writes it into your markdown vault as a note with frontmatter, ready for
[Scribble](https://github.com/Smoke516/scribble) or any Obsidian-style vault.
It never overwrites a file it did not create.

## Install

```bash
git clone https://github.com/Smoke516/yomi.git
cd yomi
cargo install --path .
```

Requires **Rust 1.82+**.

## Getting started

```bash
yomi add https://lwn.net/headlines/newrss     # name is taken from the feed
yomi add https://blog.rust-lang.org/feed.xml
yomi refresh
yomi                                          # open the edition
```

## Keys

| | |
|---|---|
| `j` `k` `↑` `↓` | move |
| `g` `G` | first / last |
| `↵` | read — or, on a held-back row, reveal that feed |
| `o` | open in browser |
| `s` | save to the vault |
| `w` | why is this here? |
| `r` | refresh |
| `m` | mute the selected feed (stays subscribed, leaves the edition) |
| `?` | keys |
| `q` | quit, or leave the article |

In an article: `j`/`k` scroll, `space` pages, `n` goes to the next one.
Reaching the end is what tells the ranker you finished it.

## From the shell

Everything the interface shows, the shell shows too.

```bash
yomi today                 # the edition as text
yomi today --json          # ... and as JSON, for jq
yomi why 3                 # the score breakdown for article 3
yomi list                  # subscribed feeds
yomi status                # where things live, and how much is stored
yomi config                # write a config file to edit
```

Rules are plain substring matches, because a rule you cannot read at a glance
is a rule you cannot trust:

```bash
yomi rule boost "rust" --weight 0.4    # promote
yomi rule demote "webinar"             # push down
yomi rule hide "crypto"                # keep out of the edition entirely
yomi rule list
yomi rule remove 2
```

Hidden articles still appear in the held-back count, which names the rule that
did it — filtering you cannot see is filtering you cannot trust.

## Configuration

`yomi config` writes a file you can edit:

```toml
[edition]
size = 12          # how many articles a day's edition holds
window_hours = 36  # how far back something can be and still be today's news
full_text = true   # fetch the article page, not just the feed summary

[appearance]
accent = "yellow"  # an ANSI colour name

[vault]
path = "~/vault/clippings"   # where `s` writes notes
```

**Colour.** Yomi uses the sixteen ANSI slots and inherits the palette you
already configured for your terminal. There is exactly one accent, and it only
ever carries meaning — why an article is here, and what is held back. Nothing
is coloured for decoration.

## Where things live

| | |
|---|---|
| store | `~/.local/share/yomi/yomi.db` (or `$YOMI_STORE`) |
| config | `~/.config/yomi/config.toml` |

One SQLite file holds the feeds, every article Yomi has seen, what you did with
them, and your rules. Keeping articles rather than refetching them is what
makes the ranking, offline reading and instant startup possible.

`yomi status` prints both paths.

## What Yomi deliberately does not have

- **An unread count.** Removing it is the point, not an oversight.
- **Mark-all-as-read.** A feature that exists to dismiss a number that should
  not be there.
- **A three-pane split.** At eighty columns each pane gets twenty-six.
- **A hardcoded theme.** Your terminal already has one.

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- **[ratatui](https://github.com/ratatui/ratatui)** — the TUI framework
- **[feed-rs](https://github.com/feed-rs/feed-rs)** — RSS and Atom parsing
- **Newsboat** — for proving terminal RSS reading was worth doing, and for
  being the thing this disagrees with

---

<div align="center">

**Yomi** (読み) means *reading* in Japanese.

</div>
