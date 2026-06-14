//! `stpl path <title>` — print a memo's absolute path and nothing else.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::commands::util;

/// Fuzzy-resolve `title` (`resolve::resolve_one`) and print the resolved memo's
/// absolute path to stdout. Intended for scripting/agentic use, so the output is
/// just the bare path with no decoration. Ambiguity/NotFound propagate as errors;
/// the caller renders ambiguous matches.
pub fn run(title: &str) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    println!("{}", memo.path.display());
    Ok(())
}
