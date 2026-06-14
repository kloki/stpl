//! `stpl overview -f <format> -a <after> -b <before>` — list memos.
//!
//! CONTRACT — implement `run`; do not change its signature.

use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use chrono::NaiveDate;

use crate::{
    cli::Format, commands::util, editor, error::StplError, memo::Memo, output::Style, store,
};

/// A single `<year>/<week>` group, serialized for `--format json`.
#[derive(serde::Serialize)]
struct Group {
    year: i32,
    week: u32,
    memos: Vec<Memo>,
}

/// List memos grouped by `<year>/<week>` folder.
pub fn run(format: Format, after: Option<&str>, before: Option<&str>) -> Result<()> {
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
            let path = std::env::temp_dir().join("stpl-overview.md");
            std::fs::write(&path, md)?;
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

fn render_text(style: &Style, groups: &[Group]) {
    if groups.is_empty() {
        output_no_memos(style);
        return;
    }
    for group in groups {
        anstream::println!("{}/{:02}", group.year, group.week);
        for memo in &group.memos {
            anstream::println!("  {}", style.memo_line(memo));
        }
    }
}

fn output_no_memos(style: &Style) {
    crate::output::success(style, "no memos found");
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
            out.push_str(&format!("- [{}](file://{})\n", memo.title, url));
        }
    }
    out
}
