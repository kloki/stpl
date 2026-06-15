//! Command-line interface definition (clap derive).

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "stpl", version, about = "staple — quick markdown notes/memos")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create the config file (~/.config/stpl.toml).
    Init,

    /// Create a new memo with the given title.
    New {
        /// Title of the memo.
        title: String,
        /// Memo content. If omitted, the memo opens in $EDITOR.
        #[arg(short = 'm', long = "message")]
        message: Option<String>,
    },

    /// Open a memo in $EDITOR.
    Edit {
        /// Title to fuzzy-match.
        title: String,
    },

    /// Print a memo's absolute path (useful for scripting/agentic use).
    Path {
        /// Title to fuzzy-match.
        title: String,
    },

    /// Commit, pull, and push the memo directory (git-backed).
    Sync,

    /// Delete a memo (asks for confirmation).
    Del {
        /// Title to fuzzy-match.
        title: String,
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Expand a memo into a project directory.
    Expand {
        /// Title to fuzzy-match.
        title: String,
    },

    /// Add one or more tags to a memo (duplicates are ignored).
    Tag {
        /// Title to fuzzy-match.
        title: String,
        /// Tag(s) to add.
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// Print an overview of memos grouped by folder.
    Overview {
        /// Output format (default `text`).
        #[arg(value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Only show memos on or after this date (YYYY-MM-DD).
        #[arg(short = 'a', long = "after")]
        after: Option<String>,
        /// Only show memos on or before this date (YYYY-MM-DD).
        #[arg(short = 'b', long = "before")]
        before: Option<String>,
        /// Only show memos that have at least one of these tags (repeatable).
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Plain text, optimized for agents (default).
    Text,
    /// JSON.
    Json,
    /// Nicely formatted markdown.
    Markdown,
    /// Verbose markdown: each memo as a section with a content preview.
    MarkdownVerbose,
    /// Markdown rendered to a file and opened in $EDITOR.
    Editor,
}
