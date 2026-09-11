//! Domain error types.

use std::path::PathBuf;

use thiserror::Error;

use crate::{diary::Diary, memo::Memo};

#[derive(Error, Debug)]
pub enum StplError {
    /// No memo matched the query.
    #[error("no memo matches '{0}'")]
    NotFound(String),

    /// Multiple memos matched the query; the caller should list `matches`.
    #[error("multiple memos match '{query}' — be more specific")]
    Ambiguous { query: String, matches: Vec<Memo> },

    /// A command that allows an implicit target was run with no title and no
    /// remembered memo (or the remembered memo is gone).
    #[error(
        "no memo given and no recent memo remembered — pass a title, or create one with `stpl new <title>`"
    )]
    NoLastMemo,

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

    /// No diary matched the query.
    #[error("no diary matches '{0}'")]
    DiaryNotFound(String),

    /// Multiple diaries matched the query; the caller should list `matches`.
    #[error("multiple diaries match '{query}' — be more specific")]
    AmbiguousDiary { query: String, matches: Vec<Diary> },

    /// A diary command that allows an implicit target was run with no name and
    /// no remembered diary (or the remembered diary is gone).
    #[error(
        "no diary given and no recent diary remembered — pass a name, or add an entry with `stpl diary <name> -m \"text\"`"
    )]
    NoLastDiary,

    /// `git pull` left unmerged paths that need manual resolution.
    #[error(
        "git pull produced merge conflicts in '{0}' — resolve them, then run `stpl sync` again"
    )]
    MergeConflict(PathBuf),
}
