//! `stpl rename <title> <new-title>` — rename a memo.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, output, store};

/// Fuzzy-resolve `title` and rename the resolved memo to `new_title`
/// (`store::rename`): re-slug, rewrite the in-file title, and move the file (or
/// project directory), keeping its date/folder. Report the new path via
/// `output::success`. Ambiguity/NotFound/Collision propagate as errors.
pub fn run(title: &str, new_title: &str) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    let (new_path, _slug) = store::rename(&memo, new_title)?;
    output::success(
        &style,
        &format!(
            "renamed '{}' -> '{}' ({})",
            memo.title,
            new_title,
            new_path.display()
        ),
    );
    Ok(())
}
