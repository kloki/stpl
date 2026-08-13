//! Shared helpers for command implementations.

use anyhow::Result;

use crate::{config::Config, error::StplError, memo::Memo, output::Style, resolve, state};

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
pub fn resolve_or_show(config: &Config, style: &Style, query: &str) -> Result<Memo> {
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

/// Resolve the memo a command should act on, allowing the title to be omitted:
/// `Some(query)` fuzzy-resolves as usual, `None` falls back to the last memo
/// used (`state::load`) and errors with `NoLastMemo` when there is none.
///
/// This does *not* touch the stored pointer — see `resolve_or_last`. Use it for
/// commands that shouldn't leave the memo remembered afterwards (`del`).
pub fn resolve_target(config: &Config, style: &Style, query: Option<&str>) -> Result<Memo> {
    match query {
        Some(query) => resolve_or_show(config, style, query),
        None => state::load().ok_or_else(|| StplError::NoLastMemo.into()),
    }
}

/// `resolve_target`, plus recording the result as the new "last used" memo.
/// This is the single write point for the pointer; commands that *move* a memo
/// (`rename`, `expand`) re-record the new path themselves afterwards.
pub fn resolve_or_last(config: &Config, style: &Style, query: Option<&str>) -> Result<Memo> {
    let memo = resolve_target(config, style, query)?;
    state::record(&memo.path);
    Ok(memo)
}
