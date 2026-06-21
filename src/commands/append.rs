//! `stpl append <title> -m <message>` — append a line to an existing memo.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, output, store};

/// Fuzzy-resolve `title` and append `message` to its body (`store::append`),
/// without opening an editor. Report what happened via `output::success`.
/// Ambiguity/NotFound propagate as errors; the caller renders ambiguous matches.
pub fn run(title: &str, message: &str) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    store::append(&memo, message)?;
    output::success(&style, &format!("appended to '{}'", memo.title));
    Ok(())
}
