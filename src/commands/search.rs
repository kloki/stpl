//! `stpl search <query> [-f format] [-a after] [-b before] [-t tag]` —
//! full-text search across memo bodies and diary entries.

use std::{env, fs};

use anstyle::{AnsiColor, Color, Style as AnsiStyle};
use anyhow::{Result, anyhow};
use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;

use crate::{
    cli::Format,
    commands::util,
    diary::{self, Diary},
    editor,
    error::StplError,
    memo::{self, Memo},
    output::{self, Style},
    store,
};

/// One search result. Untagged so a memo hit serializes exactly as it always
/// has; diary hits are told apart by their `kind: "diary"`, the same field
/// consumers already switch on for `file` vs `project`.
#[derive(Serialize)]
#[serde(untagged)]
enum Hit {
    Memo(MemoHit),
    Diary(DiaryHit),
}

/// A memo with the body lines that matched the query, serialized for `--format json`.
#[derive(Serialize)]
struct MemoHit {
    #[serde(flatten)]
    memo: Memo,
    matches: Vec<LineMatch>,
}

/// A diary with the entry lines that matched. Carries the diary's own fields
/// (name, slug, path) plus `kind`; `date`, `year`, `week`, and `tags` are
/// absent rather than null, because a diary has none.
#[derive(Serialize)]
struct DiaryHit {
    kind: &'static str,
    #[serde(flatten)]
    diary: Diary,
    matches: Vec<LineMatch>,
}

/// One matching body line: its 1-based number (within the body, after any
/// frontmatter) and trimmed text. For a diary, `entry` carries the timestamp of
/// the entry the line belongs to.
#[derive(Serialize)]
struct LineMatch {
    line: usize,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry: Option<String>,
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
    if let (Some(a), Some(b)) = (after, before)
        && a > b
    {
        return Err(anyhow!("invalid range: after {a} is later than before {b}"));
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
                entry: None,
            })
            .collect();
        if !matches.is_empty() {
            hits.push(Hit::Memo(MemoHit { memo, matches }));
        }
    }

    // Diaries have no tags, so a tag filter excludes them entirely. They are
    // listed after the memos, in name order (`list_all` sorts by slug).
    if tags.is_empty() {
        for diary in diary::list_all(&config)? {
            let Ok(content) = diary::read_content(&diary) else {
                continue;
            };
            let matches = search_diary(&content, &needle, after, before);
            if !matches.is_empty() {
                hits.push(Hit::Diary(DiaryHit {
                    kind: "diary",
                    diary,
                    matches,
                }));
            }
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

/// Find the lines of a diary matching `needle`.
///
/// Heading lines set the current entry and are never matched themselves. A
/// diary spans many dates, so the date filters apply per match, using the date
/// of the entry the line sits in — lines before the first heading belong to no
/// entry and are dropped whenever a filter is set.
fn search_diary(
    content: &str,
    needle: &str,
    after: Option<NaiveDate>,
    before: Option<NaiveDate>,
) -> Vec<LineMatch> {
    let filtered = after.is_some() || before.is_some();
    let mut current: Option<NaiveDateTime> = None;
    let mut matches = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if let Some(timestamp) = diary::parse_heading(line) {
            current = Some(timestamp);
            continue;
        }
        if !line.to_lowercase().contains(needle) {
            continue;
        }
        match current {
            Some(timestamp) => {
                let date = timestamp.date();
                if after.is_some_and(|a| date < a) || before.is_some_and(|b| date > b) {
                    continue;
                }
                matches.push(LineMatch {
                    line: i + 1,
                    text: line.trim().to_string(),
                    entry: Some(timestamp.format(diary::TIMESTAMP_FORMAT).to_string()),
                });
            }
            None => {
                if filtered {
                    continue;
                }
                matches.push(LineMatch {
                    line: i + 1,
                    text: line.trim().to_string(),
                    entry: None,
                });
            }
        }
    }
    matches
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

/// Human-facing output: the canonical clickable memo or diary line, then one
/// indented `line: text` per match (line number dimmed when color is enabled).
/// Diary matches also carry the timestamp of their entry.
fn render_text(style: &Style, hits: &[Hit], query: &str) {
    if hits.is_empty() {
        output::success(style, &format!("no memos or diaries match '{query}'"));
        return;
    }
    for (i, hit) in hits.iter().enumerate() {
        if i > 0 {
            anstream::println!();
        }
        let (heading, matches) = match hit {
            Hit::Memo(h) => (style.memo_line(&h.memo), &h.matches),
            Hit::Diary(h) => (
                format!("{} {}", style.diary_line(&h.diary), dim(style, "(diary)")),
                &h.matches,
            ),
        };
        anstream::println!("{heading}");
        for m in matches {
            let stamp = match &m.entry {
                Some(entry) => format!("{} ", dim(style, entry)),
                None => String::new(),
            };
            anstream::println!(
                "    {stamp}{} {}",
                dim(style, &format!("{}:", m.line)),
                m.text
            );
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
        let (title, path, suffix, matches) = match hit {
            Hit::Memo(h) => (&h.memo.title, &h.memo.path, "", &h.matches),
            Hit::Diary(h) => (&h.diary.name, &h.diary.path, " _(diary)_", &h.matches),
        };
        // file:// URL with spaces percent-encoded so links stay valid.
        let url = path.to_string_lossy().replace(' ', "%20");
        out.push_str(&format!("- [{title}](file://{url}){suffix}\n"));
        for m in matches {
            let stamp = match &m.entry {
                Some(entry) => format!(" {entry}"),
                None => String::new(),
            };
            out.push_str(&format!("  - `{}`{stamp}: {}\n", m.line, m.text));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const DIARY: &str = "preamble mentions ci\n\n# 2026-09-11T09:15\n\nblocked on CI\n\n# 2026-09-12T14:32\n\nCI fixed\n";

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn diary_matches_carry_their_entry_timestamp() {
        let matches = search_diary(DIARY, "ci", None, None);
        assert_eq!(matches.len(), 3);
        // The preamble belongs to no entry.
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[0].entry, None);
        assert_eq!(matches[1].line, 5);
        assert_eq!(matches[1].text, "blocked on CI");
        assert_eq!(matches[1].entry.as_deref(), Some("2026-09-11T09:15"));
        assert_eq!(matches[2].entry.as_deref(), Some("2026-09-12T14:32"));
    }

    #[test]
    fn heading_lines_are_never_matched() {
        // The timestamps contain "09", but headings are boundaries, not content.
        assert!(search_diary(DIARY, "2026-09", None, None).is_empty());
    }

    #[test]
    fn date_filters_apply_per_entry_and_drop_the_preamble() {
        let matches = search_diary(DIARY, "ci", Some(date(2026, 9, 12)), None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].entry.as_deref(), Some("2026-09-12T14:32"));

        let matches = search_diary(DIARY, "ci", None, Some(date(2026, 9, 11)));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].entry.as_deref(), Some("2026-09-11T09:15"));

        assert!(search_diary(DIARY, "ci", Some(date(2027, 1, 1)), None).is_empty());
    }

    #[test]
    fn markdown_marks_diary_hits_and_leaves_memo_hits_alone() {
        let memo = Memo::from_path(&PathBuf::from("/s/2026/24/2026-06-14-standup.md")).unwrap();
        let hits = vec![
            Hit::Memo(MemoHit {
                memo,
                matches: vec![LineMatch {
                    line: 7,
                    text: "blocked on CI".to_string(),
                    entry: None,
                }],
            }),
            Hit::Diary(DiaryHit {
                kind: "diary",
                diary: Diary {
                    name: "Work".to_string(),
                    slug: "work".to_string(),
                    path: PathBuf::from("/s/diaries/work.md"),
                },
                matches: vec![LineMatch {
                    line: 5,
                    text: "CI fixed".to_string(),
                    entry: Some("2026-09-11T14:32".to_string()),
                }],
            }),
        ];

        assert_eq!(
            render_markdown(&hits, "ci"),
            "# Search: ci\n\n\
             - [Standup](file:///s/2026/24/2026-06-14-standup.md)\n\
             \u{20}\u{20}- `7`: blocked on CI\n\
             - [Work](file:///s/diaries/work.md) _(diary)_\n\
             \u{20}\u{20}- `5` 2026-09-11T14:32: CI fixed\n"
        );
    }

    #[test]
    fn memo_hits_serialize_unchanged() {
        let memo = Memo::from_path(&PathBuf::from("/s/2026/24/2026-06-14-standup.md")).unwrap();
        let hits = vec![Hit::Memo(MemoHit {
            memo,
            matches: vec![LineMatch {
                line: 7,
                text: "blocked on CI".to_string(),
                entry: None,
            }],
        })];
        let json = serde_json::to_string(&hits).unwrap();
        // No `entry` key on memo matches, and no enum wrapper around the hit.
        assert!(json.contains("\"kind\":\"file\""));
        assert!(json.contains("\"matches\":[{\"line\":7,\"text\":\"blocked on CI\"}]"));
        assert!(!json.contains("entry"));
    }
}
