//! Domain error types.

use std::path::PathBuf;

use thiserror::Error;

use crate::memo::Memo;

#[derive(Error, Debug)]
pub enum StplError {
    /// No memo matched the query.
    #[error("no memo matches '{0}'")]
    NotFound(String),

    /// Multiple memos matched the query; the caller should list `matches`.
    #[error("multiple memos match '{query}' — be more specific")]
    Ambiguous { query: String, matches: Vec<Memo> },

    /// A target path already exists (e.g. `expand` collision, same-day memo).
    #[error("'{0}' already exists")]
    Collision(PathBuf),

    /// No editor could be determined.
    #[error("no editor found — set $EDITOR or $VISUAL")]
    NoEditor,

    /// The config file already exists.
    #[error("config already exists at '{0}'")]
    ConfigExists(PathBuf),

    /// The provided title was empty or produced an empty slug.
    #[error("invalid title: '{0}'")]
    InvalidTitle(String),

    /// A supplied date could not be parsed.
    #[error("invalid date '{0}' (expected YYYY-MM-DD)")]
    InvalidDate(String),

    /// The memo directory is not a git repository (needed by `stpl sync`).
    #[error("'{0}' is not a git repository")]
    NotAGitRepo(PathBuf),

    /// `git pull` left unmerged paths that need manual resolution.
    #[error(
        "git pull produced merge conflicts in '{0}' — resolve them, then run `stpl sync` again"
    )]
    MergeConflict(PathBuf),
}
