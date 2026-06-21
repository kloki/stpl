//! `stpl untag <title> <tags>...` — remove tags from a memo.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, output, store};

/// Fuzzy-resolve `title` and remove `tags` from its frontmatter
/// (`store::remove_tags`, case-insensitive; missing tags ignored). Report the
/// remaining tag list via `output::success`. Ambiguity/NotFound propagate as
/// errors.
pub fn run(title: &str, tags: &[String]) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    let remaining = store::remove_tags(&memo, tags)?;
    output::success(
        &style,
        &format!(
            "untagged '{}' — tags: [{}]",
            memo.title,
            remaining.join(", ")
        ),
    );
    Ok(())
}
