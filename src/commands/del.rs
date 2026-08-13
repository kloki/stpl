//! `stpl del [title] [-y]` — delete a memo after confirmation.
//!
//! CONTRACT — implement `run`; do not change its signature.

use std::io::{IsTerminal, Write};

use anyhow::{Result, anyhow};

use crate::{commands::util, memo::MemoKind, output, state, store};

/// Fuzzy-resolve `title` (or the last memo used, with no `title`), then confirm
/// `Delete <title>[...]? [y/N]` on stdin unless `yes` is set. For projects, make
/// clear the whole directory is removed. If not a TTY and `yes` is false, abort
/// safely. On confirmation, `store::delete`, forget the memo if it was the
/// remembered one, and report via `output::success`.
///
/// Resolves via `resolve_target` rather than `resolve_or_last`: recording a memo
/// we are about to delete would clear the pointer to whatever was remembered
/// before, so deleting some other memo would lose it.
pub fn run(title: Option<&str>, yes: bool) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let memo = util::resolve_target(&config, &style, title)?;

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
    // The memo is gone; don't leave the pointer aimed at it.
    state::clear_if(&memo.path);
    let what = if is_project { "project" } else { "memo" };
    output::success(&style, &format!("deleted {what} {}", memo.path.display()));
    Ok(())
}
