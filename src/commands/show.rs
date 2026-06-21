//! `stpl show <title>` — print a memo's contents to stdout.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, memo, store};

/// Fuzzy-resolve `title` and print the resolved memo's contents to stdout with
/// no decoration (intended for piping / agentic reads). With `no_frontmatter`,
/// the leading YAML frontmatter block is stripped and only the body is printed.
/// Ambiguity/NotFound propagate as errors; the caller renders ambiguous matches.
pub fn run(title: &str, no_frontmatter: bool) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;
    let content = store::read_content(&memo)?;
    let body = if no_frontmatter {
        memo::strip_frontmatter(&content)
    } else {
        &content
    };
    print!("{body}");
    if !body.ends_with('\n') {
        println!();
    }
    Ok(())
}
