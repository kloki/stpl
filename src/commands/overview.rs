//! `stpl overview [format] -a <after> -b <before> -t <tag>` — list memos.

use std::{collections::BTreeMap, env, fs};

use anstyle::{AnsiColor, Color, Style as AnsiStyle};
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use serde::Serialize;

use crate::{
    cli::Format,
    commands::util,
    editor,
    error::StplError,
    memo::Memo,
    output::{self, Style},
    store,
};

/// A single `<year>/<week>` group, serialized for `--format json`.
#[derive(Serialize)]
struct Group {
    year: i32,
    week: u32,
    memos: Vec<Memo>,
}

/// List memos grouped by `<year>/<week>` folder.
pub fn run(
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
    // Tag filter: keep memos carrying at least one of the requested tags (OR).
    if !tags.is_empty() {
        memos.retain(|m| {
            m.tags
                .iter()
                .any(|mt| tags.iter().any(|t| t.eq_ignore_ascii_case(mt)))
        });
    }

    // Group by (year, week); BTreeMap keeps the headings sorted ascending.
    let mut grouped: BTreeMap<(i32, u32), Vec<Memo>> = BTreeMap::new();
    for memo in memos {
        grouped
            .entry((memo.year, memo.week))
            .or_default()
            .push(memo);
    }
    // Stable order within a group: by date then title.
    for memos in grouped.values_mut() {
        memos.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.title.cmp(&b.title)));
    }

    let groups: Vec<Group> = grouped
        .into_iter()
        .map(|((year, week), memos)| Group { year, week, memos })
        .collect();

    match format {
        Format::Text => render_text(&style, &groups),
        Format::Json => {
            let json = serde_json::to_string_pretty(&groups)?;
            anstream::println!("{json}");
        }
        Format::Markdown => {
            anstream::println!("{}", render_markdown(&groups));
        }
        Format::Editor => {
            let md = render_markdown(&groups);
            let path = env::temp_dir().join("stpl-overview.md");
            fs::write(&path, md)?;
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

/// Render the human-facing `text` overview: a colored `<year> · week NN`
/// heading per group, then one indented memo per line. The title is a clickable
/// hyperlink (when the terminal supports it) — the raw path is never printed —
/// and tags trail in color.
fn render_text(style: &Style, groups: &[Group]) {
    if groups.is_empty() {
        output_no_memos(style);
        return;
    }
    for (i, group) in groups.iter().enumerate() {
        // Blank line between groups for breathing room.
        if i > 0 {
            anstream::println!();
        }
        anstream::println!(
            "{}",
            heading(style, &format!("{} · week {:02}", group.year, group.week))
        );
        let bullet = if style.emoji { "📎" } else { "-" };
        for memo in &group.memos {
            // The title is the clickable link; the path itself is not shown.
            let title = style.link(&memo.title, &memo.path);
            anstream::println!("  {bullet} {title}{}", colored_tags(style, &memo.tags));
        }
    }
}

/// A bold-cyan group heading (plain text when color is disabled).
fn heading(style: &Style, text: &str) -> String {
    if !style.color {
        return text.to_string();
    }
    let c = AnsiStyle::new()
        .bold()
        .fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    format!("{c}{text}{c:#}")
}

/// `  #work #urgent` trailing a memo line, in magenta when color is enabled, or
/// an empty string when there are no tags.
fn colored_tags(style: &Style, tags: &[String]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    let joined = tags
        .iter()
        .map(|t| format!("#{t}"))
        .collect::<Vec<_>>()
        .join(" ");
    if !style.color {
        return format!("  {joined}");
    }
    let c = AnsiStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Magenta)));
    format!("  {c}{joined}{c:#}")
}

/// `  #work #urgent` for a tagged memo (plain), or an empty string when there
/// are none — used by the markdown renderer where ANSI color is inappropriate.
fn tags_suffix(tags: &[String]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    let joined = tags
        .iter()
        .map(|t| format!("#{t}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("  {joined}")
}

fn output_no_memos(style: &Style) {
    output::success(style, "no memos found");
}

fn render_markdown(groups: &[Group]) -> String {
    if groups.is_empty() {
        return "# Memos\n\n_No memos found._\n".to_string();
    }
    let mut out = String::from("# Memos\n");
    for group in groups {
        out.push_str(&format!("\n## {}/{:02}\n\n", group.year, group.week));
        for memo in &group.memos {
            // file:// URL with spaces percent-encoded so links stay valid.
            let url = memo.path.to_string_lossy().replace(' ', "%20");
            out.push_str(&format!(
                "- [{}](file://{}){}\n",
                memo.title,
                url,
                tags_suffix(&memo.tags)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::memo::MemoKind;

    fn memo(title: &str, path: &str, tags: &[&str]) -> Memo {
        Memo {
            title: title.to_string(),
            slug: "x".to_string(),
            date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            year: 2026,
            week: 25,
            path: PathBuf::from(path),
            kind: MemoKind::File,
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn render_markdown_uses_inline_links() {
        let groups = vec![
            Group {
                year: 2026,
                week: 25,
                memos: vec![
                    memo("Alpha", "/m/a.md", &["api", "hub"]),
                    memo("Beta", "/m/b.md", &[]),
                ],
            },
            Group {
                year: 2026,
                week: 26,
                memos: vec![memo("Gamma", "/m/c.md", &["rest"])],
            },
        ];
        let md = render_markdown(&groups);
        assert_eq!(
            md,
            "# Memos\n\
             \n## 2026/25\n\n\
             - [Alpha](file:///m/a.md)  #api #hub\n\
             - [Beta](file:///m/b.md)\n\
             \n## 2026/26\n\n\
             - [Gamma](file:///m/c.md)  #rest\n"
        );
    }
}
