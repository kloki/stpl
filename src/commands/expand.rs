//! `stpl expand <title>` — turn a memo into a project directory.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, output, store};

/// Fuzzy-resolve `title` to a file memo and `store::expand` it into a project
/// (`<stem>/project.md`). Report the new path via `output::success`. Collision
/// or "already a project" propagate as errors.
pub fn run(title: &str) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    let newpath = store::expand(&memo)?;
    output::success(
        &style,
        &format!("expanded into project {}", newpath.display()),
    );
    Ok(())
}
