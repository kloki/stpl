//! `stpl diary [name] [-m text]` and its management subcommands — append-only,
//! timestamped logs.
//!
//! The bare form is the one people type all day, so it is the shortest: a name
//! and a message. Everything else hangs off a subcommand.

use std::{
    env, fs,
    io::{IsTerminal, Write},
};

use anstyle::{AnsiColor, Color, Style as AnsiStyle};
use anyhow::{Result, anyhow};
use chrono::Local;
use serde::Serialize;

use crate::{
    cli::{DiaryArgs, DiaryCommand, Format},
    commands::util,
    diary::{self, Diary},
    editor,
    error::StplError,
    output::{self, Style},
    state,
};

/// A diary and its entry statistics, serialized for `list --format json` as the
/// diary's own fields (name, slug, path) plus the counts.
#[derive(Serialize)]
struct DiarySummary {
    #[serde(flatten)]
    diary: Diary,
    entries: usize,
    /// Timestamp of the most recent entry, `null` for an empty diary.
    last_entry: Option<String>,
}

/// Dispatch the diary subcommand, or — with none — add an entry when `-m` was
/// given and otherwise open the diary in `$EDITOR`.
pub fn run(args: DiaryArgs) -> Result<()> {
    match args.command {
        Some(DiaryCommand::List { format }) => list(format),
        Some(DiaryCommand::Show { name }) => show(name.as_deref()),
        Some(DiaryCommand::Edit { name }) => edit(name.as_deref()),
        Some(DiaryCommand::Path { name }) => path(name.as_deref()),
        Some(DiaryCommand::Del { name, yes }) => del(name.as_deref(), yes),
        None => match args.message {
            Some(message) => add(args.name.as_deref(), &message),
            None => edit(args.name.as_deref()),
        },
    }
}

/// Append a timestamped entry, creating the diary on first use.
///
/// The name is taken *exactly* (slugified), not fuzzy-matched: with creation on
/// demand, a fuzzy match would let a typo append to a similar diary, and would
/// make a new `work-notes` unreachable while `work` exists.
fn add(name: Option<&str>, message: &str) -> Result<()> {
    let (config, style) = util::config_and_style()?;

    let diary = match name {
        Some(name) => Diary::new(&config.memo_directory, name)?,
        None => state::load_diary().ok_or(StplError::NoLastDiary)?,
    };

    let existed = diary.exists();
    diary::add_entry(&diary, Local::now().naive_local(), message)?;
    state::record_diary(&diary.path);

    let what = if existed {
        "added entry to"
    } else {
        "created diary"
    };
    output::success(
        &style,
        &format!("{what} '{}' ({})", diary.name, diary.path.display()),
    );
    Ok(())
}

/// Print a diary's contents to stdout, undecorated.
fn show(name: Option<&str>) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let diary = util::resolve_diary_or_last(&config, &style, name)?;
    let content = diary::read_content(&diary)?;
    print!("{content}");
    if !content.ends_with('\n') {
        println!();
    }
    Ok(())
}

/// Open a diary in `$EDITOR`.
fn edit(name: Option<&str>) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let diary = util::resolve_diary_or_last(&config, &style, name)?;
    editor::open(&diary.path)
}

/// Print a diary's absolute path, bare, for scripting.
fn path(name: Option<&str>) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let diary = util::resolve_diary_or_last(&config, &style, name)?;
    println!("{}", diary.path.display());
    Ok(())
}

/// Delete a diary after confirmation.
///
/// Resolves via `resolve_diary_target` rather than `resolve_diary_or_last`:
/// recording a diary we are about to delete would discard the pointer to
/// whatever was remembered before.
fn del(name: Option<&str>, yes: bool) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let diary = util::resolve_diary_target(&config, &style, name)?;

    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err(anyhow!(
                "refusing to delete without confirmation; pass -y/--yes to delete non-interactively"
            ));
        }

        let entries = diary::read_content(&diary)
            .map(|c| diary::parse_entries(&c).len())
            .unwrap_or(0);
        anstream::print!(
            "Delete diary {} and its {entries} entries? [y/N] ",
            style.diary_line(&diary).trim_start_matches("- ")
        );
        std::io::stdout().flush().ok();

        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_ascii_lowercase();
        if answer != "y" && answer != "yes" {
            output::success(&style, "aborted; nothing deleted");
            return Ok(());
        }
    }

    diary::delete(&diary)?;
    // The diary is gone; don't leave the pointer aimed at it.
    state::clear_diary_if(&diary.path);
    output::success(&style, &format!("deleted diary {}", diary.path.display()));
    Ok(())
}

/// List every diary with its entry count and most recent entry.
fn list(format: Format) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let summaries: Vec<DiarySummary> = diary::list_all(&config)?
        .into_iter()
        .map(|diary| summarize(&diary))
        .collect();

    match format {
        Format::Text => render_text(&style, &summaries),
        Format::Json => anstream::println!("{}", serde_json::to_string_pretty(&summaries)?),
        Format::Markdown => anstream::println!("{}", render_markdown(&summaries)),
        Format::Editor => {
            let path = env::temp_dir().join("stpl-diaries.md");
            fs::write(&path, render_markdown(&summaries))?;
            editor::open(&path)?;
        }
    }
    Ok(())
}

/// Count a diary's entries and find its latest timestamp. An unreadable file
/// summarizes as empty rather than failing the whole listing.
fn summarize(diary: &Diary) -> DiarySummary {
    let entries = diary::read_content(diary)
        .map(|content| diary::parse_entries(&content))
        .unwrap_or_default();
    let last_entry = entries
        .iter()
        .map(|e| e.timestamp)
        .max()
        .map(|ts| ts.format(diary::TIMESTAMP_FORMAT).to_string());
    DiarySummary {
        diary: diary.clone(),
        entries: entries.len(),
        last_entry,
    }
}

fn render_text(style: &Style, summaries: &[DiarySummary]) {
    if summaries.is_empty() {
        output::success(style, "no diaries found");
        return;
    }
    for s in summaries {
        anstream::println!("{} {}", style.diary_line(&s.diary), dim(style, &stats(s)));
    }
}

/// `(3 entries, last 2026-09-11T14:32)`, or `(empty)` when there are none.
fn stats(s: &DiarySummary) -> String {
    match &s.last_entry {
        Some(last) => {
            let plural = if s.entries == 1 { "entry" } else { "entries" };
            format!("({} {plural}, last {last})", s.entries)
        }
        None => "(empty)".to_string(),
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

fn render_markdown(summaries: &[DiarySummary]) -> String {
    if summaries.is_empty() {
        return "# Diaries\n\n_No diaries found._\n".to_string();
    }
    let mut out = String::from("# Diaries\n\n");
    for s in summaries {
        // file:// URL with spaces percent-encoded so links stay valid.
        let url = s.diary.path.to_string_lossy().replace(' ', "%20");
        out.push_str(&format!(
            "- [{}](file://{url}) {}\n",
            s.diary.name,
            stats(s)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(entries: usize, last: Option<&str>) -> DiarySummary {
        DiarySummary {
            diary: Diary {
                name: "Work".to_string(),
                slug: "work".to_string(),
                path: std::path::PathBuf::from("/s/diaries/work.md"),
            },
            entries,
            last_entry: last.map(str::to_string),
        }
    }

    #[test]
    fn stats_reads_naturally_for_none_one_and_many() {
        assert_eq!(stats(&summary(0, None)), "(empty)");
        assert_eq!(
            stats(&summary(1, Some("2026-09-11T14:32"))),
            "(1 entry, last 2026-09-11T14:32)"
        );
        assert_eq!(
            stats(&summary(3, Some("2026-09-11T14:32"))),
            "(3 entries, last 2026-09-11T14:32)"
        );
    }

    #[test]
    fn markdown_lists_each_diary_as_a_link() {
        let out = render_markdown(&[summary(2, Some("2026-09-11T14:32"))]);
        assert_eq!(
            out,
            "# Diaries\n\n- [Work](file:///s/diaries/work.md) (2 entries, last 2026-09-11T14:32)\n"
        );
    }

    #[test]
    fn markdown_handles_an_empty_listing() {
        assert_eq!(render_markdown(&[]), "# Diaries\n\n_No diaries found._\n");
    }
}
