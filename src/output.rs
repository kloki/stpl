//! Terminal output: color/emoji gating, OSC 8 hyperlinks, memo formatting.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::{
    env,
    io::{self, IsTerminal},
    path::Path,
};

use anstyle::{AnsiColor, Color, Style as AnsiStyle};

use crate::{config::Config, memo::Memo};

/// Resolved presentation capabilities for this invocation.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// ANSI color enabled.
    pub color: bool,
    /// Emoji enabled.
    pub emoji: bool,
    /// OSC 8 hyperlinks enabled.
    pub hyperlinks: bool,
}

impl Style {
    /// Derive a `Style` from config + environment.
    ///
    /// - `color`  = !config.disable_color && env `NO_COLOR` unset/empty && stdout.is_terminal()
    /// - `emoji`  = !config.disable_emoji
    /// - `hyperlinks` = stdout.is_terminal()  (fall back to bare path otherwise)
    pub fn from_config(config: &Config) -> Style {
        let is_tty = io::stdout().is_terminal();
        let no_color = env::var_os("NO_COLOR")
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        Style {
            color: !config.disable_color && !no_color && is_tty,
            emoji: !config.disable_emoji,
            hyperlinks: is_tty,
        }
    }

    /// Render `text` as an OSC 8 hyperlink to `file://<abs path>` when
    /// `hyperlinks` is on; otherwise return `text` unchanged. The `path`
    /// must be absolute; spaces are percent-encoded.
    pub fn link(&self, text: &str, path: &Path) -> String {
        if !self.hyperlinks {
            return text.to_string();
        }
        let encoded = path.to_string_lossy().replace(' ', "%20");
        format!("\x1b]8;;file://{encoded}\x1b\\{text}\x1b]8;;\x1b\\")
    }

    /// Format a single memo as the canonical `📎 title[file://...]` line.
    /// The whole `title[path]` token is the clickable link; the leading
    /// paperclip is dropped when emoji is disabled.
    pub fn memo_line(&self, memo: &Memo) -> String {
        let token = format!("{}[{}]", memo.title, memo.path.display());
        let linked = self.link(&token, &memo.path);
        if self.emoji {
            format!("📎 {linked}")
        } else {
            linked
        }
    }
}

/// Print a success/informational message to stdout (what the command did),
/// honoring the style (e.g. green/emoji when enabled).
pub fn success(style: &Style, message: &str) {
    let prefix = if style.emoji { "📎 " } else { "" };
    if style.color {
        let green = AnsiStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
        anstream::println!("{prefix}{green}{message}{green:#}");
    } else {
        anstream::println!("{prefix}{message}");
    }
}

/// Print an error in red on stderr. Always safe to call regardless of TTY.
pub fn print_error(err: &anyhow::Error) {
    // anstream's stderr AutoStream strips ANSI when stderr is not a terminal,
    // so emitting color unconditionally is safe.
    let red = AnsiStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));
    anstream::eprintln!("{red}error:{red:#} {err}");
    // Include the full anyhow cause chain.
    for cause in err.chain().skip(1) {
        anstream::eprintln!("  {red}caused by:{red:#} {cause}");
    }
}
