//! `stpl path <title>` — print a memo's absolute path and nothing else.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::commands::util;

/// Fuzzy-resolve `title` (`resolve::resolve_one`) and print the resolved memo's
/// absolute path to stdout. Intended for scripting/agentic use, so the output is
/// just the bare path with no decoration. With `dir`, print the memo's containing
/// folder instead (the week folder for a file, the project directory for a
/// project). Ambiguity/NotFound propagate as errors; the caller renders ambiguous
/// matches.
pub fn run(title: &str, dir: bool) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    let path = if dir {
        memo.path.parent().unwrap_or(&memo.path)
    } else {
        &memo.path
    };
    println!("{}", path.display());
    Ok(())
}
