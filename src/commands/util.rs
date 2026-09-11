//! Shared helpers for command implementations.

use anyhow::Result;

use crate::{
    config::Config,
    diary::{self, Diary},
    error::StplError,
    memo::Memo,
    output::Style,
    resolve, state,
};

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

/// `resolve_or_show` for diaries: on `AmbiguousDiary`, list the candidates as
/// clickable lines on stderr before propagating.
pub fn resolve_diary_or_show(config: &Config, style: &Style, query: &str) -> Result<Diary> {
    match diary::resolve_one(config, query) {
        Ok(diary) => Ok(diary),
        Err(StplError::AmbiguousDiary { query, matches }) => {
            anstream::eprintln!("multiple diaries match '{query}' — be more specific:");
            for diary in &matches {
                anstream::eprintln!("  {}", style.diary_line(diary));
            }
            Err(StplError::AmbiguousDiary { query, matches }.into())
        }
        Err(other) => Err(other.into()),
    }
}

/// `resolve_target` for diaries: `None` falls back to the last diary used.
/// Does not touch the pointer — used by `stpl diary del`.
pub fn resolve_diary_target(config: &Config, style: &Style, name: Option<&str>) -> Result<Diary> {
    match name {
        Some(name) => resolve_diary_or_show(config, style, name),
        None => state::load_diary().ok_or_else(|| StplError::NoLastDiary.into()),
    }
}

/// `resolve_diary_target`, plus recording the result as the last diary used.
pub fn resolve_diary_or_last(config: &Config, style: &Style, name: Option<&str>) -> Result<Diary> {
    let diary = resolve_diary_target(config, style, name)?;
    state::record_diary(&diary.path);
    Ok(diary)
}
