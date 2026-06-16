//! `stpl del <title> [-y]` — delete a memo after confirmation.
//!
//! CONTRACT — implement `run`; do not change its signature.

use std::io::{IsTerminal, Write};

use anyhow::{anyhow, Result};

use crate::{commands::util, memo::MemoKind, output, store};

/// Fuzzy-resolve `title`, then confirm `Delete <title>[...]? [y/N]` on stdin
/// unless `yes` is set. For projects, make clear the whole directory is
/// removed. If not a TTY and `yes` is false, abort safely. On confirmation,
/// `store::delete` and report via `output::success`.
pub fn run(title: &str, yes: bool) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_or_show(&config, &style, title)?;

    let line = style.memo_line(&memo);
    let is_project = memo.kind == MemoKind::Project;

    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err(anyhow!(
                "refusing to delete without confirmation; pass -y/--yes to delete non-interactively"
            ));
        }

        // Make destructive scope explicit for projects.
        let prompt = if is_project {
            let dir = memo
                .path
                .parent()
                .unwrap_or(&memo.path)
                .display()
                .to_string();
            format!("Delete project {line} and its entire directory {dir}? [y/N] ")
        } else {
            format!("Delete {line}? [y/N] ")
        };

        anstream::print!("{prompt}");
        std::io::stdout().flush().ok();

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_ascii_lowercase();
        if answer != "y" && answer != "yes" {
            output::success(&style, "aborted; nothing deleted");
            return Ok(());
        }
    }

    store::delete(&memo)?;
    let what = if is_project { "project" } else { "memo" };
    output::success(&style, &format!("deleted {what} {}", memo.path.display()));
    Ok(())
}
