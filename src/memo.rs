//! The `Memo` data model plus path<->memo conversion and slug helpers.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate};

use crate::error::StplError;

/// Whether a memo is a single file or an expanded project directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoKind {
    /// A `<iso_date>-<slug>.md` file.
    File,
    /// A `<iso_date>-<slug>/project.md` project directory.
    Project,
}

/// A single memo discovered on disk.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Memo {
    /// Human-readable display title (slug with `-`→space, title-cased).
    pub title: String,
    /// Filename slug (lower-kebab).
    pub slug: String,
    /// Date parsed from the filename/dir prefix.
    pub date: NaiveDate,
    /// ISO week-numbering year (folder name). See module note below.
    pub year: i32,
    /// ISO week number (folder name).
    pub week: u32,
    /// Absolute path to the memo file (the `.md` itself, i.e. `project.md` for projects).
    pub path: PathBuf,
    /// File vs project.
    pub kind: MemoKind,
}

impl Memo {
    /// Parse a `.md` file path into a `Memo`. Returns `None` if the filename
    /// does not follow the `<iso_date>-<slug>.md` convention (caller should
    /// skip such files gracefully).
    ///
    /// - A file literally named `project.md` is a project: derive date/slug
    ///   from its parent directory name (`<iso_date>-<slug>`), kind = Project.
    /// - Any other `*.md` file: derive from its own stem, kind = File.
    pub fn from_path(path: &Path) -> Option<Memo> {
        // Must be a `.md` file.
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            return None;
        }

        let file_name = path.file_name().and_then(|n| n.to_str())?;

        // Determine the stem-bearing name and kind.
        let (stem, kind) = if file_name == "project.md" {
            // Derive from the parent directory name.
            let parent_name = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())?;
            (parent_name, MemoKind::Project)
        } else {
            let stem = path.file_stem().and_then(|s| s.to_str())?;
            (stem, MemoKind::File)
        };

        let (date, slug) = parse_stem(stem)?;

        Some(Memo {
            title: title_from_slug(&slug),
            slug,
            date,
            year: date.iso_week().year(),
            week: date.iso_week().week(),
            path: path.to_path_buf(),
            kind,
        })
    }
}

/// Parse a `<iso_date>-<slug>` stem into its date and slug components.
///
/// The first 10 chars must be `%Y-%m-%d`, followed by a `-`, then a non-empty
/// slug. Returns `None` on any parse failure.
fn parse_stem(stem: &str) -> Option<(NaiveDate, String)> {
    // `2026-06-14` is exactly 10 characters; we need at least that plus a `-`
    // and one slug character.
    if stem.len() < 12 {
        return None;
    }
    let (date_part, rest) = stem.split_at(10);
    let date = NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()?;
    // `rest` must begin with the separating `-` and have a slug after it.
    let slug = rest.strip_prefix('-')?;
    if slug.is_empty() {
        return None;
    }
    Some((date, slug.to_string()))
}

/// Convert a free-form title into a lower-kebab slug.
///
/// Lowercase, collapse runs of non-alphanumeric chars into a single `-`, trim
/// leading/trailing `-`. Returns `InvalidTitle` if the result is empty.
pub fn slugify(title: &str) -> Result<String, StplError> {
    let mut slug = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                slug.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        return Err(StplError::InvalidTitle(title.to_string()));
    }
    Ok(trimmed.to_string())
}

/// Derive a display title from a slug (`my-note` -> `My Note`).
pub fn title_from_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut s: String = first.to_uppercase().collect();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The `<year>/<week>` directory for a date, joined under `memo_dir`.
///
/// IMPORTANT: use `date.iso_week().year()` for the year (NOT `date.year()`)
/// and zero-pad the week to two digits, e.g. `<memo_dir>/2026/24`.
pub fn dir_for(memo_dir: &Path, date: NaiveDate) -> PathBuf {
    let iso = date.iso_week();
    memo_dir
        .join(iso.year().to_string())
        .join(format!("{:02}", iso.week()))
}

/// The bare file stem for a memo: `<iso_date>-<slug>` (no extension).
pub fn stem_for(date: NaiveDate, slug: &str) -> String {
    format!("{}-{}", date.format("%Y-%m-%d"), slug)
}

/// The absolute `.md` path for a (date, slug) memo file:
/// `<memo_dir>/<year>/<week>/<iso_date>-<slug>.md`.
pub fn memo_path(memo_dir: &Path, date: NaiveDate, slug: &str) -> PathBuf {
    dir_for(memo_dir, date).join(format!("{}.md", stem_for(date, slug)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("My Note").unwrap(), "my-note");
        assert_eq!(slugify("My First Note").unwrap(), "my-first-note");
    }

    #[test]
    fn slugify_collapses_and_trims() {
        assert_eq!(slugify("  Hello---World!!  ").unwrap(), "hello-world");
        assert_eq!(slugify("a / b / c").unwrap(), "a-b-c");
        assert_eq!(slugify("Café & Crème").unwrap(), "café-crème");
    }

    #[test]
    fn slugify_empty_errors() {
        assert!(matches!(slugify(""), Err(StplError::InvalidTitle(_))));
        assert!(matches!(slugify("!!!"), Err(StplError::InvalidTitle(_))));
        assert!(matches!(slugify("   "), Err(StplError::InvalidTitle(_))));
    }

    #[test]
    fn title_from_slug_titlecases() {
        assert_eq!(title_from_slug("my-note"), "My Note");
        assert_eq!(title_from_slug("my-first-note"), "My First Note");
        assert_eq!(title_from_slug("single"), "Single");
    }

    #[test]
    fn stem_and_path_building() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        assert_eq!(stem_for(date, "my-note"), "2026-06-14-my-note");

        let memo_dir = Path::new("/home/u/stpls");
        assert_eq!(
            dir_for(memo_dir, date),
            PathBuf::from("/home/u/stpls/2026/24")
        );
        assert_eq!(
            memo_path(memo_dir, date, "my-note"),
            PathBuf::from("/home/u/stpls/2026/24/2026-06-14-my-note.md")
        );
    }

    #[test]
    fn iso_week_year_boundary() {
        // 2024-12-31 is a Tuesday; ISO week 01 of week-numbering year 2025.
        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let memo_dir = Path::new("/m");
        assert_eq!(dir_for(memo_dir, date), PathBuf::from("/m/2025/01"));

        let memo = Memo::from_path(&memo_path(memo_dir, date, "newyear")).unwrap();
        assert_eq!(memo.year, 2025);
        assert_eq!(memo.week, 1);
    }

    #[test]
    fn from_path_file_round_trip() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let path = memo_path(Path::new("/home/u/stpls"), date, "my-first-note");
        let memo = Memo::from_path(&path).expect("should parse");
        assert_eq!(memo.kind, MemoKind::File);
        assert_eq!(memo.slug, "my-first-note");
        assert_eq!(memo.date, date);
        assert_eq!(memo.title, "My First Note");
        assert_eq!(memo.path, path);
    }

    #[test]
    fn from_path_project_round_trip() {
        let path = PathBuf::from("/home/u/stpls/2026/24/2026-06-14-big-thing/project.md");
        let memo = Memo::from_path(&path).expect("should parse");
        assert_eq!(memo.kind, MemoKind::Project);
        assert_eq!(memo.slug, "big-thing");
        assert_eq!(memo.title, "Big Thing");
        assert_eq!(memo.date, NaiveDate::from_ymd_opt(2026, 6, 14).unwrap());
        assert_eq!(memo.path, path);
    }

    #[test]
    fn from_path_rejects_non_memo() {
        assert!(Memo::from_path(Path::new("/tmp/notes.txt")).is_none());
        assert!(Memo::from_path(Path::new("/tmp/README.md")).is_none());
        assert!(Memo::from_path(Path::new("/tmp/2026-06-14.md")).is_none());
        assert!(Memo::from_path(Path::new("/tmp/2026-06-14-.md")).is_none());
        assert!(Memo::from_path(Path::new("/tmp/not-a-date-here.md")).is_none());
        // project.md whose parent doesn't parse.
        assert!(Memo::from_path(Path::new("/tmp/random-dir/project.md")).is_none());
    }
}
