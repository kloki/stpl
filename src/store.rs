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
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating directory '{}'", dir.display()))?;

    let body = render_template(title, date, content);
    std::fs::write(&path, body).with_context(|| format!("writing memo '{}'", path.display()))?;
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

/// Delete a memo. For `MemoKind::File`, removes the `.md` file. For
/// `MemoKind::Project`, removes the entire project directory.
pub fn delete(memo: &Memo) -> Result<()> {
    match memo.kind {
        MemoKind::File => std::fs::remove_file(&memo.path)
            .with_context(|| format!("deleting '{}'", memo.path.display()))?,
        MemoKind::Project => {
            // The project directory is the parent of `project.md`.
            let dir = memo
                .path
                .parent()
                .context("project memo has no parent directory")?;
            std::fs::remove_dir_all(dir)
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
        anyhow::bail!("'{}' is already a project", memo.title);
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

    std::fs::create_dir_all(&project_dir)
        .with_context(|| format!("creating project directory '{}'", project_dir.display()))?;
    std::fs::rename(&memo.path, &target)
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
            disable_emoji: true,
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

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn expand_moves_file_into_project() {
        let tmp = std::env::temp_dir().join(format!("stpl-expand-{}", std::process::id()));
        let config = Config {
            memo_directory: tmp.clone(),
            disable_emoji: true,
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

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
