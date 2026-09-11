//! Command-line interface definition (clap derive).

use clap::{Args, Parser, Subcommand, ValueEnum};

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
        /// Title to fuzzy-match. Defaults to the last memo you used.
        title: Option<String>,
    },

    /// Print a memo's absolute path (useful for scripting/agentic use).
    Path {
        /// Title to fuzzy-match. Defaults to the last memo you used.
        title: Option<String>,
        /// Print the memo's containing folder instead of the file path.
        #[arg(short = 'd', long = "dir")]
        dir: bool,
    },

    /// Print a memo's contents to stdout (no decoration; pipe-friendly).
    Show {
        /// Title to fuzzy-match. Defaults to the last memo you used.
        title: Option<String>,
        /// Omit the YAML frontmatter, printing only the body.
        #[arg(long = "no-frontmatter")]
        no_frontmatter: bool,
    },

    /// Append a line to an existing memo (no editor).
    Append {
        /// Title to fuzzy-match. Defaults to the last memo you used.
        title: Option<String>,
        /// Text to append.
        #[arg(short = 'm', long = "message")]
        message: String,
    },

    /// Rename a memo — re-slug and move it, keeping its date/folder.
    Rename {
        /// Title to fuzzy-match.
        title: String,
        /// The new title.
        new_title: String,
    },

    /// Full-text search across memo bodies.
    Search {
        /// Text to search for (case-insensitive substring).
        query: String,
        /// Output format (default `text`).
        #[arg(short = 'f', long = "format", value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Only search memos on or after this date (YYYY-MM-DD).
        #[arg(short = 'a', long = "after")]
        after: Option<String>,
        /// Only search memos on or before this date (YYYY-MM-DD).
        #[arg(short = 'b', long = "before")]
        before: Option<String>,
        /// Only search memos that have at least one of these tags (repeatable).
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },

    /// Commit, pull, and push the memo directory (git-backed).
    Sync,

    /// Delete a memo (asks for confirmation).
    Del {
        /// Title to fuzzy-match. Defaults to the last memo you used.
        title: Option<String>,
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },

    /// Expand a memo into a project directory.
    Expand {
        /// Title to fuzzy-match. Defaults to the last memo you used.
        title: Option<String>,
    },

    /// Add one or more tags to a memo (duplicates are ignored).
    Tag {
        /// Title to fuzzy-match.
        title: String,
        /// Tag(s) to add.
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// Remove one or more tags from a memo (case-insensitive; missing tags ignored).
    Untag {
        /// Title to fuzzy-match.
        title: String,
        /// Tag(s) to remove.
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// List all tags with their memo counts.
    Tags {
        /// Output format (default `text`).
        #[arg(value_enum, default_value_t = Format::Text)]
        format: Format,
    },

    /// Add a timestamped entry to a diary, or manage diaries.
    Diary(DiaryArgs),

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

/// `stpl diary [name] [-m text]` plus the management subcommands.
///
/// `args_conflicts_with_subcommands` keeps `stpl diary list -m x` an error
/// instead of silently ignoring one half. Nothing here is required, so
/// `subcommand_negates_reqs` is unnecessary.
#[derive(Args, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct DiaryArgs {
    #[command(subcommand)]
    pub command: Option<DiaryCommand>,

    /// Diary name (created on first use with -m). Defaults to the last diary
    /// you used. For a diary named like a subcommand, put it after `--`
    /// (`stpl diary -m "text" -- list`).
    pub name: Option<String>,

    /// Entry text to append. Without it, the diary opens in $EDITOR.
    #[arg(short = 'm', long = "message")]
    pub message: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum DiaryCommand {
    /// List all diaries with their entry counts.
    List {
        /// Output format (default `text`).
        #[arg(value_enum, default_value_t = Format::Text)]
        format: Format,
    },

    /// Print a diary's contents to stdout (no decoration; pipe-friendly).
    Show {
        /// Name to fuzzy-match. Defaults to the last diary you used.
        name: Option<String>,
    },

    /// Open a diary in $EDITOR.
    Edit {
        /// Name to fuzzy-match. Defaults to the last diary you used.
        name: Option<String>,
    },

    /// Print a diary's absolute path (useful for scripting/agentic use).
    Path {
        /// Name to fuzzy-match. Defaults to the last diary you used.
        name: Option<String>,
    },

    /// Delete a diary and all its entries (asks for confirmation).
    Del {
        /// Name to fuzzy-match. Defaults to the last diary you used.
        name: Option<String>,
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
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
    /// Markdown rendered to a file and opened in $EDITOR.
    Editor,
}
