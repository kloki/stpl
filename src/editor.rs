//! Launching the user's editor.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::{
    env,
    io::{self, IsTerminal},
    path::Path,
    process::Command,
};

use anyhow::{Result, anyhow};

use crate::error::StplError;

/// Determine the editor command: `$EDITOR`, then `$VISUAL`, then `vi`.
/// Returns the command string, or `None` only if a deliberate "no editor"
/// state should be surfaced (implementations may always return `Some("vi")`).
pub fn editor_command() -> Option<String> {
    if let Some(editor) = env_nonempty("EDITOR") {
        return Some(editor);
    }
    if let Some(visual) = env_nonempty("VISUAL") {
        return Some(visual);
    }
    Some("vi".to_string())
}

/// Read an environment variable, treating empty/whitespace-only as unset.
fn env_nonempty(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Open `path` in the user's editor, inheriting stdio, and wait for it to
/// exit. Errors with `StplError::NoEditor` if none is available and stdin is
/// not a TTY; surfaces a non-zero editor exit as an error.
pub fn open(path: &Path) -> Result<()> {
    let cmd = match editor_command() {
        Some(cmd) => cmd,
        None => return Err(StplError::NoEditor.into()),
    };

    // Without a TTY there is no interactive editor to fall into, so refuse
    // rather than spawning a terminal editor that would immediately fail.
    if !io::stdin().is_terminal() {
        return Err(StplError::NoEditor.into());
    }

    let mut parts = cmd.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow!("empty editor command"))?;

    let status = Command::new(program)
        .args(parts)
        .arg(path)
        .status()
        .map_err(|e| anyhow!("failed to launch editor '{cmd}': {e}"))?;

    if !status.success() {
        return Err(anyhow!("editor '{cmd}' exited with {status}"));
    }
    Ok(())
}
