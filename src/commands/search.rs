//! `stpl search <query> [-f format] [-a after] [-b before] [-t tag]` —
//! full-text search across memo bodies.

use std::{env, fs};

use anstyle::{AnsiColor, Color, Style as AnsiStyle};
use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use serde::Serialize;

use crate::{
    cli::Format,
    commands::util,
    editor,
    error::StplError,
    memo::{self, Memo},
    output::{self, Style},
    store,
};

/// A memo with the body lines that matched the query, serialized for `--format json`.
#[derive(Serialize)]
struct Hit {
    #[serde(flatten)]
    memo: Memo,
    matches: Vec<LineMatch>,
}

/// One matching body line: its 1-based number (within the body, after any
/// frontmatter) and trimmed text.
#[derive(Serialize)]
struct LineMatch {
    line: usize,
    text: String,
}

/// Search memo bodies for `query` (case-insensitive substring), honoring the
/// same date/tag filters as `overview`.
pub fn run(
    query: &str,
    format: Format,
    after: Option<&str>,
    before: Option<&str>,
    tags: &[String],
) -> Result<()> {
    let (config, style) = util::config_and_style()?;

    let after = parse_date(after)?;
    let before = parse_date(before)?;
    if let (Some(a), Some(b)) = (after, before) {
        if a > b {
            return Err(anyhow!("invalid range: after {a} is later than before {b}"));
        }
    }

    let mut memos = store::list_all(&config)?;
    memos.retain(|m| after.is_none_or(|a| m.date >= a) && before.is_none_or(|b| m.date <= b));
    if !tags.is_empty() {
        memos.retain(|m| {
            m.tags
                .iter()
                .any(|mt| tags.iter().any(|t| t.eq_ignore_ascii_case(mt)))
        });
    }
    // Stable order: by date then title.
    memos.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.title.cmp(&b.title)));

    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for memo in memos {
        // Unreadable files are skipped silently, matching the rest of the tool.
        let Ok(content) = store::read_content(&memo) else {
            continue;
        };
        let matches: Vec<LineMatch> = memo::strip_frontmatter(&content)
            .lines()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&needle))
            .map(|(i, line)| LineMatch {
                line: i + 1,
                text: line.trim().to_string(),
            })
            .collect();
        if !matches.is_empty() {
            hits.push(Hit { memo, matches });
        }
    }

    match format {
        Format::Text => render_text(&style, &hits, query),
        Format::Json => anstream::println!("{}", serde_json::to_string_pretty(&hits)?),
        Format::Markdown => anstream::println!("{}", render_markdown(&hits, query)),
        Format::Editor => {
            let path = env::temp_dir().join("stpl-search.md");
            fs::write(&path, render_markdown(&hits, query))?;
            editor::open(&path)?;
        }
    }
    Ok(())
}

/// Parse a strict `YYYY-MM-DD` date, mapping failures to `InvalidDate`.
fn parse_date(s: Option<&str>) -> Result<Option<NaiveDate>> {
    match s {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| StplError::InvalidDate(s.to_string()).into()),
    }
}

/// Human-facing output: the canonical clickable memo line, then one indented
/// `line: text` per match (line number dimmed when color is enabled).
fn render_text(style: &Style, hits: &[Hit], query: &str) {
    if hits.is_empty() {
        output::success(style, &format!("no memos match '{query}'"));
        return;
    }
    for (i, hit) in hits.iter().enumerate() {
        if i > 0 {
            anstream::println!();
        }
        anstream::println!("{}", style.memo_line(&hit.memo));
        for m in &hit.matches {
            anstream::println!("    {} {}", dim(style, &format!("{}:", m.line)), m.text);
        }
    }
}

/// `text` dimmed (bright-black) when color is enabled, else plain.
fn dim(style: &Style, text: &str) -> String {
    if !style.color {
        return text.to_string();
    }
    let c = AnsiStyle::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));
    format!("{c}{text}{c:#}")
}

fn render_markdown(hits: &[Hit], query: &str) -> String {
    if hits.is_empty() {
        return format!("# Search: {query}\n\n_No matches._\n");
    }
    let mut out = format!("# Search: {query}\n\n");
    for hit in hits {
        // file:// URL with spaces percent-encoded so links stay valid.
        let url = hit.memo.path.to_string_lossy().replace(' ', "%20");
        out.push_str(&format!("- [{}](file://{})\n", hit.memo.title, url));
        for m in &hit.matches {
            out.push_str(&format!("  - `{}`: {}\n", m.line, m.text));
        }
    }
    out
}
