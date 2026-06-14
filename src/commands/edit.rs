//! `stpl edit <title>` — open a memo in $EDITOR.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, editor};

/// Fuzzy-resolve `title` (`resolve::resolve_one`) and open the resolved memo's
/// path in the editor (`editor::open`). Ambiguity/NotFound propagate as errors;
/// the caller renders ambiguous matches.
pub fn run(title: &str) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    editor::open(&memo.path)?;
    Ok(())
}
