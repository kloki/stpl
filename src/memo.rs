//! The `Memo` data model plus path<->memo conversion and slug helpers.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{Datelike, NaiveDate};
use serde::Serialize;

use crate::error::StplError;

/// Whether a memo is a single file or an expanded project directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoKind {
    /// A `<iso_date>-<slug>.md` file.
    File,
    /// A `<iso_date>-<slug>/project.md` project directory.
    Project,
}

/// A single memo discovered on disk.
#[derive(Debug, Clone, Serialize)]
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
    /// Tags parsed from the YAML frontmatter (empty when none / unreadable).
    pub tags: Vec<String>,
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
            tags: read_tags(path),
            path: path.to_path_buf(),
            kind,
        })
    }
}

/// Read a memo file and parse its frontmatter `tags`. Best-effort: returns an
/// empty vec when the file is absent or unreadable (e.g. `from_path` called on
/// a path that doesn't exist yet).
fn read_tags(path: &Path) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(content) => parse_tags(&content),
        Err(_) => Vec::new(),
    }
}

/// Parse the inline `tags: [a, b, c]` list from a memo's YAML frontmatter.
///
/// Only the leading frontmatter block (delimited by `---` lines) is scanned,
/// and only the inline `[..]` form is understood — matching what `stpl` itself
/// writes. Surrounding whitespace and optional quotes are stripped from each
/// tag; empty items are dropped. Returns an empty vec when there is no
/// frontmatter or no `tags:` line.
fn parse_tags(content: &str) -> Vec<String> {
    // Frontmatter must be at the very top: a `---` line, then the block, then a
    // closing `---` line.
    let rest = match content.strip_prefix("---\n") {
        Some(rest) => rest,
        None => return Vec::new(),
    };

    for line in rest.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            // End of frontmatter without a `tags:` line.
            break;
        }
        if let Some(value) = trimmed.strip_prefix("tags:") {
            let value = value.trim();
            let inner = match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                Some(inner) => inner,
                None => return Vec::new(),
            };
            return inner
                .split(',')
                .map(|t| t.trim().trim_matches(|c| c == '"' || c == '\'').trim())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect();
        }
    }
    Vec::new()
}

/// Render the inline frontmatter `tags:` line for `tags`, e.g. `tags: [a, b]`.
fn render_tags_line(tags: &[String]) -> String {
    format!("tags: [{}]", tags.join(", "))
}

/// Return `content` with its frontmatter `tags:` list set to `tags` (inline
/// form, matching what the template writes).
///
/// - If the leading frontmatter has an inline `tags:` line, it is replaced in
///   place (other lines are preserved verbatim).
/// - If the frontmatter has no `tags:` line, one is inserted just before the
///   closing `---`.
/// - If `content` has no leading frontmatter at all, a fresh block is prepended.
pub fn write_tags(content: &str, tags: &[String]) -> String {
    let tags_line = render_tags_line(tags);

    // No leading frontmatter: prepend a fresh block.
    let body = match content.strip_prefix("---\n") {
        Some(body) => body,
        None => return format!("---\n{}\n---\n\n{}", tags_line, content),
    };

    let mut out = String::from("---\n");
    let mut handled = false;
    let mut in_frontmatter = true;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim();
        if in_frontmatter && trimmed == "---" {
            // Closing fence: insert the tags line here if we never saw one.
            if !handled {
                out.push_str(&tags_line);
                out.push('\n');
                handled = true;
            }
            out.push_str(line);
            in_frontmatter = false;
            continue;
        }
        if in_frontmatter && !handled && trimmed.strip_prefix("tags:").is_some() {
            out.push_str(&tags_line);
            out.push('\n');
            handled = true;
            continue;
        }
        out.push_str(line);
    }
    out
}

/// Return the body of `content` with any leading YAML frontmatter block
/// removed. If there is no `---`-delimited frontmatter at the top, the whole
/// string is returned unchanged. Leading blank lines after the closing fence
/// are trimmed so the body starts at its first real line.
pub fn strip_frontmatter(content: &str) -> &str {
    let rest = match content.strip_prefix("---\n") {
        Some(rest) => rest,
        None => return content,
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches('\n').trim() == "---" {
            let after = offset + line.len();
            return rest[after..].trim_start_matches('\n');
        }
        offset += line.len();
    }
    // Unterminated frontmatter: leave content untouched.
    content
}

/// Return `content` with its title set to `new_title`: replaces the frontmatter
/// `title:` line (if present) and the first level-1 (`# `) heading in the body
/// (if present). All other lines — and the file's trailing-newline shape — are
/// preserved verbatim.
pub fn rewrite_title(content: &str, new_title: &str) -> String {
    let has_fm = content.starts_with("---\n");
    let mut out = String::with_capacity(content.len() + new_title.len());
    let mut in_frontmatter = false;
    let mut fm_title_done = false;
    let mut h1_done = false;

    for (i, line) in content.split_inclusive('\n').enumerate() {
        let body = line.trim_end_matches('\n');
        let nl = if line.ends_with('\n') { "\n" } else { "" };

        if i == 0 && has_fm {
            in_frontmatter = true;
            out.push_str(line);
            continue;
        }
        if in_frontmatter {
            if body.trim() == "---" {
                in_frontmatter = false;
                out.push_str(line);
                continue;
            }
            if !fm_title_done && body.trim().strip_prefix("title:").is_some() {
                out.push_str("title: ");
                out.push_str(new_title);
                out.push_str(nl);
                fm_title_done = true;
                continue;
            }
            out.push_str(line);
            continue;
        }
        if !h1_done && body.strip_prefix("# ").is_some() {
            out.push_str("# ");
            out.push_str(new_title);
            out.push_str(nl);
            h1_done = true;
            continue;
        }
        out.push_str(line);
    }
    out
}

/// Parse a `<iso_date>-<slug>` stem into its date and slug components.
///
/// The first 10 chars must be `%Y-%m-%d`, followed by a `-`, then a non-empty
/// slug. Returns `None` on any parse failure.
fn parse_stem(stem: &str) -> Option<(NaiveDate, String)> {
    // `2026-06-14` is exactly 10 bytes; we need at least that plus a `-` and
    // one slug character. `get(..10)` returns `None` (rather than panicking
    // like `split_at`) when byte 10 is not a char boundary, e.g. for a foreign
    // file whose name starts with a multibyte character.
    let date_part = stem.get(..10)?;
    let rest = &stem[10..];
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

    #[test]
    fn parse_tags_inline_list() {
        let content = "---\ntitle: T\ndate: 2026-06-14\ntags: [work, urgent]\n---\n\n# T\n";
        assert_eq!(parse_tags(content), vec!["work", "urgent"]);
    }

    #[test]
    fn parse_tags_empty_list() {
        let content = "---\ntitle: T\ndate: 2026-06-14\ntags: []\n---\n\n# T\n";
        assert!(parse_tags(content).is_empty());
    }

    #[test]
    fn parse_tags_strips_quotes_and_spaces() {
        let content = "---\ntags: [ \"work\" , 'home' ,  , urgent ]\n---\n";
        assert_eq!(parse_tags(content), vec!["work", "home", "urgent"]);
    }

    #[test]
    fn parse_tags_missing_line_or_frontmatter() {
        // No `tags:` line in the frontmatter.
        let no_tags = "---\ntitle: T\ndate: 2026-06-14\n---\n\n# T\n";
        assert!(parse_tags(no_tags).is_empty());
        // No leading frontmatter at all.
        let no_fm = "# T\n\ntags: [work]\n";
        assert!(parse_tags(no_fm).is_empty());
        // `tags:` present but not the inline form.
        let block = "---\ntags:\n  - work\n---\n";
        assert!(parse_tags(block).is_empty());
    }

    #[test]
    fn write_tags_replaces_inline_list() {
        let content = "---\ntitle: T\ndate: 2026-06-14\ntags: []\n---\n\n# T\n";
        let out = write_tags(content, &["work".to_string(), "urgent".to_string()]);
        assert_eq!(
            out,
            "---\ntitle: T\ndate: 2026-06-14\ntags: [work, urgent]\n---\n\n# T\n"
        );
        // The written form round-trips through the parser.
        assert_eq!(parse_tags(&out), vec!["work", "urgent"]);
    }

    #[test]
    fn write_tags_inserts_when_missing() {
        let content = "---\ntitle: T\ndate: 2026-06-14\n---\n\n# T\n";
        let out = write_tags(content, &["work".to_string()]);
        assert_eq!(
            out,
            "---\ntitle: T\ndate: 2026-06-14\ntags: [work]\n---\n\n# T\n"
        );
    }

    #[test]
    fn write_tags_prepends_when_no_frontmatter() {
        let content = "# T\n\nbody\n";
        let out = write_tags(content, &["work".to_string()]);
        assert_eq!(out, "---\ntags: [work]\n---\n\n# T\n\nbody\n");
        assert_eq!(parse_tags(&out), vec!["work"]);
    }

    #[test]
    fn strip_frontmatter_removes_block() {
        let content = "---\ntitle: T\ntags: [a]\n---\n\n# T\n\nbody line\n";
        assert_eq!(strip_frontmatter(content), "# T\n\nbody line\n");
    }

    #[test]
    fn strip_frontmatter_passthrough_and_unterminated() {
        // No frontmatter: returned unchanged.
        let no_fm = "# T\n\nbody\n";
        assert_eq!(strip_frontmatter(no_fm), no_fm);
        // Opening fence but no closing fence: left untouched.
        let unterminated = "---\ntitle: T\nstill in frontmatter\n";
        assert_eq!(strip_frontmatter(unterminated), unterminated);
    }

    #[test]
    fn rewrite_title_updates_frontmatter_and_h1() {
        let content = "---\ntitle: Old\ndate: 2026-06-14\ntags: []\n---\n\n# Old\n\nbody\n";
        let out = rewrite_title(content, "New Name");
        assert_eq!(
            out,
            "---\ntitle: New Name\ndate: 2026-06-14\ntags: []\n---\n\n# New Name\n\nbody\n"
        );
    }

    #[test]
    fn rewrite_title_only_first_h1_and_no_frontmatter() {
        // No frontmatter; only the first `# ` heading is changed.
        let content = "# Old\n\n# Old\n";
        assert_eq!(rewrite_title(content, "New"), "# New\n\n# Old\n");
    }

    #[test]
    fn from_path_handles_multibyte_stems_without_panicking() {
        // A foreign file whose name is >= 12 bytes but has a multibyte char
        // straddling byte index 10 must be rejected, not panic. `日`/`本` are
        // three bytes each, so `123456789日本` puts `日` across bytes 9..12.
        assert!(parse_stem("123456789日本").is_none());
        assert!(Memo::from_path(Path::new("/tmp/123456789日本.md")).is_none());
        // A genuine date prefix followed by a multibyte slug still parses.
        let (date, slug) = parse_stem("2026-06-14-café").expect("valid date prefix");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 6, 14).unwrap());
        assert_eq!(slug, "café");
    }
}
