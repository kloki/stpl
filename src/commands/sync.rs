//! `stpl sync` — commit, pull, and push the (git-backed) memo directory.
//!
//! CONTRACT — implement `run`; do not change its signature.

use std::{
    path::Path,
    process::{Command, Output},
};

use anyhow::{Result, anyhow};
use chrono::Local;

use crate::{commands::util, error::StplError, output};

/// Synchronize the memo directory with its git remote.
///
/// Flow (commit → pull → push):
/// 1. Refuse with a setup guide if the memo directory isn't a git repo.
/// 2. `git add -A`; commit with a timestamped message if anything is staged.
/// 3. `git pull`; on merge conflicts, explain and abort before pushing.
/// 4. `git push`.
pub fn run() -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let dir = config.memo_directory.clone();

    // 1. Must be a git repository (and thus must exist).
    if !dir.join(".git").exists() {
        print_setup_guide(&dir);
        return Err(StplError::NotAGitRepo(dir).into());
    }

    // 2. Stage everything, commit only if there's something staged.
    git_check(&dir, &["add", "-A"])?;
    if has_staged_changes(&dir)? {
        let message = format!("stpl sync: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
        git_check(&dir, &["commit", "-m", &message])?;
        output::success(&style, &format!("committed local changes ({message})"));
    } else {
        output::success(&style, "nothing to commit");
    }

    // 3. Pull. `--no-rebase` forces a merge so the outcome doesn't depend on
    //    the user's `pull.rebase` config: clean divergence auto-merges, while
    //    real conflicts leave unmerged paths we detect below. Any other failure
    //    surfaces git's own stderr.
    let pull = git_capture(&dir, &["pull", "--no-rebase"])?;
    if !pull.status.success() {
        if has_unmerged_paths(&dir)? {
            return Err(StplError::MergeConflict(dir).into());
        }
        return Err(output_error("git pull", &pull));
    }
    output::success(&style, "pulled from remote");

    // 4. Push.
    git_check(&dir, &["push"])?;
    output::success(&style, "pushed to remote");

    Ok(())
}

/// Whether `git diff --cached --quiet` reports staged changes (exit code 1).
fn has_staged_changes(dir: &Path) -> Result<bool> {
    // `--quiet` exits 0 when there is no diff, 1 when there is one.
    let out = git_capture(dir, &["diff", "--cached", "--quiet"])?;
    Ok(!out.status.success())
}

/// Whether the index has unmerged (conflicted) paths.
fn has_unmerged_paths(dir: &Path) -> Result<bool> {
    let out = git_capture(dir, &["ls-files", "--unmerged"])?;
    Ok(!out.stdout.is_empty())
}

/// Run `git -C <dir> <args>` capturing stdout/stderr.
fn git_capture(dir: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| anyhow!("failed to run git (is it installed and on $PATH?): {e}"))
}

/// Run a git command and turn a non-zero exit into an error carrying git's
/// stderr. Use for steps where any failure should abort the sync.
fn git_check(dir: &Path, args: &[&str]) -> Result<()> {
    let out = git_capture(dir, args)?;
    if !out.status.success() {
        return Err(output_error(&format!("git {}", args.join(" ")), &out));
    }
    Ok(())
}

/// Build an error from a failed command's output, including its trimmed stderr.
fn output_error(label: &str, out: &Output) -> anyhow::Error {
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        anyhow!("{label} failed ({})", out.status)
    } else {
        anyhow!("{label} failed: {stderr}")
    }
}

/// Print the one-time setup instructions when the memo directory isn't a repo.
fn print_setup_guide(dir: &Path) {
    let dir = dir.display();
    anstream::eprintln!(
        "'{dir}' is not a git repository.\n\
         \n\
         `stpl sync` keeps your notes in sync through a git remote (e.g. GitHub).\n\
         Set it up once:\n\
         \n\
         \x20 cd {dir}\n\
         \x20 git init\n\
         \x20 git remote add origin <your-repo-url>\n\
         \x20 git add -A && git commit -m \"initial notes\"\n\
         \x20 git push -u origin main\n\
         \n\
         Then run `stpl sync` again."
    );
}
