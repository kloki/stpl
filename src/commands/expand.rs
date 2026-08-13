//! `stpl expand [title]` — turn a memo into a project directory.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, output, state, store};

/// Fuzzy-resolve `title` to a file memo (or take the last memo used, with no
/// `title`) and `store::expand` it into a project (`<stem>/project.md`). Report
/// the new path via `output::success`. Collision or "already a project"
/// propagate as errors.
pub fn run(title: Option<&str>) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_last(&config, &style, title)?;
    let newpath = store::expand(&memo)?;
    // The memo moved from `<stem>.md` to `<stem>/project.md`; follow it.
    state::record(&newpath);
    output::success(
        &style,
        &format!("expanded into project {}", newpath.display()),
    );
    Ok(())
}
