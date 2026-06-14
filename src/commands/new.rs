//! `stpl new <title> [-m <content>]` — create a new memo.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{commands::util, editor, memo, output, store};

/// Create a new memo dated today. Slugify `title`, lazily create the
/// `<year>/<week>` dirs, and write the template (`store::create`).
/// With `content` (`-m`): write and report the path via `output::success`.
/// Without it: create the file then open it in `$EDITOR` (`editor::open`).
pub fn run(title: &str, content: Option<&str>) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let slug = memo::slugify(title)?;
    let date = chrono::Local::now().date_naive();
    let path = store::create(&config, date, &slug, title, content)?;

    match content {
        Some(_) => {
            output::success(&style, &format!("created {}", path.display()));
        }
        None => {
            // No body given: drop the user into their editor on the new file.
            editor::open(&path)?;
        }
    }
    Ok(())
}
