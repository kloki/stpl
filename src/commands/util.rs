//! Shared helpers for command implementations.

use anyhow::Result;

use crate::{config::Config, error::StplError, output::Style, resolve};

/// Load config and derive the output `Style` in one shot — the common preamble
/// for nearly every command.
pub fn config_and_style() -> Result<(Config, Style)> {
    let config = Config::load()?;
    let style = Style::from_config(&config);
    Ok((config, style))
}

/// Resolve `query` to a single memo, but on `Ambiguous` print the candidate
/// matches as clickable lines to stderr before propagating the error. This is
/// what surfaces the candidate list to the user while still exiting non-zero.
pub fn resolve_or_show(config: &Config, style: &Style, query: &str) -> Result<crate::memo::Memo> {
    match resolve::resolve_one(config, query) {
        Ok(memo) => Ok(memo),
        Err(StplError::Ambiguous { query, matches }) => {
            anstream::eprintln!("multiple memos match '{query}' — be more specific:");
            for memo in &matches {
                anstream::eprintln!("  {}", style.memo_line(memo));
            }
            Err(StplError::Ambiguous { query, matches }.into())
        }
        Err(other) => Err(other.into()),
    }
}
