//! `stpl tag <title> <tags>...` — add tags to a memo.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, output, store};

/// Fuzzy-resolve `title` and add `tags` to its frontmatter (`store::add_tags`),
/// ignoring duplicates. Report the resulting tag list via `output::success`.
/// Ambiguity/NotFound propagate as errors.
pub fn run(title: &str, tags: &[String]) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    let all = store::add_tags(&memo, tags)?;
    output::success(
        &style,
        &format!("tagged '{}' — tags: [{}]", memo.title, all.join(", ")),
    );
    Ok(())
}
