//! Filesystem operations over the memo tree.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use walkdir::WalkDir;

use crate::{
    config::Config,
    error::StplError,
    memo::{self, Memo, MemoKind},
};

/// Walk the memo directory and return every parseable memo.
///
/// Returns an empty vec (NOT an error) when the memo directory does not exist.
/// Files whose names don't parse as memos are skipped silently. Order is
/// unspecified; callers sort/group as needed.
pub fn list_all(config: &Config) -> Result<Vec<Memo>> {
    let root = &config.memo_directory;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut memos = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Some(memo) = Memo::from_path(entry.path()) {
            memos.push(memo);
        }
    }
    Ok(memos)
}

/// Create a new memo file for `date`/`slug` with the YAML-frontmatter + H1
/// template (see [`render_template`]). Lazily creates the `<year>/<week>`
/// directories. `content` is the optional body (`-m`).
///
/// Errors with `StplError::Collision` if the target file already exists.
/// Returns the absolute path written.
pub fn create(
    config: &Config,
    date: NaiveDate,
    slug: &str,
    title: &str,
    content: Option<&str>,
) -> Result<PathBuf> {
    let path = memo::memo_path(&config.memo_directory, date, slug);
    if path.exists() {
        return Err(StplError::Collision(path).into());
    }

    let dir = memo::dir_for(&config.memo_directory, date);
    fs::create_dir_all(&dir).with_context(|| format!("creating directory '{}'", dir.display()))?;

    let body = render_template(title, date, content);
    fs::write(&path, body).with_context(|| format!("writing memo '{}'", path.display()))?;
    Ok(path)
}

/// The initial file contents for a new memo:
///
/// ```text
/// ---
/// title: <title>
/// date: <YYYY-MM-DD>
/// tags: []
/// ---
///
/// # <title>
///
/// <content, if any>
/// ```
pub fn render_template(title: &str, date: NaiveDate, content: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("title: {}\n", title));
    out.push_str(&format!("date: {}\n", date.format("%Y-%m-%d")));
    out.push_str("tags: []\n");
    out.push_str("---\n");
    out.push('\n');
    out.push_str(&format!("# {}\n", title));
    if let Some(content) = content {
        out.push('\n');
        out.push_str(content);
        // Ensure the file ends with a single trailing newline.
        if !content.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Add `new_tags` to a memo's frontmatter `tags:` list, rewriting the file in
/// place. Existing tags are kept (in order) and duplicates are ignored — both
/// against tags already on the memo and within `new_tags` itself. Blank tags
/// are dropped. Returns the resulting full tag list.
pub fn add_tags(memo: &Memo, new_tags: &[String]) -> Result<Vec<String>> {
    let content = fs::read_to_string(&memo.path)
        .with_context(|| format!("reading '{}'", memo.path.display()))?;

    let mut merged = memo.tags.clone();
    for tag in new_tags {
        let tag = tag.trim();
        if !tag.is_empty() && !merged.iter().any(|existing| existing == tag) {
            merged.push(tag.to_string());
        }
    }

    let updated = memo::write_tags(&content, &merged);
    fs::write(&memo.path, updated).with_context(|| format!("writing '{}'", memo.path.display()))?;
    Ok(merged)
}

/// Read a memo's full file contents.
pub fn read_content(memo: &Memo) -> Result<String> {
    fs::read_to_string(&memo.path).with_context(|| format!("reading '{}'", memo.path.display()))
}

/// Append `text` to a memo's body, rewriting the file in place. A blank line is
/// inserted between the existing content and the new text, and the file is left
/// ending in exactly one newline.
pub fn append(memo: &Memo, text: &str) -> Result<()> {
    let mut content = read_content(memo)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    // Separating blank line, unless the file is effectively empty.
    if !content.trim_end().is_empty() {
        content.push('\n');
    }
    content.push_str(text.trim_end_matches('\n'));
    content.push('\n');
    fs::write(&memo.path, content).with_context(|| format!("writing '{}'", memo.path.display()))?;
    Ok(())
}

/// Remove `tags` (case-insensitive) from a memo's frontmatter, rewriting the
/// file in place. Tags not present are ignored. Returns the remaining tags.
pub fn remove_tags(memo: &Memo, tags: &[String]) -> Result<Vec<String>> {
    let content = read_content(memo)?;
    let drop: Vec<String> = tags
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    let remaining: Vec<String> = memo
        .tags
        .iter()
        .filter(|t| !drop.iter().any(|d| *d == t.to_lowercase()))
        .cloned()
        .collect();

    let updated = memo::write_tags(&content, &remaining);
    fs::write(&memo.path, updated).with_context(|| format!("writing '{}'", memo.path.display()))?;
    Ok(remaining)
}

/// Count tag occurrences across all memos. Returns `(tag, count)` pairs sorted
/// by count descending, then tag ascending.
pub fn tag_counts(config: &Config) -> Result<Vec<(String, usize)>> {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for memo in list_all(config)? {
        for tag in memo.tags {
            *counts.entry(tag).or_insert(0) += 1;
        }
    }
    let mut out: Vec<(String, usize)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(out)
}

/// Rename a memo to `new_title`: rewrite the in-file title (frontmatter
/// `title:` and the first `# ` heading) and move the file — or, for a project,
/// the whole project directory — to its new `<iso_date>-<new_slug>` name. The
/// `<year>/<week>` folder (and a project's date prefix) are unchanged. Returns
/// the new `.md` path and the new slug.
///
/// Errors with `StplError::Collision` if the target already exists, before any
/// changes are made.
pub fn rename(memo: &Memo, new_title: &str) -> Result<(PathBuf, String)> {
    let new_slug = memo::slugify(new_title)?;
    let new_stem = memo::stem_for(memo.date, &new_slug);

    match memo.kind {
        MemoKind::File => {
            let parent = memo
                .path
                .parent()
                .context("memo file has no parent directory")?;
            let new_path = parent.join(format!("{new_stem}.md"));
            if new_path != memo.path && new_path.exists() {
                return Err(StplError::Collision(new_path).into());
            }

            let content = read_content(memo)?;
            let updated = memo::rewrite_title(&content, new_title);
            fs::write(&memo.path, updated)
                .with_context(|| format!("writing '{}'", memo.path.display()))?;

            if new_path != memo.path {
                fs::rename(&memo.path, &new_path).with_context(|| {
                    format!(
                        "renaming '{}' to '{}'",
                        memo.path.display(),
                        new_path.display()
                    )
                })?;
            }
            Ok((new_path, new_slug))
        }
        MemoKind::Project => {
            let project_dir = memo
                .path
                .parent()
                .context("project memo has no parent directory")?;
            let grandparent = project_dir
                .parent()
                .context("project directory has no parent")?;
            let new_dir = grandparent.join(&new_stem);
            if new_dir != project_dir && new_dir.exists() {
                return Err(StplError::Collision(new_dir).into());
            }

            let content = read_content(memo)?;
            let updated = memo::rewrite_title(&content, new_title);
            fs::write(&memo.path, updated)
                .with_context(|| format!("writing '{}'", memo.path.display()))?;

            if new_dir != project_dir {
                fs::rename(project_dir, &new_dir).with_context(|| {
                    format!(
                        "renaming '{}' to '{}'",
                        project_dir.display(),
                        new_dir.display()
                    )
                })?;
            }
            Ok((new_dir.join("project.md"), new_slug))
        }
    }
}

/// Delete a memo. For `MemoKind::File`, removes the `.md` file. For
/// `MemoKind::Project`, removes the entire project directory.
pub fn delete(memo: &Memo) -> Result<()> {
    match memo.kind {
        MemoKind::File => fs::remove_file(&memo.path)
            .with_context(|| format!("deleting '{}'", memo.path.display()))?,
        MemoKind::Project => {
            // The project directory is the parent of `project.md`.
            let dir = memo
                .path
                .parent()
                .context("project memo has no parent directory")?;
            fs::remove_dir_all(dir)
                .with_context(|| format!("deleting project '{}'", dir.display()))?;
        }
    }
    Ok(())
}

/// Expand a file memo into a project: create `<stem>/` next to the file and
/// move `<stem>.md` into it as `project.md`. Returns the new `project.md` path.
///
/// Errors with `StplError::Collision` if the target directory or `project.md`
/// already exists, and refuses to expand a memo that is already a project.
pub fn expand(memo: &Memo) -> Result<PathBuf> {
    if memo.kind == MemoKind::Project {
        bail!("'{}' is already a project", memo.title);
    }

    // `<stem>/` lives next to the `.md`, named after the file stem.
    let parent = memo
        .path
        .parent()
        .context("memo file has no parent directory")?;
    let stem = memo
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("memo file has no stem")?;
    let project_dir = parent.join(stem);

    if project_dir.exists() {
        return Err(StplError::Collision(project_dir).into());
    }

    let target = project_dir.join("project.md");
    if target.exists() {
        return Err(StplError::Collision(target).into());
    }

    fs::create_dir_all(&project_dir)
        .with_context(|| format!("creating project directory '{}'", project_dir.display()))?;
    fs::rename(&memo.path, &target)
        .with_context(|| format!("moving '{}' to '{}'", memo.path.display(), target.display()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template_no_content() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let out = render_template("My Note", date, None);
        assert_eq!(
            out,
            "---\ntitle: My Note\ndate: 2026-06-14\ntags: []\n---\n\n# My Note\n"
        );
    }

    #[test]
    fn render_template_with_content() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
        let out = render_template("My Note", date, Some("hello body"));
        assert_eq!(
            out,
            "---\ntitle: My Note\ndate: 2026-06-14\ntags: []\n---\n\n# My Note\n\nhello body\n"
        );
    }

    #[test]
    fn create_list_delete_round_trip() {
        let tmp = std::env::temp_dir().join(format!("stpl-test-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        // Empty/missing dir -> empty list.
        assert!(list_all(&config).unwrap().is_empty());

        let path = create(&config, date, "my-note", "My Note", None).unwrap();
        assert!(path.exists());
        assert_eq!(path, tmp.join("2026/24/2026-06-14-my-note.md"));

        // Collision on second create.
        assert!(matches!(
            create(&config, date, "my-note", "My Note", None)
                .unwrap_err()
                .downcast::<StplError>()
                .unwrap(),
            StplError::Collision(_)
        ));

        let memos = list_all(&config).unwrap();
        assert_eq!(memos.len(), 1);
        assert_eq!(memos[0].slug, "my-note");

        delete(&memos[0]).unwrap();
        assert!(!path.exists());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn add_tags_merges_and_dedupes() {
        let tmp = std::env::temp_dir().join(format!("stpl-tags-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let path = create(&config, date, "my-note", "My Note", None).unwrap();

        // First add: two fresh tags.
        let memo = Memo::from_path(&path).unwrap();
        let all = add_tags(&memo, &["work".to_string(), "urgent".to_string()]).unwrap();
        assert_eq!(all, vec!["work", "urgent"]);

        // Re-read and add overlapping + blank + duplicate-within-input tags.
        let memo = Memo::from_path(&path).unwrap();
        assert_eq!(memo.tags, vec!["work", "urgent"]);
        let all = add_tags(
            &memo,
            &[
                "work".to_string(),
                " ".to_string(),
                "home".to_string(),
                "home".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(all, vec!["work", "urgent", "home"]);

        // The on-disk file reflects the merged set.
        assert_eq!(
            Memo::from_path(&path).unwrap().tags,
            vec!["work", "urgent", "home"]
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn append_adds_line_with_blank_separator() {
        let tmp = std::env::temp_dir().join(format!("stpl-append-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let path = create(&config, date, "log", "Log", Some("first")).unwrap();
        let memo = Memo::from_path(&path).unwrap();
        append(&memo, "second").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            "---\ntitle: Log\ndate: 2026-06-14\ntags: []\n---\n\n# Log\n\nfirst\n\nsecond\n"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn remove_tags_is_case_insensitive_and_ignores_missing() {
        let tmp = std::env::temp_dir().join(format!("stpl-untag-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let path = create(&config, date, "n", "N", None).unwrap();
        let memo = Memo::from_path(&path).unwrap();
        add_tags(&memo, &["work".to_string(), "urgent".to_string()]).unwrap();

        let memo = Memo::from_path(&path).unwrap();
        // "WORK" matches "work" (case-insensitive); "missing" is ignored.
        let remaining = remove_tags(&memo, &["WORK".to_string(), "missing".to_string()]).unwrap();
        assert_eq!(remaining, vec!["urgent"]);
        assert_eq!(Memo::from_path(&path).unwrap().tags, vec!["urgent"]);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tag_counts_aggregates_and_sorts() {
        let tmp = std::env::temp_dir().join(format!("stpl-tagcount-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let a = Memo::from_path(&create(&config, date, "a", "A", None).unwrap()).unwrap();
        add_tags(&a, &["work".to_string(), "home".to_string()]).unwrap();
        let b = Memo::from_path(&create(&config, date, "b", "B", None).unwrap()).unwrap();
        add_tags(&b, &["work".to_string()]).unwrap();

        // Sorted by count desc, then tag asc.
        assert_eq!(
            tag_counts(&config).unwrap(),
            vec![("work".to_string(), 2), ("home".to_string(), 1)]
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rename_file_moves_and_rewrites_title() {
        let tmp = std::env::temp_dir().join(format!("stpl-rename-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let path = create(&config, date, "old-name", "Old Name", Some("body")).unwrap();
        let memo = Memo::from_path(&path).unwrap();

        let (new_path, new_slug) = rename(&memo, "New Name").unwrap();
        assert_eq!(new_slug, "new-name");
        assert!(!path.exists());
        assert_eq!(new_path, tmp.join("2026/24/2026-06-14-new-name.md"));

        let renamed = Memo::from_path(&new_path).unwrap();
        assert_eq!(renamed.title, "New Name");
        let content = fs::read_to_string(&new_path).unwrap();
        assert!(content.contains("title: New Name"));
        assert!(content.contains("# New Name"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rename_project_moves_directory() {
        let tmp = std::env::temp_dir().join(format!("stpl-renameproj-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let path = create(&config, date, "old-proj", "Old Proj", Some("body")).unwrap();
        let project_md = expand(&Memo::from_path(&path).unwrap()).unwrap();
        let project = Memo::from_path(&project_md).unwrap();

        let (new_path, _) = rename(&project, "New Proj").unwrap();
        assert_eq!(new_path, tmp.join("2026/24/2026-06-14-new-proj/project.md"));
        assert!(new_path.exists());
        assert!(!project_md.exists());

        let renamed = Memo::from_path(&new_path).unwrap();
        assert_eq!(renamed.kind, MemoKind::Project);
        assert_eq!(renamed.slug, "new-proj");
        assert_eq!(renamed.title, "New Proj");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rename_collision_errors_before_changes() {
        let tmp = std::env::temp_dir().join(format!("stpl-renamecol-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        create(&config, date, "taken", "Taken", None).unwrap();
        let src = create(&config, date, "source", "Source", None).unwrap();
        let memo = Memo::from_path(&src).unwrap();

        assert!(matches!(
            rename(&memo, "Taken")
                .unwrap_err()
                .downcast::<StplError>()
                .unwrap(),
            StplError::Collision(_)
        ));
        // The source file is untouched (title not rewritten).
        let content = fs::read_to_string(&src).unwrap();
        assert!(content.contains("title: Source"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn expand_moves_file_into_project() {
        let tmp = std::env::temp_dir().join(format!("stpl-expand-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_color: true,
        };
        let date = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();

        let path = create(&config, date, "big-thing", "Big Thing", Some("body")).unwrap();
        let memo = Memo::from_path(&path).unwrap();
        assert_eq!(memo.kind, MemoKind::File);

        let project_md = expand(&memo).unwrap();
        assert!(!path.exists());
        assert!(project_md.exists());
        assert_eq!(
            project_md,
            tmp.join("2026/24/2026-06-14-big-thing/project.md")
        );

        // The moved file is now recognized as a project.
        let project = Memo::from_path(&project_md).unwrap();
        assert_eq!(project.kind, MemoKind::Project);
        assert_eq!(project.slug, "big-thing");

        // Refuse to expand an existing project.
        assert!(expand(&project).is_err());

        let _ = fs::remove_dir_all(&tmp);
    }
}
