//! Diaries: append-only, timestamped logs.
//!
//! A diary is a single markdown file at `<memo_dir>/diaries/<slug>.md` holding
//! a run of entries, each introduced by an ISO timestamp heading:
//!
//! ```text
//! # 2026-09-11T09:15
//!
//! Standup: blocked on CI.
//!
//! # 2026-09-11T14:32
//!
//! CI fixed, merged the PR.
//! ```
//!
//! Unlike memos there is no frontmatter, no date in the filename, and no
//! `<year>/<week>` tree — a diary is long-lived, so it lives outside the dated
//! memo layout. `store::list_all` prunes `diaries/` for exactly that reason.
//!
//! This module owns the diary model, the pure entry formatting/parsing helpers,
//! and the diary filesystem operations (the memo equivalents live in `store`).
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use serde::Serialize;

use crate::{
    config::Config,
    error::StplError,
    memo,
    resolve::{self, Pick},
};

/// Name of the directory holding every diary, directly under the memo root.
pub const DIARIES_DIR: &str = "diaries";

/// Heading timestamp format: ISO-8601 local date/time to the minute. Minute
/// precision keeps several entries a day visually distinct without noise.
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M";

/// One diary: a named append-only log file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diary {
    /// Display name (slug with `-` -> space, title-cased).
    pub name: String,
    /// Lower-kebab slug; the file stem.
    pub slug: String,
    /// Absolute path to the `.md` file.
    pub path: PathBuf,
}

/// One entry within a diary file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The heading's timestamp.
    pub timestamp: NaiveDateTime,
    /// 1-based line number of the `# <timestamp>` heading within the file.
    pub line: usize,
    /// The entry's text, with surrounding blank lines trimmed.
    pub body: String,
}

impl Diary {
    /// The diary called `name` under `memo_dir`. Slugifies the name; does not
    /// touch the filesystem, so this is also how a new diary is addressed.
    pub fn new(memo_dir: &Path, name: &str) -> Result<Diary, StplError> {
        let slug = memo::slugify(name)?;
        Ok(Diary {
            name: memo::title_from_slug(&slug),
            path: diary_path(memo_dir, &slug),
            slug,
        })
    }

    /// Parse `<...>/diaries/<slug>.md`. Returns `None` for anything else — a
    /// non-`.md` file, an empty stem, or a file outside a `diaries` directory —
    /// so callers can skip foreign files silently.
    pub fn from_path(path: &Path) -> Option<Diary> {
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            return None;
        }
        if path.parent()?.file_name()? != DIARIES_DIR {
            return None;
        }
        let slug = path.file_stem()?.to_str()?;
        if slug.is_empty() {
            return None;
        }
        Some(Diary {
            name: memo::title_from_slug(slug),
            slug: slug.to_string(),
            path: path.to_path_buf(),
        })
    }

    /// Whether the diary file exists on disk yet.
    pub fn exists(&self) -> bool {
        self.path.is_file()
    }
}

/// `<memo_dir>/diaries`.
pub fn diaries_dir(memo_dir: &Path) -> PathBuf {
    memo_dir.join(DIARIES_DIR)
}

/// `<memo_dir>/diaries/<slug>.md`.
pub fn diary_path(memo_dir: &Path, slug: &str) -> PathBuf {
    diaries_dir(memo_dir).join(format!("{slug}.md"))
}

/// Render one entry: a timestamp heading, a blank line, then the text,
/// ending in exactly one newline.
pub fn format_entry(now: NaiveDateTime, text: &str) -> String {
    format!(
        "# {}\n\n{}\n",
        now.format(TIMESTAMP_FORMAT),
        text.trim_end_matches('\n')
    )
}

/// `content` with a new entry appended, separated from what came before by one
/// blank line. An empty (or whitespace-only) file starts directly with the
/// heading, so a fresh diary has no leading blank line.
pub fn append_entry(content: &str, now: NaiveDateTime, text: &str) -> String {
    if content.trim().is_empty() {
        return format_entry(now, text);
    }
    let mut out = content.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&format_entry(now, text));
    out
}

/// The timestamp of an entry heading, or `None` for any other line.
///
/// The single source of truth for what counts as a heading — shared by
/// [`parse_entries`] and `stpl search`. Deliberately strict: `# Notes` and
/// `## 2026-09-11T14:32` are body text, not entry boundaries.
pub fn parse_heading(line: &str) -> Option<NaiveDateTime> {
    let rest = line.strip_prefix("# ")?;
    NaiveDateTime::parse_from_str(rest.trim(), TIMESTAMP_FORMAT).ok()
}

/// Split a diary file into its entries, in file order.
///
/// Lines before the first heading are ignored: a hand-edited diary may carry a
/// preamble, and it belongs to no entry.
pub fn parse_entries(content: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for (i, line) in content.lines().enumerate() {
        match parse_heading(line) {
            Some(timestamp) => entries.push(Entry {
                timestamp,
                line: i + 1,
                body: String::new(),
            }),
            None => {
                if let Some(entry) = entries.last_mut() {
                    entry.body.push_str(line);
                    entry.body.push('\n');
                }
            }
        }
    }
    for entry in &mut entries {
        entry.body = entry.body.trim_matches('\n').to_string();
    }
    entries
}

/// Every diary, sorted by slug.
///
/// Returns an empty vec (NOT an error) when the `diaries/` directory does not
/// exist. The listing is non-recursive: subdirectories are not diaries.
pub fn list_all(config: &Config) -> Result<Vec<Diary>> {
    let dir = diaries_dir(&config.memo_directory);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut diaries = Vec::new();
    let entries =
        fs::read_dir(&dir).with_context(|| format!("reading directory '{}'", dir.display()))?;
    for entry in entries.filter_map(|e| e.ok()) {
        if !entry.path().is_file() {
            continue;
        }
        if let Some(diary) = Diary::from_path(&entry.path()) {
            diaries.push(diary);
        }
    }
    diaries.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(diaries)
}

/// Append a timestamped entry, creating `diaries/` and the file if needed.
pub fn add_entry(diary: &Diary, now: NaiveDateTime, text: &str) -> Result<()> {
    if let Some(parent) = diary.path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory '{}'", parent.display()))?;
    }
    // A missing file is an empty diary, so its first entry needs no special case.
    let content = fs::read_to_string(&diary.path).unwrap_or_default();
    let updated = append_entry(&content, now, text);
    fs::write(&diary.path, updated)
        .with_context(|| format!("writing diary '{}'", diary.path.display()))?;
    Ok(())
}

/// Read a diary's full contents.
pub fn read_content(diary: &Diary) -> Result<String> {
    fs::read_to_string(&diary.path).with_context(|| format!("reading '{}'", diary.path.display()))
}

/// Delete a diary file.
pub fn delete(diary: &Diary) -> Result<()> {
    fs::remove_file(&diary.path).with_context(|| format!("deleting '{}'", diary.path.display()))?;
    Ok(())
}

/// Resolve a free-form `query` to exactly one existing diary, using the same
/// exact-then-fuzzy algorithm as memo titles ([`resolve::pick`]).
///
/// Note that adding an entry does NOT go through here: `stpl diary <name> -m`
/// takes the name exactly (slugified) so a typo creates a new diary rather than
/// silently appending to a similarly-named one.
pub fn resolve_one(config: &Config, query: &str) -> Result<Diary, StplError> {
    let diaries = list_all(config).map_err(|_| StplError::DiaryNotFound(query.to_string()))?;

    match resolve::pick(&diaries, query, |d| &d.name, |d| &d.slug) {
        Pick::One(diary) => Ok(diary),
        Pick::NotFound => Err(StplError::DiaryNotFound(query.to_string())),
        Pick::Ambiguous(matches) => Err(StplError::AmbiguousDiary {
            query: query.to_string(),
            matches,
        }),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("stpl-diary-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn config_for(dir: &Path) -> Config {
        Config {
            memo_directory: dir.to_path_buf(),
            disable_color: true,
        }
    }

    #[test]
    fn diary_path_lives_under_diaries_dir() {
        let root = Path::new("/home/u/stpls");
        assert_eq!(diaries_dir(root), PathBuf::from("/home/u/stpls/diaries"));
        assert_eq!(
            diary_path(root, "work"),
            PathBuf::from("/home/u/stpls/diaries/work.md")
        );
    }

    #[test]
    fn new_slugifies_the_name() {
        let diary = Diary::new(Path::new("/home/u/stpls"), "Work Log").unwrap();
        assert_eq!(diary.slug, "work-log");
        assert_eq!(diary.name, "Work Log");
        assert_eq!(
            diary.path,
            PathBuf::from("/home/u/stpls/diaries/work-log.md")
        );
    }

    #[test]
    fn new_rejects_an_empty_slug() {
        assert!(Diary::new(Path::new("/home/u/stpls"), "   ").is_err());
    }

    #[test]
    fn from_path_requires_md_under_a_diaries_dir() {
        assert!(Diary::from_path(Path::new("/s/diaries/work.md")).is_some());
        // Not a diary: wrong extension, wrong parent, or no stem.
        assert!(Diary::from_path(Path::new("/s/diaries/work.txt")).is_none());
        assert!(Diary::from_path(Path::new("/s/2026/24/2026-06-14-x.md")).is_none());
        assert!(Diary::from_path(Path::new("/s/diaries/.md")).is_none());
    }

    #[test]
    fn format_entry_renders_heading_blank_line_and_text() {
        assert_eq!(
            format_entry(at(2026, 9, 11, 14, 32), "fixed CI"),
            "# 2026-09-11T14:32\n\nfixed CI\n"
        );
    }

    #[test]
    fn format_entry_keeps_multiline_text_and_trims_trailing_newlines() {
        assert_eq!(
            format_entry(at(2026, 9, 11, 14, 32), "one\ntwo\n\n"),
            "# 2026-09-11T14:32\n\none\ntwo\n"
        );
    }

    #[test]
    fn append_entry_to_empty_file_has_no_leading_blank_line() {
        let out = append_entry("", at(2026, 9, 11, 9, 15), "first");
        assert_eq!(out, "# 2026-09-11T09:15\n\nfirst\n");
        // Whitespace-only counts as empty too.
        assert_eq!(out, append_entry("\n  \n", at(2026, 9, 11, 9, 15), "first"));
    }

    #[test]
    fn append_entry_separates_entries_with_one_blank_line() {
        let first = append_entry("", at(2026, 9, 11, 9, 15), "first");
        let both = append_entry(&first, at(2026, 9, 11, 14, 32), "second");
        assert_eq!(
            both,
            "# 2026-09-11T09:15\n\nfirst\n\n# 2026-09-11T14:32\n\nsecond\n"
        );
    }

    #[test]
    fn append_entry_normalizes_a_missing_trailing_newline() {
        let out = append_entry("# 2026-09-11T09:15\n\nfirst", at(2026, 9, 11, 14, 32), "x");
        assert_eq!(
            out,
            "# 2026-09-11T09:15\n\nfirst\n\n# 2026-09-11T14:32\n\nx\n"
        );
    }

    #[test]
    fn parse_heading_is_strict() {
        assert_eq!(
            parse_heading("# 2026-09-11T14:32"),
            Some(at(2026, 9, 11, 14, 32))
        );
        assert!(parse_heading("## 2026-09-11T14:32").is_none());
        assert!(parse_heading("# 2026-09-11T14:32:07").is_none());
        assert!(parse_heading("# Notes").is_none());
        assert!(parse_heading("#2026-09-11T14:32").is_none());
    }

    #[test]
    fn parse_entries_round_trips_two_entries() {
        let content = "# 2026-09-11T09:15\n\nfirst\n\n# 2026-09-11T14:32\n\nsecond\nline two\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp, at(2026, 9, 11, 9, 15));
        assert_eq!(entries[0].line, 1);
        assert_eq!(entries[0].body, "first");
        assert_eq!(entries[1].timestamp, at(2026, 9, 11, 14, 32));
        assert_eq!(entries[1].line, 5);
        assert_eq!(entries[1].body, "second\nline two");
    }

    #[test]
    fn parse_entries_ignores_preamble_and_non_timestamp_headings() {
        let content = "some preamble\n\n# 2026-09-11T09:15\n\n# Notes\nbody\n";
        let entries = parse_entries(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].line, 3);
        assert_eq!(entries[0].body, "# Notes\nbody");
    }

    #[test]
    fn add_entry_creates_then_appends_and_list_delete_round_trip() {
        let dir = temp_dir("round-trip");
        let config = config_for(&dir);

        // Nothing yet: no diaries/ directory at all.
        assert!(list_all(&config).unwrap().is_empty());

        let diary = Diary::new(&dir, "Work").unwrap();
        assert!(!diary.exists());
        add_entry(&diary, at(2026, 9, 11, 9, 15), "first").unwrap();
        assert!(diary.exists());
        add_entry(&diary, at(2026, 9, 11, 14, 32), "second").unwrap();

        assert_eq!(
            read_content(&diary).unwrap(),
            "# 2026-09-11T09:15\n\nfirst\n\n# 2026-09-11T14:32\n\nsecond\n"
        );

        let listed = list_all(&config).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].slug, "work");
        assert_eq!(listed[0].name, "Work");

        delete(&diary).unwrap();
        assert!(list_all(&config).unwrap().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_one_matches_exactly_then_fuzzily_and_reports_ambiguity() {
        let dir = temp_dir("resolve");
        let config = config_for(&dir);
        for name in ["Work", "Work Log", "Travel"] {
            let diary = Diary::new(&dir, name).unwrap();
            add_entry(&diary, at(2026, 9, 11, 9, 15), "x").unwrap();
        }

        // Exact name/slug beats the fuzzy scores.
        assert_eq!(resolve_one(&config, "work").unwrap().slug, "work");
        assert_eq!(resolve_one(&config, "work-log").unwrap().slug, "work-log");
        // A clear fuzzy winner.
        assert_eq!(resolve_one(&config, "trav").unwrap().slug, "travel");
        // Nothing close.
        assert!(matches!(
            resolve_one(&config, "zzzz"),
            Err(StplError::DiaryNotFound(_))
        ));

        let _ = fs::remove_dir_all(&dir);
    }
}
